# Standard: schedules

No single global URL defines "agent schedules." Use platform docs for timers.

## Live references

- macOS: [launchd.plist](https://developer.apple.com/library/archive/documentation/MacOSX/Conceptual/BPFundamentals/LaunchServices/LaunchServices.html)
- Linux: [cron](https://man7.org/linux/man-pages/man5/crontab.5.html)

## Local minimum

Path: `.agents/schedules/<name>.md`

Each file describes **one** recurring task:

1. **Name** — short title
2. **Cadence** — daily, weekly, or cron expression
3. **Steps** — links to [checklists/](../checklists/), [workflows/](../workflows/), or skills
4. **Owner** — human or agent role

Example: [weekly-cleanup.md](../schedules/weekly-cleanup.md).

## Rules

1. STE100 prose ([writing-standard](../context/writing-standard.md)).
2. Do not embed secrets. Point to [tools/](../tools/) for commands.
3. Wire execution in the host OS or provider; this folder is the **spec**.
