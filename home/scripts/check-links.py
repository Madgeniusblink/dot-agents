#!/usr/bin/env python3
"""Check that relative markdown links resolve to existing files.

Scans the home this script is in. With one argument, scans that directory
instead; the template repository passes its root to check the maintainer docs.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else Path(__file__).resolve().parent.parent
SCAN = [ROOT / "AGENTS.md", ROOT / "principles.md", ROOT / "README.md",
        ROOT / "rules", ROOT / "context", ROOT / "standards",
        ROOT / "skills", ROOT / "templates", ROOT / "docs"]
LINK = re.compile(r"\]\(([^)]+)\)")


def check_file(md: Path) -> int:
    errors = 0
    text = md.read_text(encoding="utf-8")
    for match in LINK.finditer(text):
        target = match.group(1).split("#")[0].strip()
        if not target or target.startswith(("http://", "https://", "mailto:")):
            continue
        resolved = (md.parent / target).resolve()
        if not resolved.exists():
            rel = md.relative_to(ROOT)
            print(f"broken: {rel} -> {target}")
            errors += 1
    return errors


# A merge command without --no-edit opens an editor and waits. In a shell no
# one is watching, that is indistinguishable from a hang.
BAD_MERGE = re.compile(r"git (?:-C \S+ )?merge (?!--no-edit|--abort|--continue)")


def check_merges(md: Path) -> int:
    errors = 0
    for i, line in enumerate(md.read_text(encoding="utf-8").splitlines(), 1):
        if BAD_MERGE.search(line):
            print(f"{md.relative_to(ROOT)}:{i}: git merge without --no-edit")
            errors += 1
    return errors


def main() -> int:
    errors = 0
    for path in SCAN:
        if path.is_dir():
            for md in path.rglob("*.md"):
                errors += check_file(md) + check_merges(md)
        elif path.is_file():
            errors += check_file(path) + check_merges(path)
    if errors:
        return 1
    print("link and command check OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
