# Agent rules

Router for this agent home. It names folders, not files. A file you add to a
folder loads without an edit here.

## Every session

1. Read [principles.md](principles.md).
2. Read every `*.md` file in [rules/](rules/).
3. If `AGENTS.local.md` exists beside this file, read it. It holds your own
   additions. Where it conflicts with a shipped file, it wins.

## When the task needs it

| Folder | Holds | Use |
|--------|-------|-----|
| [memory/](memory/) | Facts: accounts, hosts, preferences, lessons | List the folder. Open files whose name or title matches the task. |
| [templates/](templates/) | Output shapes | List the folder. Fill the one that matches the output. |
| [skills/](skills/) | Procedures whose order the model cannot derive | Your tool finds them by `description`. Do not call one by name. |
| [checklists/](checklists/) | Done criteria | Open before commit or release. |
| [decisions/](decisions/) | Your settled choices | Read before you reopen a choice. |
| [context/](context/) | Foundation, writing standard, glossary | Open when a file links it. |
| [standards/](standards/) | How to add a file to this tree | Open before you add or change a file. |

## Global vs project

| Copy location | Use |
|---------------|-----|
| User agent home (e.g. `~/.agents/`) | Cross-repo rules, memory, personal decisions |
| `<repo>/.agents/` | Rules and skills for one codebase |

When both exist, prefer **project** files for repo work.

## Your changes

Install refreshes shipped files. Do not edit them; your edit is replaced.

- New rule: add a file to `rules/`.
- New fact: add a file to `memory/`.
- New skill or template: add a folder or file beside the shipped ones.
- Anything else for this router: `AGENTS.local.md`.

To change a shipped file, open a pull request on the upstream template
([upstream.json](context/upstream.json)). See [standards/extending.md](standards/extending.md).

## Hooks

Do not put git or IDE hooks in this tree. Use your tool's hook paths.

Most tools do **not** auto-load this tree. Point your tool's user rules at this
file.
