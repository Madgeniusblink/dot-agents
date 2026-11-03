# Principles

1. **Add only what the model gets wrong.** Run the task without the file. If
   the result already passes, do not add the file. Delete a file when the model
   outgrows it.
2. **Match the container to the content.** Boundaries, preferences, and
   corrections go in `rules/`, one file per decay clock. A fact goes in `memory/`. An output shape goes in `templates/`. A
   procedure with ordering the model cannot derive goes in `skills/`. Rules are
   the default. A skill is the exception.
3. **A skill invokes itself.** Its `description` is the trigger, not a label.
   Write it for discovery: name the situation, not the feature. If it only
   fires when a human types its name, it is not a skill.
4. **Link, do not copy.** Point at the owning source. Keep no local copy of
   vendor documentation.
5. **State the outcome, not the procedure.** A better model finds a better
   route. A written route caps it at today's best idea.
6. **One idea per file.** Keep a skill under 120 lines. Longer detail goes in
   that skill's `references/`.
7. **Add beside the base; do not edit it.** Install updates every shipped
   file you have not touched. Your own rules, facts, and skills go in new
   files. Change a shipped file upstream.
8. **More intelligence needs less text, not more.** Corrections expire;
   boundaries do not. If the set grows while models improve, something is in
   the wrong file.
