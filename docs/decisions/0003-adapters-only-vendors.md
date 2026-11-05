# 0003 Vendor names only in adapters

## Status

Superseded by [0005](0005-dynamic-base-layer.md). `adapters.md` is removed; no shipped file names a vendor.

## Context

Neutral template must not favor one agent product.

## Decision

Product names and vendor doc links live only in `standards/adapters.md`. Other files say "your tool."

## Consequences

Neutrality grep checks exclude `adapters.md` only.
