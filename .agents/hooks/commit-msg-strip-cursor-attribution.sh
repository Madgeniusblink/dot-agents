#!/bin/sh
# Remove Cursor agent trailers from commit messages (global commit-msg hook).
# Matches Co-authored-by: Cursor and Made-with: Cursor (case-insensitive).
msg="$1"
grep -viE '^(co-authored-by: cursor|made-with: cursor)' "$msg" > "$msg.tmp"
mv "$msg.tmp" "$msg"
