# dot-agents

Filesystem template for agent rules: install `.agents/` into a **global home** or a **project** repo. The router names folders, not files, so what you add loads without edits and base improvements arrive on the next install.

Canonical URLs and version pins: [`.agents/context/upstream.json`](.agents/context/upstream.json).

## Install

```sh
git clone --branch v0.5.1 --depth 1 https://github.com/Madgeniusblink/dot-agents.git dot-agents
cd dot-agents
./install.sh
```

Re-run `./install.sh` to update. It replaces a file only if you have not touched it since the last install, and lists every file it kept ([install.md](.agents/standards/install.md)).

Customize by adding files: your own `rules/*.md`, `memory/*.md`, and skills. `AGENTS.md` is a router of links; there is nothing in it to edit.

## Why it is small

Your agent's context is mostly spent before your instructions are read. Tool definitions, MCP servers, and the system prompt come first. `AGENTS.md` is last in line and competes with all of it.

So the template's job is not to give you more instructions. It is to spend as few tokens as possible correcting the model, and to keep everything else out of the always-on path.

| Kind | Cost | Put here |
|------|------|----------|
| **Rule** (`rules/`) | Always loaded, so one line | A behavior the model gets wrong |
| **Skill** (`skills/`) | Description always, body when triggered | A procedure |
| **Fact** (`memory/`) | Read on demand | What the model cannot know |

Nothing goes in because it is true. A file earns its place by changing what the agent does when it is absent.

Rules are sorted by when they go stale. Boundaries (what needs a human) never expire. Corrections (what the model misses) are written as outcomes and each carries the task that proves it; on a model upgrade, run the task without the line and delete it if the output holds. So the set shrinks by construction as models improve.

CI enforces the budget: `AGENTS.md` at 50 lines or fewer, one line per rule, a test on every correction, `SKILL.md` at 120 lines or fewer ([check-budget.sh](scripts/check-budget.sh)).

**Windows:** `.\install.ps1` after clone. Override target with `DOT_AGENTS_HOME` or `$env:DOT_AGENTS_HOME`.

## Shipped tree (`.agents/`)

```
.agents/
├── AGENTS.md          router (folders, not files)
├── principles.md      what belongs where
├── rules/           boundaries (never expire), corrections (expire)
├── templates/         handoff, brief
├── skills/            patch, report
├── standards/
├── context/
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
