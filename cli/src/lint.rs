use anyhow::{bail, Context, Result};
use std::fs;
use std::path::Path;

use crate::readme;
use crate::upstream;

const UPSTREAM_JSON: &str = include_str!("../../.agents/context/upstream.json");
const CARGO_REPO: &str = env!("CARGO_PKG_REPOSITORY");

pub fn lint_repo(repo: &Path) -> Result<()> {
    println!("lint  readme tree");
    readme::check_readme(repo).context("README tree stale — run: dot-agents readme sync")?;

    println!("lint  upstream.json vs Cargo.toml repository");
    lint_upstream_repository()?;

    let agents = repo.join(".agents");
    println!("lint  var/.gitignore contract");
    lint_var_gitignore(&agents.join("var/scratch"))?;
    lint_var_gitignore(&agents.join("var/worktrees"))?;

    println!("lint  root .gitignore var patterns");
    lint_root_gitignore(repo)?;

    println!("lint  required standards files");
    for rel in [
        "standards/lint.md",
        "standards/extending.md",
        "context/upstream.json",
        "context/data-locations.md",
    ] {
        let p = agents.join(rel);
        if !p.is_file() {
            bail!("missing .agents/{rel}");
        }
    }

    println!("lint OK");
    Ok(())
}

fn lint_upstream_repository() -> Result<()> {
    let u = upstream::parse_upstream(UPSTREAM_JSON)?;
    if u.repository != CARGO_REPO {
        bail!(
            "upstream.json repository {} != Cargo.toml {}",
            u.repository,
            CARGO_REPO
        );
    }
    Ok(())
}

fn lint_var_gitignore(dir: &Path) -> Result<()> {
    let gi = dir.join(".gitignore");
    let body = fs::read_to_string(&gi)
        .with_context(|| format!("read {}", gi.display()))?;
    for needle in ["*", "!.gitignore", "!README.md"] {
        if !body.contains(needle) {
            bail!("{} must contain {:?}", gi.display(), needle);
        }
    }
    Ok(())
}

fn lint_root_gitignore(repo: &Path) -> Result<()> {
    let gi = repo.join(".gitignore");
    let body = fs::read_to_string(&gi).context("read .gitignore")?;
    for needle in [
        ".agents/var/scratch/**",
        ".agents/var/worktrees/**",
    ] {
        if !body.contains(needle) {
            bail!(".gitignore must ignore {needle}");
        }
    }
    Ok(())
}
