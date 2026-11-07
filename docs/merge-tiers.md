# Merge tiers

A PR's tier comes from the paths it changes. The repository enforces the gate for each tier, so a session can merge what the gates allow and no more ([boundaries](../rules/boundaries.md)).

| Tier | Blast radius | Gate |
|---|---|---|
| green | Only this repository reads it: docs, tests, code that tests cover, rules, memory, and skills | The required checks pass. Auto-merge then merges it. |
| yellow | Another repository, fork, or tool uses it: an install script, a CI workflow, a dependency, the release setup, a file that forks read | The required checks pass, and the `review` job finds no stale fact. |
| red | It has no one-line undo, or it acts outside the repository: the gate itself, a secret, a publish, an authority boundary | A code owner, who is a person, approves it. |

Aim for about one red PR in twenty.

## The path lists

[.github/merge-tiers.yml](../.github/merge-tiers.yml) holds the yellow and red patterns. A PR is red when one changed path matches a red pattern. Otherwise it is yellow when one path matches a yellow pattern. Otherwise it is green. In a pattern, `*` matches any characters, `/` too.

[.github/CODEOWNERS](../.github/CODEOWNERS) has one line for each red pattern. GitHub reads only that file for code owners, so the red list is in both files. [scripts/merge-tier-test.sh](../scripts/merge-tier-test.sh) fails when the two differ, when a pattern names no path in the checkout, or when a fixture PR gets the wrong tier.

## The `review` job

[.github/workflows/review.yml](../.github/workflows/review.yml) runs on every PR:

1. It runs the tier test.
2. It decides the tier with [scripts/merge-tier.sh](../scripts/merge-tier.sh).
3. It writes one comment on the PR, or updates it. The comment names the tier and each path that set it.
4. A green or red PR passes at once. A yellow PR goes to a model with [review-prompt.md](../scripts/githooks/review-prompt.md), the diff, the list of tracked files, and `context/upstream.json`. The job fails when the last line of the reply is `BLOCK`.

The job reads the model key from the Actions secret `REVIEW_API_KEY`. A yellow PR fails when that secret is not set. The model ID is in the workflow.

## Turn it on

1. Add the secret: **Settings → Secrets and variables → Actions → New repository secret**, name `REVIEW_API_KEY`.
2. Make `review` a required check: **Settings → Branches → `main` → Require status checks to pass**, add `review`.
3. Require the code owner: **Settings → Branches → `main` → Require a pull request before merging → Require review from Code Owners**.

## In a fork

Edit the two path lists so they name your paths. Put your own owners in `.github/CODEOWNERS`. Change nothing else.
