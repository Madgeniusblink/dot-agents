# Optional skills (not in the template budget)

The shipped home keeps at most three skills under `home/skills/` so `agents check --template` can enforce the budget ([decision 0008](../decisions/0008-ship-no-fill-in-files.md), `budget.skills` in [home/docs/check.md](../../home/docs/check.md)).

Skills here are maintained in the glue tree and are **not** part of the template install. Copy a directory into your home when you want it:

```sh
cp -R docs/optional-skills/owner-decision ~/.agents/skills/
```

Adjust the destination if your home path differs.