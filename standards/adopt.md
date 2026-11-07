# Standard: adopt and update

Your agent home is a git checkout. There is no installer and no copy step, so a
file you edit is live at once.

## Start

```sh
git clone <your-repo> ~/.agents
```

Begin from this template with **Use this template** on the repository page,
make your copy private, then clone that.

## Update from upstream

```sh
cd ~/.agents
git remote add upstream <this-repo>
git fetch upstream && git merge --no-edit upstream/main
```

A conflict is the point: it shows you the line that changed under a rule you
rewrote. Resolve it and commit.

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
