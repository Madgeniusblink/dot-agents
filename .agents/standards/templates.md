# Standard: templates

**Do not maintain templates when a URL defines the shape.**

1. Check [sources-policy](../context/sources-policy.md).
2. If [ADR](https://adr.github.io/) or another URL applies, link it in your skill or checklist. Skip a local file.
3. Add under `.agents/templates/` only for gaps (e.g. [pr-description.md](../templates/pr-description.md)).

Each template file:

- Title
- Section headings with empty guidance (one line per section)
- STE100 ([writing-standard](../context/writing-standard.md))

When a vendor publishes a stable template URL, replace the local file with a link in [standards/README.md](README.md).
