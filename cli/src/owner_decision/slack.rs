//! Slack Lists Web API (`slackLists.*`) client.

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::collections::HashMap;

pub const DEFAULT_API_BASE: &str = "https://slack.com/api/";
pub const OPEN_LABEL: &str = "Open";
pub const DONE_LABEL: &str = "Done";

pub struct SlackClient {
    token: String,
    base: String,
}

impl SlackClient {
    pub fn new(token: String, base: String) -> Self {
        Self {
            token,
            base: normalize_base(&base),
        }
    }

    fn post(&self, method: &str, body: Value) -> Result<Value> {
        let url = format!("{}{}", self.base, method);
        let resp = attohttpc::post(&url)
            .bearer_auth(&self.token)
            .header("Content-Type", "application/json; charset=utf-8")
            .json(&body)
            .map_err(|e| anyhow::anyhow!("{e}"))?
            .send()
            .with_context(|| format!("POST {method}"))?;
        let status = resp.status().as_u16();
        let doc: Value = resp.json().context("Slack response is not JSON")?;
        if !(200..300).contains(&status) {
            bail!("Slack HTTP {status}: {doc}");
        }
        if doc.get("ok").and_then(Value::as_bool) == Some(false) {
            let err = doc
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("unknown_error");
            bail!("Slack API error: {err}");
        }
        Ok(doc)
    }

    pub fn list_schema(&self, list_id: &str) -> Result<ListSchema> {
        let doc = self.post(
            "slackLists.items.list",
            json!({ "list_id": list_id, "limit": 1 }),
        )?;
        ListSchema::from_response(&doc)
    }

    pub fn list_items(&self, list_id: &str) -> Result<Vec<ListItem>> {
        let mut items = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let mut body = json!({ "list_id": list_id, "limit": 200 });
            if let Some(c) = &cursor {
                body["cursor"] = json!(c);
            }
            let doc = self.post("slackLists.items.list", body)?;
            items.extend(ListItem::from_list_response(&doc)?);
            cursor = doc
                .get("response_metadata")
                .and_then(|m| m.get("next_cursor"))
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_string);
            if cursor.is_none() {
                break;
            }
        }
        Ok(items)
    }

    pub fn count_open_rows(&self, list_id: &str, schema: &ListSchema) -> Result<usize> {
        let items = self.list_items(list_id)?;
        Ok(items.iter().filter(|i| !i.is_done(schema)).count())
    }

    pub fn create_item(&self, list_id: &str, initial_fields: Vec<Value>) -> Result<String> {
        let doc = self.post(
            "slackLists.items.create",
            json!({
                "list_id": list_id,
                "initial_fields": initial_fields,
            }),
        )?;
        item_id_from_value(&doc["item"])
            .or_else(|| item_id_from_value(&doc))
            .context("create response missing item id")
    }

    pub fn item_info(&self, list_id: &str, item_id: &str) -> Result<ListItem> {
        let doc = self.post(
            "slackLists.items.info",
            json!({ "list_id": list_id, "id": item_id }),
        )?;
        ListItem::from_info_response(&doc).context("items.info missing item")
    }

    pub fn update_item(&self, list_id: &str, item_id: &str, cells: Vec<Value>) -> Result<()> {
        self.post(
            "slackLists.items.update",
            json!({
                "list_id": list_id,
                "cells": cells_with_row(cells, item_id),
            }),
        )?;
        Ok(())
    }
}

fn normalize_base(base: &str) -> String {
    let b = base.trim();
    if b.ends_with('/') {
        b.to_string()
    } else {
        format!("{b}/")
    }
}

