# Spec: dot-agents deterministic engineering core

**Revision:** 1  
**Source repo:** `Madgeniusblink/dot-agents` @ `20bf16c786fc22a51fc634b1de211badad033669` (branch `cursor/readme-dynamic-tree`)  
**Product plan:** Stage 1 outcomes in chat (2026-11-02)  
**Status:** `spec:draft` — engineer-ready self-check; independent `plan-gate` not run in this session

---

## 1. Traversal and scope

### User outcome

An engineer, founder, or CTO can adopt a **global agent home** (`~/.agents/`) and optional **project layer** (`.agents/`) with **deterministic** install paths, ephemeral locations, verification, and docs—without provider-specific hooks or maintained vendor copies.

### Top-down (entry → infrastructure)

| Layer | This change |
|-------|-------------|
| Entry surface | `AGENTS.md` router; optional Cursor user-rules bootstrap |
| Module | `dot-agents` CLI (install, doctor, status, readme sync/check) |
| Contract | Env vars + [data-locations.md](../.agents/context/data-locations.md); [var-layout.md](../.agents/standards/var-layout.md) |
| Data | Tracked template under `.agents/`; ephemeral under `var/scratch`, `var/worktrees` (gitignored) |
| Configuration | `DOT_AGENTS_HOME`, `DOT_AGENTS_SCRATCH`, `DOT_AGENTS_WORKTREES`, optional `DOT_AGENTS_SOURCE` |
| CI | `.github/workflows/readme-tree.yml` |
| Providers | Document only; no auto-load claim |

### Bottom-up (dependents)

- **Users** depend on CLI + env contract for install and paths.
- **Maintainers** depend on `readme sync` + CI for README accuracy.
- **Projects** copying `.agents/` depend on `PROJECT-GITIGNORE-SNIPPET.md` and `var/**/.gitignore`.
- **Future release binaries** depend on same embedded template as `cargo install` (out of slice 1).

### Non-goals

- Git/IDE hooks under `~/.agents/`.
- Profiles, evals, routing JSON in base template.
- Mirroring vendor documentation.
- Automatic provider discovery of `~/.agents/` without user wiring.

---

## 2. Goal (pointer)

Deliver the **deterministic engineering surface** in section 9 so behaviors B1–B8 are testable.

---

## 3. Acceptance (behaviors)

| ID | Assertion | Proof |
|----|-----------|--------|
| **B1** | `dot-agents install` with no checkout writes `~/.agents/AGENTS.md`, `standards/README.md`, `context/writing-standard.md`, `var/scratch/.gitignore`, `var/worktrees/.gitignore`. | `DOT_AGENTS_HOME=$(mktemp -d) dot-agents install && test -f …` |
| **B2** | `dot-agents doctor` exits 0 when B1 layout present; exits non-zero with message to run install when core files missing. | Shell test in `cli/` or CI job |
| **B3** | `dot-agents status` prints CLI version, home path, file count, and **embedded template version** matching `cli/Cargo.toml` version. | stdout regex test |
| **B4** | `dot-agents paths` (new) prints resolved `home`, `scratch`, `worktrees` with defaults `~/.agents`, `~/.agents/var/scratch`, `~/.agents/var/worktrees`, honoring env overrides. | Command test with env set |
| **B5** | `dot-agents readme check` fails when README tree block stale; passes after `readme sync`. | CI workflow + local test |
| **B6** | Repo `.gitignore` ignores contents of `.agents/var/scratch/**` and `.agents/var/worktrees/**` except README and `.gitignore`. | `git check-ignore -v` on dummy file |
| **B7** | [data-locations.md](../.agents/context/data-locations.md) documents global vs project scratch/worktree choice in STE100 prose. | Doc review + link from AGENTS.md |
| **B8** | [bootstrap/cursor-user-rules.md](../bootstrap/cursor-user-rules.md) gives copy-paste user rules: load `~/.agents/AGENTS.md`, cite scratch/worktree env paths (no secrets). | File exists; manual smoke |

---

## 4. Out of scope (spec)

- GitHub Release binaries (recommend slice 2).
- `dot-agents install --mirror` default change.
- Content inside `memory/lessons.md` (user-owned).
- Merging PR #2 (human merge).

---

## 5. Contracts and types

### Environment (deterministic resolution)

| Variable | When unset |
|----------|------------|
| `DOT_AGENTS_HOME` | `$HOME/.agents` |
| `DOT_AGENTS_SCRATCH` | `$DOT_AGENTS_HOME/var/scratch` |
| `DOT_AGENTS_WORKTREES` | `$DOT_AGENTS_HOME/var/worktrees` |
| `DOT_AGENTS_SOURCE` | Dev: `./.agents` if present; else embedded |

