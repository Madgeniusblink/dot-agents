# dot-agents

Filesystem template for agent rules. Your agent home is a git checkout, not an install target, so a file you edit is live at once. The router names folders, not files, so what you add loads with no edit to it.

Canonical URLs and version pins: [`context/upstream.json`](context/upstream.json).

## Start

Press **Use this template**, make your copy private, then clone it to your agent home:

```sh
git clone <your-copy> ~/.agents
```

Take later improvements with a merge:

```sh
cd ~/.agents && git remote add upstream https://github.com/Madgeniusblink/dot-agents
git fetch upstream && git merge --no-edit upstream/main
```

Or let the `agents` binary clone, add the upstream, and point every tool on the machine at the home in one step:

```sh
brew install madgeniusblink/tap/agents
agents init <your-copy> --apply
```

The binary embeds no content and reads the same tree. Every command prints its plan and writes nothing without `--apply`. Without Homebrew, `cargo install --git` builds it, and the scripts in `scripts/` do the same with no binary ([docs/setup.md](docs/setup.md)).

A conflict is the point: it shows the line that moved under a rule you rewrote. There is no installer, no copy, and no manifest, because `git status` and `git diff` already answer every question one would need to reinvent ([adopt.md](standards/adopt.md)).

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

Most tools do not read `~/.agents` on their own. [docs/setup.md](docs/setup.md) shows how to point yours at it, once.

## The tree

The root of this repository is the tree. Clone it to `~/.agents` and the paths below are live.

```
~/.agents/
├── AGENTS.md          router (folders, not files)
├── principles.md      what belongs where
├── rules/             boundaries (never expire), corrections (expire)
├── templates/         handoff, brief
├── skills/            agent-home, patch, report
├── standards/
├── context/
├── scripts/           wire.sh, context.sh, check.sh
├── cli/               the `agents` binary: status, tree, scan, skills, memory, wire, check, sync, reset
├── memory/            you own
└── decisions/         you own
```

Detail: [`AGENTS.md`](AGENTS.md). Changes: [CHANGELOG.md](CHANGELOG.md).

## Project copy

A repository gets its own `.agents/` directory, committed with that repository. Copy in the files that repository needs. It is not a checkout of this one.

## Maintainer docs

[CONTRIBUTING.md](CONTRIBUTING.md) · [SECURITY.md](SECURITY.md)

## License

MIT — see [LICENSE](LICENSE).
