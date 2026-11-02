use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Default)]
struct TreeNode {
    children: BTreeMap<String, TreeNode>,
    is_file: bool,
}

pub fn paths_from_dir(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .min_depth(1)
        .sort_by_file_name()
    {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = entry.path().strip_prefix(root)?;
        if skip_tree_path(rel) {
            continue;
        }
        out.push(rel.to_path_buf());
    }
    out.sort();
    Ok(out)
}

fn skip_tree_path(rel: &Path) -> bool {
    rel == Path::new("STRUCTURE.md")
}

fn insert_path(root: &mut TreeNode, rel: &Path) {
    let mut node = root;
    let components: Vec<_> = rel.components().collect();
    for (i, comp) in components.iter().enumerate() {
        let name = comp.as_os_str().to_string_lossy().to_string();
        let is_last = i + 1 == components.len();
        let entry = node.children.entry(name).or_default();
        if is_last {
            entry.is_file = true;
        }
        node = entry;
    }
}

pub fn render_tree(root_label: &str, rel_paths: &[PathBuf]) -> String {
    let mut root = TreeNode::default();
    for p in rel_paths {
        insert_path(&mut root, p);
    }
    let mut lines = vec![format!("{root_label}")];
    render_children(&root, "", &mut lines);
    lines.join("\n")
}

fn render_children(node: &TreeNode, prefix: &str, lines: &mut Vec<String>) {
    let keys: Vec<_> = node.children.keys().collect();
    for (i, name) in keys.iter().enumerate() {
        let child = &node.children[*name];
        let last = i + 1 == keys.len();
        let branch = if last { "└── " } else { "├── " };
        let display = if child.is_file && child.children.is_empty() {
            name.to_string()
        } else if child.is_file {
            format!("{name}/")
        } else {
            format!("{name}/")
        };
        lines.push(format!("{prefix}{branch}{display}"));
        let extension = if last { "    " } else { "│   " };
        if !child.children.is_empty() {
            render_children(child, &format!("{prefix}{extension}"), lines);
        }
    }
}

pub fn find_repo_root() -> Result<PathBuf> {
    let mut dir = std::env::current_dir()?;
    loop {
        if dir.join("README.md").is_file() && dir.join(".agents").is_dir() {
            return Ok(dir);
        }
        if !dir.pop() {
            break;
        }
    }
    anyhow::bail!("Run from the dot-agents repo (needs README.md and .agents/)")
}

pub fn template_paths(repo: &Path) -> Result<Vec<PathBuf>> {
    paths_from_dir(&repo.join(".agents"))
}

pub fn generate_tree_markdown(repo: &Path) -> Result<String> {
    let paths = template_paths(repo)?;
    let body = render_tree("~/.agents/", &paths);
    Ok(format!(
        "```\n{body}\n```\n\nSame layout in this repo under [`.agents/`](.agents/). Repo-only file `.agents/STRUCTURE.md` is not installed to `~/.agents/`."
    ))
}

const START: &str = "<!-- dot-agents-tree:start -->";
const END: &str = "<!-- dot-agents-tree:end -->";

pub fn replace_readme_section(readme: &str, generated: &str) -> Result<String> {
    let start = readme
        .find(START)
        .with_context(|| format!("missing marker {START}"))?;
    let end = readme
        .find(END)
        .with_context(|| format!("missing marker {END}"))?;
    if end <= start {
        anyhow::bail!("invalid readme markers");
    }
    let mut out = String::new();
    out.push_str(&readme[..start + START.len()]);
    out.push('\n');
    out.push_str(generated);
    out.push('\n');
    out.push_str(&readme[end..]);
    Ok(out)
}

pub fn sync_readme(repo: &Path) -> Result<()> {
    let generated = generate_tree_markdown(repo)?;
    let readme_path = repo.join("README.md");
    let readme = fs::read_to_string(&readme_path)?;
    let updated = replace_readme_section(&readme, &generated)?;
    fs::write(&readme_path, updated)?;
    println!("Updated {}", readme_path.display());
    Ok(())
}

pub fn check_readme(repo: &Path) -> Result<()> {
    let generated = generate_tree_markdown(repo)?;
    let readme = fs::read_to_string(repo.join("README.md"))?;
    let expected = replace_readme_section(&readme, &generated)?;
    if expected != readme {
        anyhow::bail!("README tree is stale. Run: dot-agents readme sync");
    }
    println!("README tree OK");
    Ok(())
}
