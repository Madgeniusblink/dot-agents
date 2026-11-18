//! Owner Decision Queue: Slack List rows for human-only decisions (`ask`, `read`, `close`).

mod config;
mod slack;

use anyhow::{bail, Context, Result};
use config::OwnerDecisionSettings;
use serde_json::{json, Value};
use slack::{ListItem, SlackClient, DONE_LABEL, OPEN_LABEL};
use std::io::{self, Write};

/// Base URL override for tests (`AGENTS_SLACK_API_BASE`); production uses the default Slack API host.
pub fn slack_api_base() -> String {
    std::env::var("AGENTS_SLACK_API_BASE").unwrap_or_else(|_| slack::DEFAULT_API_BASE.to_string())
}

pub fn ask(
    decision: &str,
    recommended: &str,
    checked: &str,
    why_you: &str,
    project: &str,
    session: &str,
    json: bool,
) -> Result<()> {
    validate_checked_link(checked)?;
    validate_nonempty("why you", why_you)?;
    let settings = OwnerDecisionSettings::load()?;
    let client = SlackClient::new(settings.token()?, slack_api_base());
    let list_id = settings.list_id()?;
    let schema = client.list_schema(&list_id)?;
    let open = client.count_open_rows(&list_id, &schema)?;
    if open >= 10 {
        bail!("owner-decision ask: list already has {open} open rows (limit 10)");
    }
    let asked = today_iso();
    let fields = schema.initial_fields(&[
        ("Decision", json!(decision)),
        ("Recommended", json!(recommended)),
        ("Checked", json!(checked)),
        ("Why you", json!(why_you)),
        ("Project", json!(project)),
        ("Status", json!(OPEN_LABEL)),
        ("Asked", json!(asked)),
        ("Session", json!(session)),
    ])?;
    let record_id = client.create_item(&list_id, fields)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "record_id": record_id,
                "session": session,
                "status": OPEN_LABEL,
                "decision": decision,
            }))?
        );
    } else {
        println!("record_id: {record_id}");
        println!("session: {session}");
        println!("status: {OPEN_LABEL}");
        println!("decision: {decision}");
    }
    Ok(())
}

pub fn read(session: &str, record_id: Option<&str>, json: bool) -> Result<()> {
    let settings = OwnerDecisionSettings::load()?;
    let client = SlackClient::new(settings.token()?, slack_api_base());
    let list_id = settings.list_id()?;
    let schema = client.list_schema(&list_id)?;
    let item = find_item(&client, &list_id, &schema, session, record_id)?;
    let fields = item.field_map(&schema);
    let answer = fields.get("answer").cloned().unwrap_or_default();
    let pending = answer.trim().is_empty();
    let doc = json!({
        "record_id": item.id,
        "session": fields.get("session").cloned().unwrap_or_default(),
        "status": fields.get("status").cloned().unwrap_or_default(),
        "answer": if pending { Value::Null } else { json!(answer) },
        "pending": pending,
        "decision": fields.get("decision"),
        "recommended": fields.get("recommended"),
        "checked": fields.get("checked"),
        "why_you": fields.get("why_you"),
        "project": fields.get("project"),
        "asked": fields.get("asked"),
    });
    if json {
        println!("{}", serde_json::to_string_pretty(&doc)?);
    } else if pending {
        writeln!(io::stdout(), "answer: (pending)")?;
        print_fields(&fields);
    } else {
        writeln!(io::stdout(), "answer: {answer}")?;
        print_fields(&fields);
    }
    Ok(())
}

pub fn close(session: &str, record_id: Option<&str>, proof_link: &str, json: bool) -> Result<()> {
    validate_checked_link(proof_link)?;
    let settings = OwnerDecisionSettings::load()?;
    let client = SlackClient::new(settings.token()?, slack_api_base());
    let list_id = settings.list_id()?;
    let schema = client.list_schema(&list_id)?;
    let item = find_item(&client, &list_id, &schema, session, record_id)?;
    let fields = item.field_map(&schema);
    let answer = fields
        .get("answer")
        .map(String::as_str)
        .unwrap_or("")
        .trim();
    if answer.is_empty() {
        bail!("owner-decision close: Answer is still empty; set it in the list before closing");
    }
    let prior_checked = fields.get("checked").cloned().unwrap_or_default();
    let merged_checked = merge_checked(&prior_checked, proof_link);
    let cells = schema.update_cells(&[
        ("Status", json!(DONE_LABEL)),
        ("Checked", json!(merged_checked)),
    ])?;
    client.update_item(&list_id, &item.id, cells)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "record_id": item.id,
                "status": DONE_LABEL,
                "checked": merged_checked,
                "proof_link": proof_link,
            }))?
        );
    } else {
        println!("record_id: {}", item.id);
        println!("status: {DONE_LABEL}");
        println!("checked: {merged_checked}");
    }
    Ok(())
}

fn find_item(
    client: &SlackClient,
    list_id: &str,
    schema: &slack::ListSchema,
    session: &str,
    record_id: Option<&str>,
) -> Result<ListItem> {
    if let Some(id) = record_id {
        return client
            .item_info(list_id, id)
            .with_context(|| format!("record_id {id}"));
    }
    let items = client.list_items(list_id)?;
    items
        .into_iter()
        .find(|i| i.field_map(schema).get("session").map(String::as_str) == Some(session))
        .with_context(|| format!("no row with Session {session}"))
}

fn print_fields(fields: &std::collections::HashMap<String, String>) {
    for key in [
        "decision",
        "recommended",
        "checked",
        "why_you",
        "project",
        "status",
        "asked",
    ] {
        if let Some(v) = fields.get(key) {
            if !v.is_empty() {
                let label = key.replace('_', " ");
                println!("{label}: {v}");
            }
        }
    }
}

fn validate_nonempty(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("owner-decision ask: missing {label}");
    }
    Ok(())
}

/// Evidence must include an http(s) URL or markdown link target.
pub fn validate_checked_link(value: &str) -> Result<()> {
    let t = value.trim();
    if t.is_empty() {
        bail!("owner-decision: Checked must include a link");
    }
    if t.contains("http://") || t.contains("https://") {
        return Ok(());
    }
    if let Some(start) = t.find("](") {
        let rest = &t[start + 2..];
        if rest.contains(')') {
            let url = rest.split(')').next().unwrap_or("");
            if url.starts_with("http://") || url.starts_with("https://") {
                return Ok(());
            }
        }
    }
    bail!("owner-decision: Checked must include an http(s) link");
}

fn merge_checked(prior: &str, proof: &str) -> String {
    let prior = prior.trim();
    if prior.is_empty() {
        return proof.to_string();
    }
    if prior.contains(proof) {
        return prior.to_string();
    }
    format!("{prior}\nproof: {proof}")
}

fn today_iso() -> String {
    std::process::Command::new("date")
        .args(["-u", "+%F"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "1970-01-01".to_string())
}

/// Session correlation for this agent run.
pub fn session_id() -> String {
    std::env::var("OWNER_DECISION_SESSION")
        .or_else(|_| std::env::var("AGENTS_OWNER_DECISION_SESSION"))
        .unwrap_or_else(|_| format!("local-{}", config::machine_hash()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_requires_http_link() {
        assert!(validate_checked_link("see https://example.com/evidence").is_ok());
        assert!(validate_checked_link("[evidence](https://example.com/x)").is_ok());
        assert!(validate_checked_link("no link here").is_err());
    }

    #[test]
    fn merge_checked_appends_proof() {
        let m = merge_checked("https://example.com/a", "https://example.com/proof");
        assert!(m.contains("proof: https://example.com/proof"));
    }
}
