# Principles

1. **Add only what the model gets wrong.** Model capability is the baseline.
   A file is admitted for one of four things: a durable human preference, a
   non-obvious operating fact, an authority boundary, or a demonstrated
   recurring failure. Generic encouragement, standard engineering advice,
   speculative edge cases, and one-off detail are not admitted, and nothing is
   admitted because it sounds useful. Compare the failure and one boundary case
   with and without the candidate; keep a removal when outcomes stay correct.
2. **Match the container to the content.** Boundaries, preferences, and
   corrections go in `rules/`, one file per decay clock. A fact goes in `memory/`. An output shape goes in `templates/`. A
   procedure with ordering the model cannot derive goes in `skills/`. Rules are
   the default. A skill is the exception.
3. **A skill invokes itself.** Its `description` is the trigger, not a label.
   Write it for discovery: name the situation, not the feature. If it only
   fires when a human types its name, it is not a skill.
4. **Link, do not copy.** Point at the owning source. Keep no local copy of
   vendor documentation.
5. **Find the owner before you add.** Replace or consolidate the existing rule
   instead of appending another version. Reject a duplicate even when its
   wording or its file differs.
6. **State the outcome, not the procedure.** A better model finds a better
   route. A written route caps it at today's best idea.
7. **One idea per file.** Keep a skill under 120 lines. Longer detail goes in
   that skill's `references/`.
8. **Add beside the base; do not edit it.** Install updates every shipped
   file you have not touched. Your own rules, facts, and skills go in new
   files. Change a shipped file upstream.
9. **Write plainly; match the repo.** Prose follows
   [ASD-STE100](context/writing-standard.md). Code matches the owning repo.
   With no repo style, keep the diff small and match nearby code.
10. **One fact per file in `memory/`.** Review monthly and delete stale files.
   Log a lesson as `YYYY-MM-DD — mistake — fix`. Promote a repeat into
   `rules/corrections.md`.
11. **More intelligence needs less text, not more.** Corrections expire;
   boundaries do not. If the set grows while models improve, something is in
   the wrong file.
