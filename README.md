# dot-agents

Filesystem template for agent rules: install `.agents/` into a **global home** or a **project** repo. Vendor wiring: [`.agents/standards/adapters.md`](.agents/standards/adapters.md).

Canonical URLs and version pins: [`.agents/context/upstream.json`](.agents/context/upstream.json).

## Install

Pinned clone (replace `<url>` with `links.repo` in [upstream.json](.agents/context/upstream.json)):

```sh
git clone --branch v0.1.0 --depth 1 <url> dot-agents
cd dot-agents
./install.sh
```

Re-run `./install.sh` to add **missing** files and refresh paths in [`.agents/.managed-refresh`](.agents/.managed-refresh). It does **not** overwrite existing paths in [`.agents/.user-owned`](.agents/.user-owned) or other files you already edited (for example `AGENTS.md`).

**Windows:** `.\install.ps1` after clone. Override target with `DOT_AGENTS_HOME` or `$env:DOT_AGENTS_HOME`.

## Shipped tree (`.agents/`)

```
.agents/
├── AGENTS.md
├── standards/
├── context/
├── checklists/
├── memory/
├── decisions/
├── skills/self-verify/
├── rules/
└── templates/
```

Detail: [`.agents/AGENTS.md`](.agents/AGENTS.md). Changes: [CHANGELOG.md](CHANGELOG.md).

## Project copy

```sh
DOT_AGENTS_HOME=/path/to/your-repo/.agents ./install.sh
```

## Maintainer docs

[CONTRIBUTING.md](CONTRIBUTING.md) · [SECURITY.md](SECURITY.md) · optional CLI in [upstream.json](.agents/context/upstream.json) → `links.cli_install`

## License

MIT — see [LICENSE](LICENSE).