fn item_id_from_value(v: &Value) -> Option<String> {
    v.get("id")
        .or_else(|| v.get("item_id"))
        .or_else(|| v.get("record_id"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn cells_with_row(cells: Vec<Value>, row_id: &str) -> Vec<Value> {
    cells
        .into_iter()
        .map(|mut c| {
            if c.get("row_id").is_none() {
                c["row_id"] = json!(row_id);
            }
            c
        })
        .collect()
}

#[derive(Clone, Debug)]
pub struct ListSchema {
    columns: Vec<Column>,
}

#[derive(Clone, Debug)]
struct Column {
    id: String,
    name: String,
    key: String,
    kind: String,
}

impl ListSchema {
    fn from_response(doc: &Value) -> Result<Self> {
        let cols = extract_columns(doc)?;
        if cols.is_empty() {
            bail!("Slack list response missing column schema");
        }
        Ok(Self { columns: cols })
    }

    fn column(&self, display_name: &str) -> Option<&Column> {
        let want = normalize_key(display_name);
        self.columns
            .iter()
            .find(|c| c.key == want || c.name.eq_ignore_ascii_case(display_name))
    }

    pub fn initial_fields(&self, pairs: &[(&str, Value)]) -> Result<Vec<Value>> {
        let mut fields = Vec::new();
        for (name, value) in pairs {
            let col = self
                .column(name)
                .with_context(|| format!("list has no column named {name}"))?;
            fields.push(field_cell(col, value.clone())?);
        }
        Ok(fields)
    }

    pub fn update_cells(&self, pairs: &[(&str, Value)]) -> Result<Vec<Value>> {
        self.initial_fields(pairs)
    }
}

fn extract_columns(doc: &Value) -> Result<Vec<Column>> {
    let mut cols = Vec::new();
    for path in [
        doc.get("schema").and_then(|s| s.get("columns")),
        doc.get("list")
            .and_then(|l| l.get("schema"))
            .and_then(|s| s.get("columns")),
        doc.get("columns"),
    ] {
        if let Some(arr) = path.and_then(Value::as_array) {
            for c in arr {
                if let Some(col) = parse_column(c) {
                    cols.push(col);
                }
            }
        }
    }
    Ok(cols)
}

fn parse_column(v: &Value) -> Option<Column> {
    let id = v
        .get("id")
        .or_else(|| v.get("column_id"))
        .and_then(Value::as_str)?
        .to_string();
    let name = v
        .get("name")
        .or_else(|| v.get("title"))
        .or_else(|| v.get("label"))
        .and_then(Value::as_str)
        .unwrap_or(&id)
        .to_string();
    let kind = v
        .get("type")
        .or_else(|| v.get("kind"))
        .and_then(Value::as_str)
        .unwrap_or("text")
        .to_string();
    Some(Column {
        key: normalize_key(&name),
        id,
        name,
        kind,
    })
}

fn field_cell(col: &Column, value: Value) -> Result<Value> {
    let mut cell = json!({ "column_id": col.id });
    if col.kind == "select" {
        let label = value.as_str().context("select value must be a string")?;
        cell["select"] = json!([{ "label": label }]);
    } else if col.kind == "date" {
        let d = value.as_str().context("date value must be a string")?;
        cell["date"] = json!(d);
    } else {
        let t = match value {
            Value::String(s) => s,
            other => other.to_string(),
        };
        cell["text"] = json!(t);
    }
    Ok(cell)
}

fn normalize_key(name: &str) -> String {
    name.to_ascii_lowercase().replace(' ', "_")
}

#[derive(Clone, Debug)]
pub struct ListItem {
    pub id: String,
    raw: Value,
}

impl ListItem {
    fn from_list_response(doc: &Value) -> Result<Vec<Self>> {
        let arr = doc
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        Ok(arr
            .into_iter()
            .filter_map(|v| Self::parse_item(&v))
            .collect())
    }

    fn from_info_response(doc: &Value) -> Option<Self> {
        doc.get("record")
            .and_then(Self::parse_item)
            .or_else(|| doc.get("item").and_then(Self::parse_item))
            .or_else(|| Self::parse_item(doc))
    }

    fn parse_item(v: &Value) -> Option<Self> {
        let id = item_id_from_value(v)?;
        Some(Self { id, raw: v.clone() })
    }

    pub fn field_map(&self, schema: &ListSchema) -> HashMap<String, String> {
        let mut map = HashMap::new();
        let cells = self
            .raw
            .get("fields")
            .or_else(|| self.raw.get("cells"))
            .and_then(Value::as_array);
        let Some(cells) = cells else {
            return map;
        };
        for cell in cells {
            let col_id = cell.get("column_id").and_then(Value::as_str).unwrap_or("");
            let col = schema.columns.iter().find(|c| c.id == col_id);
            let key = col
                .map(|c| c.key.clone())
                .unwrap_or_else(|| col_id.to_string());
            if let Some(text) = cell_text(cell) {
                map.insert(key, text);
            }
        }
        map
    }

    pub fn is_done(&self, schema: &ListSchema) -> bool {
        let fields = self.field_map(schema);
        let status = fields.get("status").map(String::as_str).unwrap_or("");
        status.eq_ignore_ascii_case(DONE_LABEL)
    }
}

fn cell_text(cell: &Value) -> Option<String> {
    if let Some(s) = cell.get("text").and_then(Value::as_str) {
        return Some(s.to_string());
    }
    if let Some(s) = cell.get("value").and_then(Value::as_str) {
        return Some(s.to_string());
    }
    if let Some(sel) = cell.get("select").and_then(Value::as_array) {
        let labels: Vec<_> = sel
            .iter()
            .filter_map(|v| {
                v.get("label")
                    .or_else(|| v.get("value"))
                    .and_then(Value::as_str)
            })
            .collect();
        if !labels.is_empty() {
            return Some(labels.join(", "));
        }
    }
    if let Some(s) = cell.get("date").and_then(Value::as_str) {
        return Some(s.to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_schema_from_fixture() {
        let doc: Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/owner-decision/list_schema.json"
        ))
        .unwrap();
        let schema = ListSchema::from_response(&doc).unwrap();
        assert!(schema.column("Session").is_some());
    }

    #[test]
    fn field_map_reads_cells() {
        let doc: Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/owner-decision/list_items.json"
        ))
        .unwrap();
        let schema = ListSchema::from_response(&doc).unwrap();
        let items = ListItem::from_list_response(&doc).unwrap();
        let fields = items[0].field_map(&schema);
        assert_eq!(
            fields.get("session").map(String::as_str),
            Some("sess-example")
        );
        assert_eq!(fields.get("status").map(String::as_str), Some("Open"));
    }
}
