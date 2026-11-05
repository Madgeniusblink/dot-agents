#!/usr/bin/env python3
"""Check that relative markdown links resolve to existing files."""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
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


def main() -> int:
    errors = 0
    for path in SCAN:
        if path.is_dir():
            for md in path.rglob("*.md"):
                errors += check_file(md)
        elif path.is_file():
            errors += check_file(path)
    if errors:
        return 1
    print("link check OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
