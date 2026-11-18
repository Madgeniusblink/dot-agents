# 0013 HTTP client for Owner Decision Queue (attohttpc)

## Status

Accepted

## Context

Issue #135 adds `agents owner-decision` subcommands that call Slack Lists Web
API methods (`slackLists.items.*`). The CLI needs HTTPS POST with JSON bodies and
rustls-backed TLS without pulling OpenSSL dev packages onto every CI runner.

## Decision

Add **attohttpc** `0.28.2` with `json` and `tls-rustls` as the only new direct
dependency for outbound HTTP. Transitive crates from rustls, icu, and url land in
`Cargo.lock` and are listed in `cli/crates.txt`.

## Consequences

- Owner Decision Queue integration tests mock HTTP on loopback; production calls
  `https://slack.com/api/`.
- Future HTTP from `agents` should reuse this crate rather than adding a second
  client stack.