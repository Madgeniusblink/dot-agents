# dot-agents

Filesystem template for agent rules. Your agent home is a git checkout, not an install target, so a file you edit is live at once. The router names folders, not files, so what you add loads with no edit to it.

Canonical URLs and version pins: [`home/context/upstream.json`](home/context/upstream.json).

The home is the [`home/`](home/) directory of this repository. The root holds what the repository needs and an agent never reads: the `agents` binary's source, the changelog, the maintainer docs, CI, and the release files. A home carries `home/` only ([decision 0012](docs/decisions/0012-home-directory-is-the-home.md)).

## Start

Make an empty private repository for your home, then let the `agents` binary clone it, give it the template's `home/` as its root, add this repository as `upstream`, and point every tool on the machine at it:

```sh
cargo install --git https://github.com/Madgeniusblink/dot-agents dot-agents
agents init <your-repo> --apply
```

Without the binary, the same in git:

```sh
git clone --origin upstream https://github.com/Madgeniusblink/dot-agents ~/.agents
cd ~/.agents
git reset --hard "$(git commit-tree -p HEAD -m 'feat: make the root the home' HEAD:home)"
git remote add origin <your-repo> && git push -u origin main
```

Take later improvements with a merge. The `-X subtree=home` flag shifts the template down to `home/` first, so nothing outside it enters the home:

```sh
cd ~/.agents
git fetch upstream && git merge --no-edit -X subtree=home upstream/main
```

`agents sync --apply` runs that merge, adds `--allow-unrelated-histories` for a copy made with **Use this template**, and rewires. The binary embeds no content and reads the same tree. Every command prints its plan and writes nothing without `--apply`. `agents --version` names the commit it was built from, and once a day a command says on stderr how many commits main is ahead of that build; `AGENTS_UPDATE_CHECK=off` silences it. The first tagged release adds `brew install`; the scripts in `home/scripts/` do the same with no binary ([docs/setup.md](home/docs/setup.md)).

A conflict is the point: it shows the line that moved under a rule you rewrote. There is no installer, no copy, and no manifest, because `git status` and `git diff` already answer every question one would need to reinvent ([adopt.md](home/standards/adopt.md)).

Customize by adding files: your own `rules/*.md`, `memory/*.md`, and skills. `AGENTS.md` is a router of links; there is nothing in it to edit.

### A home that started before 0.18.0

Before 0.18.0 the root of this repository was the home, so a home from then carries the glue. One merge with `-X subtree=home` removes it and keeps every file you added:

```sh
cd ~/.agents
git fetch upstream && git merge --no-edit -X subtree=home upstream/main
```

A glue file you edited, such as `CONTRIBUTING.md` or `docs/repo-rules.md`, is a modify/delete conflict once. Keep it with `git checkout --ours -- <path> && git add <path>`, or drop it with `git rm -- <path>`, then `git commit --no-edit`. Your `merge=ours` lines for `README.md` and `.gitignore` still apply, since `home/` ships both.

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

CI enforces the budget: `AGENTS.md` at 50 lines or fewer, one line per rule, a test on every correction, `SKILL.md` at 120 lines or fewer ([check-budget.sh](home/scripts/check-budget.sh)).

Most tools do not read `~/.agents` on their own. [docs/setup.md](home/docs/setup.md) shows how to point yours at it, once.

## The tree

`home/` is the home. A checkout of it at `~/.agents` has the paths below live.

```
~/.agents/                 home/ in this repository
├── AGENTS.md              router (folders, not files)
├── principles.md          what belongs where
├── rules/                 boundaries (never expire), corrections (expire)
├── templates/             handoff, brief
├── skills/                agent-home, patch, report
├── standards/
├── context/
├── docs/                  setup, check, json
├── scripts/               wire.sh, context.sh, check.sh
├── memory/                you own
└── decisions/             you own
```

The root of this repository, which a home does not carry:

```
cli/                 the `agents` binary: status, tree, scan, skills, memory, wire, check, sync, reset
docs/decisions/      template decisions
.github/, scripts/   CI, merge tiers, neutrality, release
CHANGELOG.md, CONTRIBUTING.md, SECURITY.md, LICENSE, dist-workspace.toml
```

Detail: [`home/AGENTS.md`](home/AGENTS.md). Changes: [CHANGELOG.md](CHANGELOG.md).

## Project copy

A repository gets its own `.agents/` directory, committed with that repository. Copy in the files that repository needs. It is not a checkout of this one.

## Maintainer docs

[CONTRIBUTING.md](CONTRIBUTING.md) · [SECURITY.md](SECURITY.md)

## License

MIT — see [LICENSE](LICENSE).
