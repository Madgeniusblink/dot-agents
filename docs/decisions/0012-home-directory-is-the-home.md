# 0012 The home is `home/`; the repository root is glue

## Status

Proposed

## Context

The root of this repository has been the agent home since 0001. A clone of it is a `~/.agents`. That clone also carries everything the repository needs and the agent does not: `cli/` source, `CHANGELOG.md`, `CONTRIBUTING.md`, `SECURITY.md`, `LICENSE`, `dist-workspace.toml`, `.github/`, `docs/decisions/`, the merge-tier scripts, and the release files. A private fork inherits all of it and must delete it by hand on every merge, or keep it. The owner's fork has deleted the merge-tier glue and edited `CONTRIBUTING.md` and `README.md` to get rid of the rest.

Homebrew keeps a formula apart from the files it installs. XDG keeps a binary, its config, and its data in three places. The CLI already follows that: the binary is in `~/.cargo/bin`, the config in `~/.config/agents/config.toml`, and the home in `~/.agents`. Only the home itself was not lean.

Three options were compared.

**(a) An `agents install` or `sync` that copies content from a clone elsewhere into `~/.agents`.** This is the installer that 0006 built and [adopt.md](../../home/standards/adopt.md) removed. The home stops being a checkout, so `git status` and `git diff` stop answering what changed, and a manifest and orphan detection come back.

**(b) A content directory inside this repository.** `home/` holds everything a model reads. The root holds the glue. A fork keeps its root as the home and merges upstream with `git merge --no-edit -X subtree=home upstream/main`. Git shifts the template tree down to `home/` before the three-way merge, so a file outside `home/` is a deletion on the template side and leaves the fork. The merge base is still a real commit, so later merges are ordinary.

**(c) Sparse checkout or `export-ignore`.** Cone-mode sparse checkout always includes the files at the root, so it cannot hide `CHANGELOG.md`. Non-cone mode is marked "not recommended" by git. Both are per-clone settings that a fork's other machines do not get. `export-ignore` affects `git archive` only.

The `-X subtree=home` merge was tested on a scratch repository in three shapes. A fork started from the current root layout merged the moved template and lost every glue file. A new home made from `home/` with `git commit-tree` merged later template commits. A copy made with GitHub's template button, which has an unrelated history, merged once with `--allow-unrelated-histories` and then normally.

## Decision

Take option (b).

- `home/` is the agent home. It holds `AGENTS.md`, `principles.md`, `rules/`, `context/`, `skills/`, `standards/`, `templates/`, `memory/`, `decisions/`, `docs/` for the home, and `scripts/` for wiring and checking. It also holds a short `README.md` and a `.gitignore`, so a fork's own `README.md` and `.gitignore` meet a file on the template side and the fork's `merge=ours` attribute applies.
- The root holds the glue: the `cli/` source, the template's `README.md`, `CHANGELOG.md`, `CONTRIBUTING.md`, `SECURITY.md`, `LICENSE`, `dist-workspace.toml`, `.github/`, `docs/decisions/`, the maintainer docs, and the scripts that check the template and not a home.
- A home is still a git checkout. Nothing is copied. A home's root is `home/` from this repository, and its history is this repository's history.
- `agents update` adds `-X subtree=home` when `upstream/main` has `home/AGENTS.md` and the home has no `home/` directory. It adds `--allow-unrelated-histories` when the two histories share no commit.
- `agents init <url>` clones a copy that is already a home as before. When the clone has `home/AGENTS.md` and no root `AGENTS.md`, it makes one commit whose tree is `home/` and whose parent is the clone's head, then resets to it. When the clone is empty, it fetches the template and does the same.
- A link inside `home/` resolves inside `home/`. A link from `home/` to a maintainer file is a GitHub URL.
- CI runs the home's checks on `home/` and the template's checks on the root.

### Migrating a private fork whose root is the home

Run these once in the fork. The first merge after this change deletes the glue and keeps every file the fork added.

```sh
cd ~/.agents
git fetch upstream
git merge --no-edit -X subtree=home upstream/main
```

A glue file the fork edited and the template moved or dropped is a modify/delete conflict. `git status --porcelain | awk '$1 == "DU" || $1 == "UD" { print $2 }'` lists them. Keep or drop each with `git checkout --ours -- <path> && git add <path>` or `git rm -- <path>`, then `git commit --no-edit`. A fork's `merge=ours` lines for `README.md`, `.gitignore`, and `docs/repo-rules.md` resolve the first two. The template no longer ships `docs/repo-rules.md` in the home, so that file is one modify/delete conflict the first time, and never again.

`agents sync --apply` runs the same merge from the next release of the binary. A fork that has not merged this change yet uses the command above.

## Consequences

- A fork is lean by construction. The owner's fork drops `cli/`, `CHANGELOG.md`, `CONTRIBUTING.md`, `SECURITY.md`, `LICENSE`, `dist-workspace.toml`, `.github/`, `docs/decisions/`, and the merge-tier scripts on its next merge, and deletes nothing by hand after that.
- A developer's checkout of this repository is not a home. `agents --home <checkout>/home` reads the home in it, and `Home::locate` says so when it finds `home/AGENTS.md`.
- Every path in CI, the merge tiers, the neutrality check, and the CLI tests gains the `home/` prefix. `scripts/check.sh` at the root runs the home's checks and the template's checks in one command.
- `-X subtree=home` is one more flag in the update command. The binary adds it; the documented command carries it. A fork that forgets it gets the glue back as new files, which `git status` shows and a second merge with the flag removes.
- The template's `README.md` on GitHub describes the repository. The home's `README.md` describes the home. The two are different files and stay short.
- 0009 holds: one CLI in the tree, embedding no content. 0004 holds: no hook binary in the home. 0008 holds: `memory/` and `decisions/` ship empty.
