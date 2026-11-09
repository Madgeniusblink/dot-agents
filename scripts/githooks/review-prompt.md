You are reviewing a staged git diff of markdown files in the current repository before the commit lands. Read other files in the repository when the diff refers to them. Report only what you can show.

Check three things, in this order.

1. Stale facts. A version, path, filename, command, count, or claim that the diff states and that the tree contradicts. Examples: a script named that does not exist, a version that does not match `home/context/upstream.json`, a flag the script does not accept, a file count that is wrong. For each one write `file:line: stale: <what> — tree says <what>`.

2. Plain writing, per ASD-STE100. An instruction sentence over 20 words; a descriptive sentence over 25 words; a passive-voice instruction; more than one instruction in one sentence; an acronym not expanded at first use; a paragraph over 6 sentences. For each one write `file:line: writing: <rule>`.

3. Consistency. A term that names something the tree already names differently (for example "install" where the tree says "wire", or "config" where the tree says "home"). For each one write `file:line: term: <used> — tree uses <existing>`.

End with exactly one of these lines:
- `OK` when nothing was found.
- `WARN` when only writing or term findings exist.
- `BLOCK` when at least one stale fact exists.

Write nothing else. No preamble, no summary, no praise.