### CLI commands (stable)

`install`, `status`, `doctor`, `paths`, `readme sync|check|print`

### Ephemeral git contract

Each of `var/scratch`, `var/worktrees`: `.gitignore` contains `*`, `!.gitignore`, `!README.md`.

---

## 6. Invariants

- Embedded template at CLI build matches `.agents/` in repo at that tag.
- `STRUCTURE.md` never installed to `~/.agents/`.
- No tracked file under `var/scratch` or `var/worktrees` except README + `.gitignore`.

---

## 7. Failure handling

| Failure | Behavior |
|---------|----------|
| Missing `HOME` | `install`/`paths` error with clear message |
| `readme sync` outside repo | Error: need README.md + `.agents/` |
| Doctor missing files | Non-zero exit; list missing paths |
| Stale README in CI | `readme check` fails PR |

---

## 8. Compatibility and observability

- macOS/Linux; Rust 1.70+.
- `dot-agents status` is the support bundle (version, home, file count, template version).

---

## 9. Data and interfaces (section 9 detail)

### Tracked template (user-facing engineering canon)

| Area | Purpose | User edits? |
|------|---------|-------------|
| `standards/` | How to add skills, rules, var layout | Rarely |
| `context/` | STE100, sources policy, **data-locations** | Glossary/architecture yes |
| `checklists/` | Before commit/release | Optional |
| `memory/` | Lessons, preferences | **Yes** |
| `decisions/` | Settled choices | **Yes** |
| `skills/self-verify` | Verification procedure | Optional |
| `rules/README` | When to add scoped rules | N/A until rules added |
| `templates/pr-description` | Gap template only | Optional |
| `var/` | Scratch + worktree **roots** only | Paths fixed; contents ephemeral |

### Not in base template (documented only)

Optional extensions: profiles, evals, workflows, MCP JSON — [optional-extensions.md](../.agents/context/optional-extensions.md).

### Deterministic vs user-focus

| Deterministic (spec owns) | User-focus (canon layout, user content) |
|---------------------------|----------------------------------------|
| CLI install/doctor/paths | `memory/lessons.md` entries |
| Env path resolution | `memory/preferences.md` |
| gitignore for var | Scratch **files** |
| readme sync/check | Worktree **checkouts** |
| bootstrap snippet | Project `.agents/` copy per repo |
| STE100 + link-first standards | `decisions/` content |

---

## 10. Agent-ready execution scope (slice 1 — one session)

**Outcome:** B3, B4, B8 implemented and B1–B2, B5–B7 verified on branch.

**Branch:** `cursor/readme-dynamic-tree` (or successor) → merge via PR #2+

**Modules:** `cli/src/main.rs`, `cli/src/paths.rs`, `bootstrap/cursor-user-rules.md`, tests in `cli/tests/`

**Commands:**

```sh
cd cli && cargo test
cargo run -- readme check
DOT_AGENTS_HOME=$(mktemp -d) ./target/debug/dot-agents install
./target/debug/dot-agents doctor
./target/debug/dot-agents paths
```

**Behavior IDs in slice:** B3, B4, B8 (+ regression B1, B2, B5, B6, B7)

**Blocked-by:** None if PR branch contains var/ + readme work.

**Not validated in slice:** Release binaries (B9 future).

---

## Gate self-check (engineer-ready)

| Criterion | Met? |
|-----------|------|
| Behaviors B1–B8 with IDs | Yes |
| Data model / interfaces | Section 5, 9 |
| Traversal both directions | Section 1 |
| Failure handling | Section 7 |
| No open product decisions in spec | Binary releases deferred to slice 2 with recommendation |
| Independent plan-gate | **Not met** — author self-check only |

## Gate self-check (agent-ready for slice 1)

| Criterion | Met? |
|-----------|------|
| One session outcome | Yes |
| Behavior IDs cited | B3, B4, B8 + regression |
| Objective check + command | `cargo test`, readme check |
| Files named | cli, bootstrap |
| No executor decisions | Yes |

**Verdict:** Spec is **engineer-ready (self-check)** for full B1–B8; **agent-ready (self-check)** for slice 1. Independent `plan-gate` reviewer still required per org skills before `/implement` in strict mode.

---

## Next stage

Human: merge PR #2, choose **implement slice 1** or full B1–B8.

Question (delegate-spec): **Implement slice 1 directly from this spec (recommended), or tickets first?**
