//! Machine-local Owner Decision Queue settings (list id + token env names).

use crate::config::{machine, path};
use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::fs;

const DEFAULT_LIST_ID_ENV: &str = "OWNER_DECISION_LIST_ID";
const DEFAULT_TOKEN_ENV: &str = "OWNER_DECISION_SLACK_TOKEN";

#[allow(dead_code)] // Referenced from `cli/examples/owner-decision.op.env.example` and future docs.
pub const OWNER_DECISION_DOC: &str = r#"## Owner Decision Queue configuration

Add a `[owner_decision]` table to your agents config (`$AGENTS_CONFIG` or `~/.config/agents/config.toml`):

```toml
[owner_decision]
# Slack List encoded ID (F… form). Optional when OWNER_DECISION_LIST_ID is set.
list_id = ""

# Env var names that hold the list id and bot token (values are never stored in git).
list_id_env = "OWNER_DECISION_LIST_ID"
token_env = "OWNER_DECISION_SLACK_TOKEN"
```

Token needs Slack bot scopes `lists:read` and `lists:write` (see https://api.slack.com/methods/slackLists.items.create).

Inject the token at runtime with 1Password CLI, for example:

```sh
op run --env-file=~/.config/agents/owner-decision.op.env -- agents owner-decision ask ...
```

See `cli/examples/owner-decision.op.env.example` for reference-only `op://` placeholders.
"#;

#[derive(Clone, Debug)]
pub struct OwnerDecisionSettings {
    pub list_id: Option<String>,
    pub list_id_env: String,
    pub token_env: String,
}

impl OwnerDecisionSettings {
    pub fn load() -> Result<Self> {
        let path = path();
        let Ok(text) = fs::read_to_string(&path) else {
            return Ok(Self::default());
        };
        let doc: Value = basic_toml::from_str(&text)
            .with_context(|| format!("{} is not valid TOML", path.display()))?;
        let table = &doc["owner_decision"];
        let str_field = |key: &str, default: &str| -> Result<String> {
            match &table[key] {
                Value::Null => Ok(default.to_string()),
                Value::String(s) => Ok(s.clone()),
                _ => bail!("{}: owner_decision.{key} must be a string", path.display()),
            }
        };
        let list_id = match &table["list_id"] {
            Value::Null => None,
            Value::String(s) if s.is_empty() => None,
            Value::String(s) => Some(s.clone()),
            _ => bail!(
                "{}: owner_decision.list_id must be a string",
                path.display()
            ),
        };
        Ok(Self {
            list_id,
            list_id_env: str_field("list_id_env", DEFAULT_LIST_ID_ENV)?,
            token_env: str_field("token_env", DEFAULT_TOKEN_ENV)?,
        })
    }

    pub fn list_id(&self) -> Result<String> {
        if let Some(id) = &self.list_id {
            if !id.is_empty() {
                return Ok(id.clone());
            }
        }
        std::env::var(&self.list_id_env)
            .with_context(|| format!("set {} or [owner_decision] list_id", self.list_id_env))
    }

    pub fn token(&self) -> Result<String> {
        std::env::var(&self.token_env).with_context(|| {
            format!(
                "set {} to a Slack token with lists:read and lists:write",
                self.token_env
            )
        })
    }
}

impl Default for OwnerDecisionSettings {
    fn default() -> Self {
        Self {
            list_id: None,
            list_id_env: DEFAULT_LIST_ID_ENV.to_string(),
            token_env: DEFAULT_TOKEN_ENV.to_string(),
        }
    }
}

/// Short stable suffix when no session env is set.
pub fn machine_hash() -> String {
    let m = machine();
    let h = m
        .bytes()
        .fold(0u32, |a, b| a.wrapping_mul(31).wrapping_add(b as u32));
    format!("{h:08x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_env_names() {
        let s = OwnerDecisionSettings::default();
        assert_eq!(s.list_id_env, "OWNER_DECISION_LIST_ID");
        assert_eq!(s.token_env, "OWNER_DECISION_SLACK_TOKEN");
    }
}
