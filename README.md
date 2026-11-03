# dot-agents

Filesystem template for agent rules: install `.agents/` into a **global home** or a **project** repo. The router names folders, not files, so what you add loads without edits and base improvements arrive on the next install.

Canonical URLs and version pins: [`.agents/context/upstream.json`](.agents/context/upstream.json).

## Install

```sh
git clone --branch v0.2.0 --depth 1 https://github.com/Madgeniusblink/dot-agents.git dot-agents
cd dot-agents
./install.sh
```

Re-run `./install.sh` to add **missing** files and refresh shipped paths in [`.agents/.managed-refresh`](.agents/.managed-refresh), including `AGENTS.md`. It does **not** overwrite paths in [`.agents/.user-owned`](.agents/.user-owned) or files the template does not ship.

Customize by adding files: your own `rules/*.md`, `memory/*.md`, skills, and an optional `AGENTS.local.md` for router additions. Do not edit shipped files; install replaces them.

**Windows:** `.\install.ps1` after clone. Override target with `DOT_AGENTS_HOME` or `$env:DOT_AGENTS_HOME`.

## Shipped tree (`.agents/`)

```
.agents/
├── AGENTS.md          router (folders, not files)
├── principles.md      what belongs where
├── rules/behavior.md  always on
├── templates/         handoff, brief
├── skills/            self-verify, patch
├── standards/
├── context/
├── checklists/
├── memory/            you own
└── decisions/         you own
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
