# Check rules

`agents check` holds the home to the rules in [standards/](../standards/). It prints one line per fault and nothing else, then exits 1:

```text
path:line rule: message. fix: <what to change>
```

A clean run prints one line with the always-on bytes, the cap, and where the cap came from, then exits 0. `--json` prints the same faults as one document ([json.md](json.md)). `--template` adds the two template rules at the end of the table. CI runs `agents check --template` on this repository.

| Rule | Message | Fix | Source |
|---|---|---|---|
| `router.lines` | `<n> lines, max 50` | move detail to a linked file | [check-budget.sh](../scripts/check-budget.sh) |
| `rule.one-line` | rule continues on an indented line, or paragraph spans more than one line | join the rule or paragraph into one line | [rules.md](../standards/rules.md) |
| `correction.dated` | correction does not end in a dated test | end the line with `` `YYYY-MM · task that proves it` `` | [rules.md](../standards/rules.md) |
| `skill.missing` | directory has no SKILL.md | add SKILL.md, or move the directory out of `skills/` | [skills/README.md](../skills/README.md) |
| `skill.frontmatter` | SKILL.md does not open with a block between `---` lines | edit the frontmatter to meet the Agent Skills standard | [skills.md](../standards/skills.md) |
| `skill.name-required` | no name | same | same |
| `skill.name-length` | name has `<n>` characters, allowed 1 to 64 | same | same |
| `skill.name-chars` | name has a character that is not a lowercase letter, digit, or hyphen | same | same |
| `skill.name-hyphens` | name starts or ends with a hyphen, or has two together | same | same |
| `skill.name-matches-dir` | name is `<name>`, directory is `<dir>` | same | same |
| `skill.description-required` | no description | same | same |
| `skill.description-length` | description has `<n>` characters, allowed 1 to 1024 | same | same |
| `skill.compatibility-length` | compatibility has `<n>` characters, allowed 1 to 500 | same | same |
| `skill.metadata-strings` | metadata value is not a string | same | same |
| `skill.lines` | body has `<n>` lines, max 120 | move detail to `references/` | [principles.md](../principles.md) |
| `memory.title` | file does not start with a `# ` title | make the first line `# <the fact>` | [principles.md](../principles.md) |
| `memory.one-fact` | second `# ` title | split the file, one fact per file | [principles.md](../principles.md) |
| `template.frontmatter` | template has frontmatter, and `standards/` names none | remove the frontmatter block | [standards/](../standards/) |
| `link.relative` | link target `<path>` does not exist | point the link at a file that exists | [sources-policy.md](../context/sources-policy.md) |
| `link.merge-no-edit` | a documented merge command lacks `--no-edit` | add `--no-edit` to the command | [adopt.md](../standards/adopt.md) |
| `budget.always-on` | principles and rules hold `<n>` bytes, the cap is 12 KiB | move text out of the always-on files, or raise `budget.always_on_bytes` in `context/upstream.json` | [rules.md](../standards/rules.md) |
| `budget.skills` | the template ships more than 3 skills (`--template` only) | move the skill out of the template | [decision 0008](https://github.com/Madgeniusblink/dot-agents/blob/main/docs/decisions/0008-ship-no-fill-in-files.md) |
| `budget.template-files` | the template ships a file in `memory/` or `decisions/` (`--template` only) | move the file out of the template | [decision 0008](https://github.com/Madgeniusblink/dot-agents/blob/main/docs/decisions/0008-ship-no-fill-in-files.md) |
