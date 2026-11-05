# Sources policy

**Link first. Template second.**

## Rule

1. Search for an **official or primary** URL (vendor docs, spec site, RFC, standard body).
2. Put that URL in the local file. Add **one paragraph** of how you use it in this tree.
3. Add a new file to this tree only when no credible URL exists, or you need a dated snapshot (note source and date in the file). For pull requests on this git repo, use [`.github/pull_request_template.md`](../.github/pull_request_template.md).

## Credible sources (examples)

| Topic | Primary URL |
|-------|-------------|
| Agent entry file | [agents.md](https://agents.md/) |
| Simplified English | [ASD-STE100](https://www.asd-ste100.org/) |
| Architecture decisions | [adr.github.io](https://adr.github.io/) |

Tool-specific docs change often. Do not copy or pin them here; say "your tool" and let the reader use their tool's current docs.

Do not treat blogs or forks as primary if an official doc exists.

## Dynamic data

Prefer live URLs in `references/` inside skills.

## Maintenance

You are not in the business of maintaining vendor text. You maintain **pointers**, **personal memory**, and **corrections** ([rules/](../rules/)).
