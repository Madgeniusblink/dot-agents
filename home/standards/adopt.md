# Standard: adopt and update

Your agent home is a git checkout. There is no installer and no copy step, so a
file you edit is live at once.

## Start

```sh
git clone <your-repo> ~/.agents
```

The template repository keeps the home under `home/` and its own files, the
binary's source and the maintainer docs, at the root. A home carries `home/`
only. To start one from the template, clone it as `upstream`, make one commit
whose tree is `home/`, then add your own private repository as `origin`:

```sh
git clone --origin upstream <template-repo> ~/.agents
cd ~/.agents
git reset --hard "$(git commit-tree -p HEAD -m 'feat: make the root the home' HEAD:home)"
git remote add origin <your-repo> && git push -u origin main
```

`agents init <your-repo> --apply` does the same in one step, for an empty
repository or for a copy made with **Use this template**.

## Update from upstream

```sh
cd ~/.agents
git fetch upstream && git merge --no-edit -X subtree=home upstream/main
```

`-X subtree=home` shifts the template tree down to `home/` before the merge,
so a file the template keeps outside `home/` never enters the home. A copy
made with **Use this template** shares no commit with the template, so its
first merge adds `--allow-unrelated-histories`. `agents sync --apply` adds
both flags when they apply.

A conflict is the point: it shows you the line that changed under a rule you
rewrote. Resolve it and commit.

A home whose root came from an earlier template, where the root was the
home, takes this change with the same merge. The glue it inherited leaves on
that merge. A glue file you edited is a modify/delete conflict once: keep it
with `git checkout --ours -- <path> && git add <path>`, or drop it with
`git rm -- <path>`, then `git commit --no-edit`.

## Why git and not an installer

Git already answers every question a copy-based install has to reinvent.

| Question | Command |
|---|---|
| What did I change? | `git status` |
| How does my version differ? | `git diff` |
| What did upstream change? | `git log upstream/main` |
| Remove a file upstream dropped | `git merge` does it |
| Go back | `git revert` |

A copy cannot answer any of them, which is why a copy-based install needs a
manifest, orphan detection, and a report of files it kept. None of that is
needed here.

## Keep private content private

The checkout holds your memory and preferences. Keep the repository private and
commit no secret value. Point at a secret by the name of its entry in your
password manager.

## Project use

A repository gets its own `.agents/` directory, committed with that repository.
Copy the files you want into it. It is not a checkout of this one.
