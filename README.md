# dot-agents

Filesystem template for agent rules: copy `.agents/` into a **global home** or a **project** repo. This repository is **template only** (no Rust CLI). Vendor-specific wiring lives in [`.agents/standards/adapters.md`](.agents/standards/adapters.md).

## Install

**macOS / Linux**

```sh
git clone https://github.com/Madgeniusblink/dot-agents.git
cd dot-agents
./install.sh
```

Re-run `./install.sh` to add **missing** template files only. It does **not** overwrite your `memory/` or `decisions/`. Use `./install.sh --force` to overwrite.

**Windows**

```powershell
git clone https://github.com/Madgeniusblink/dot-agents.git
cd dot-agents
.\install.ps1
```

Override target: `DOT_AGENTS_HOME` (Unix) or `$env:DOT_AGENTS_HOME` (Windows). Default: `~/.agents` or `%USERPROFILE%\.agents`.

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

Detail: [`.agents/AGENTS.md`](.agents/AGENTS.md) and [`.agents/standards/README.md`](.agents/standards/README.md).

## Project copy

```sh
cp -R dot-agents/.agents /path/to/your-repo/
```

Add a short root `AGENTS.md` in the project if you use one.

## Upstream

Canonical repo: [github.com/Madgeniusblink/dot-agents](https://github.com/Madgeniusblink/dot-agents). Change the shared template via PR here; keep personal content in `memory/` and `decisions/`.

## License

MIT — see [LICENSE](LICENSE).
