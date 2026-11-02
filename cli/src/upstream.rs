use anyhow::Result;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Upstream {
    #[allow(dead_code)]
    pub version: u32,
    pub name: String,
    pub repository: String,
    pub default_branch: String,
    pub extends_doc: String,
    pub links: UpstreamLinks,
}

#[derive(Deserialize)]
pub struct UpstreamLinks {
    pub repo: String,
    pub tree_main: String,
    pub raw_main: String,
    pub cli_install: String,
}

pub fn parse_upstream(json: &str) -> Result<Upstream> {
    Ok(serde_json::from_str(json)?)
}

pub fn format_human(u: &Upstream) -> String {
    format!(
        r#"dot-agents upstream — canonical template source

Name:       {}
Repository: {}
Branch:     {}
Extend doc: {}/{} (in installed tree: .agents/{})

Links (use in skills and rules):
  Repo:     {}
  Tree:     {}
  Raw:      {}
  CLI:      {}

Install or refresh: dot-agents install
"#,
        u.name,
        u.repository,
        u.default_branch,
        u.links.tree_main,
        u.extends_doc,
        u.extends_doc,
        u.links.repo,
        u.links.tree_main,
        u.links.raw_main,
        u.links.cli_install,
    )
}
