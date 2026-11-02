# Sources policy

**Link first. Template second.**

## Rule

1. Search for an **official or primary** URL (vendor docs, spec site, RFC, standard body).
2. Put that URL in the local file. Add **one paragraph** of how you use it in this tree.
3. Create a local [templates/](../templates/) file **only** when:
   - No credible URL exists, or
   - The URL is unstable and you need a frozen snapshot (note the date and source in the file).

## Credible sources (examples)

| Topic | Primary URL |
|-------|-------------|
| Agent entry file | [agents.md](https://agents.md/) |
| Simplified English | [ASD-STE100](https://www.asd-ste100.org/) |
| Architecture decisions | [adr.github.io](https://adr.github.io/) |

Tool-specific doc links belong in [standards/adapters.md](../standards/adapters.md).

Do not treat blogs or forks as primary if an official doc exists.

## Dynamic data

Prefer live URLs in `references/` inside skills.

## Maintenance

You are not in the business of maintaining vendor text. You maintain **pointers**, **personal memory**, and **verification** ([checklists/](../checklists/)).
