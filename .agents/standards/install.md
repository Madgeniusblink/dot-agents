# Standard: install

Target: **agent home** directory (default `~/.agents` on Unix, `$HOME\.agents` on Windows when `$HOME` is set). Not disk `/`.

## Scripts

From a clone (see [upstream.json](../context/upstream.json) → `links.repo` or `links.raw_tag`):

```sh
./install.sh
```

Windows: `.\install.ps1`. `DOT_AGENTS_HOME` overrides the destination.

## Manifest

Install writes `.installed` in your home: one SHA-256 per file, as installed. On the next install, per shipped file:

| On disk | Action |
|---------|--------|
| Missing | Add it |
| Same as the new version | Nothing to do |
| Same as `.installed` (you did not touch it) | Replace with the new version |
| Anything else (you edited it, or it predates the manifest) | Keep it and print `kept (edited)` |

A file the template stopped shipping is removed if unchanged, else kept.

Your own files (`rules/mine.md`, `memory/*`, `decisions/*`) are never shipped, so install never touches them. An edited shipped file stops getting updates; delete it to take the current version.

## Project copy

```sh
DOT_AGENTS_HOME=/path/to/repo/.agents ./install.sh
```
