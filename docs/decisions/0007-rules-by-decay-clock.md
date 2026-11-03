# 0007 Sort rules by decay clock; corrections expire

## Status

Accepted

## Context

`rules/behavior.md` mixed gates, taste, and model corrections. Only corrections go stale as models improve, but the mixed file looked like it needed review forever. Several lines prescribed a route (read ranges, pipe output) that a better model would beat.

## Decision

- Split shipped rules into `rules/boundaries.md` (what needs a human) and `rules/corrections.md` (what the model misses). User preferences go in the user's own `rules/preferences.md`; the template ships none.
- Write corrections as outcomes: what must be true, not how to get there.
- Each correction ends with `` `YYYY-MM · task` ``: the month added and the task that proves it. On a model upgrade, run the task without the line; delete the line if the output holds. CI enforces the suffix.

## Consequences

The retirement test costs a few tokens per correction in every session; it keeps the death condition beside the line it retires. Boundaries are never reviewed. The correction file is expected to shrink.
