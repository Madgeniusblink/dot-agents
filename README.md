# dot-agents

Filesystem template for agent rules: install `.agents/` into a **global home** or a **project** repo. Template only (no Rust CLI in this repo). Vendor wiring: [`.agents/standards/adapters.md`](.agents/standards/adapters.md).

Canonical URLs and version pins: [`.agents/context/upstream.json`](.agents/context/upstream.json).

## Install

**macOS / Linux**

Clone this repository from GitHub (`links.repo` in [upstream.json](.agents/context/upstream.json)).

```sh
cd dot-agents
./install.sh
```

Re-run `./install.sh` to refresh **upstream-owned** files and add missing paths. It **never** overwrites existing files under `memory/` or `decisions/`. The `--force` flag does not change that.

**Windows**

```powershell
cd dot-agents
.\install.ps1
```

Override target: `DOT_AGENTS_HOME` (Unix) or `$env:DOT_AGENTS_HOME` (Windows). Default: `~/.agents` or `$HOME\.agents` (PowerShell uses `$HOME` when set).

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

Detail: [`.agents/AGENTS.md`](.agents/AGENTS.md).

## Project copy

```sh
DOT_AGENTS_HOME=/path/to/your-repo/.agents ./install.sh
```

Add a short root `AGENTS.md` in the project if your tool expects one at repo root.

## Maintainer docs

- [CONTRIBUTING.md](CONTRIBUTING.md) · [SECURITY.md](SECURITY.md)
- Optional Rust CLI: `cli_install` URL in [upstream.json](.agents/context/upstream.json)

## License

MIT — see [LICENSE](LICENSE).
