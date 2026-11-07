# JSON output

`agents status`, `scan`, `skills`, `memory`, and `check` take `--json`. With it, stdout
holds one JSON document and nothing else. The exit code does not change. Save
the document to a file and hand it to another session for analysis.

A path under your home directory starts with `~`. A key with no value is
`null`. The `agents` version is the only schema version.

## Every document

The envelope comes first. The command's own keys follow it.

| Key | Value |
|---|---|
| `agents` | The CLI version, such as `0.17.0` |
| `build` | `commit`, the short commit the binary was built from, and `main_ahead`, how many commits `main` is ahead of it from the once-a-day check, or `null` when the check is off or did not answer |
| `command` | `status`, `scan`, `skills`, `memory`, or `check` |
| `generated` | When the document was made, RFC 3339 in UTC |
| `home` | The agent home, or `null` when there is none |

## status

| Key | Value |
|---|---|
| `machine` | The name this machine gives itself, or `unknown` |
| `config` | The config file for this machine, below |
| `git` | `null` when the home is not a git checkout; else `branch`, `sha`, `uncommitted` (a count), and `behind_upstream` (a count, or `null` with no `upstream/main`) |
| `merge_ours_unset` | `true` when `.gitattributes` marks files `merge=ours` and `merge.ours.driver` is not set |
| `always_on` | `files`, each with `name` and `bytes`; `bytes`, the total; the `cap` it must fit; and `cap_source`, as in `check` |
| `on_demand` | One object per directory: `dir` and `files`, a count |
| `harnesses` | One object per harness file, below |
| `manual` | `null`, or `label` and `line`, the pointer line to paste into a tool that keeps rules in app settings |
| `hooks` | `null` when the settings file is missing; else `settings`, `valid_json`, and `missing_paths` |
| `links` | One object per link into the home: `path`, `target`, and `broken` |

The `config` object:

| Key | Value |
|---|---|
| `path` | The file: `$AGENTS_CONFIG`, else `~/.config/agents/config.toml` |
| `exists` | `false` when there is no file, so every key has its default |
| `editor` | The string, or `null` when it is empty or unset |
| `scan` | `roots` and `skip`, each a list of strings |
| `tools` | `present`, a list of tool names; empty means every tool found |

Each object in `harnesses`:

| Key | Value |
|---|---|
| `label` | The label the text shows for the file |
| `path` | Where the harness reads |
| `exists` | `false` when the harness loads nothing from there |
| `link` | The link target, or `null` when the file is not a link |
| `lines`, `bytes` | The size of the file |
| `imports` | `home` and `elsewhere` (counts), and `broken`, each import line whose target is gone |
| `over_cap` | `null`, or `bytes`, the size of the file, and the `cap` they pass |
| `stale` | `true` when the home changed after `wire` wrote the file |
| `own_rule_lines` | `null`, or the count of rule lines kept in the file instead of the home |
| `generator_lines` | Each `line:text` that says a generator owns the file |

## scan

| Key | Value |
|---|---|
| `machine` | The name this machine gives itself, or `unknown` |
| `config` | The config file for this machine, as in `status` |
| `root` | The first directory walked |
| `roots` | Every directory walked: `--root`, else `[scan] roots`, else your home directory |
| `seconds` | How long the walk took |
| `skipped` | The directory names the walk skipped; empty with `--all` |
| `count` | How many instruction files it found |
| `files` | One object per file, below |
| `projects` | One object per checkout, below |
| `auto_memory` | `root`, and `indexes`, each with `index` and `lines` |
| `identical` | Groups of paths with the same content |
| `identical_remotes` | One list per group in `identical`: each `remote` with more than one file in the group, and `checkouts`, the count |
| `same_remote` | One object per remote in more than one place: `remote`, and `checkouts`, the paths, newest last commit first |
| `settings` | One object per tool settings file, below, with `scope` and `project` as in `files` |

Each object in `projects`:

| Key | Value |
|---|---|
| `root` | The checkout |
| `skills` | A count per place a tool reads |
| `remote` | `owner/repo` from `origin`, or `null` |
| `kind` | `clone`, `worktree`, `submodule`, or `orphan worktree` |
| `main_checkout` | The main checkout of a worktree, the superproject of a submodule, else `null` |
| `branch` | The branch, or `null` |
| `ahead`, `behind` | Counts against the upstream ref on disk, or `null` with no upstream |
| `uncommitted` | How many files have changes not committed |
| `last_commit` | The date of the last commit, `YYYY-MM-DD`, or `null` |
| `stale` | `true` for an orphan, or a last commit older than 30 days with nothing ahead |
| `prune` | For an orphan, the command that clears its record from the source, else `null` |

Each object in `files`:

| Key | Value |
|---|---|
| `path` | The file |
| `scope` | `home`, `global`, `project`, `loose`, `moved-aside`, or `plugin-cache` |
| `project` | The checkout root for scope `project`, else `null` |
| `lines`, `bytes` | The size of the file |
| `link` | The link target, or `null` |
| `generated` | The line that says a generator wrote the file, or `null` |

Each settings object:

| Key | Value |
|---|---|
| `path` | The file |
| `tool` | The tool that reads it |
| `bytes` | The size of the file |
| `validity` | `valid`, `invalid`, or `empty` |
| `error_line`, `error` | For `invalid`, the line of the parse error and the error, else `null` |
| `hooks` | Each hook: `event`, `command`, and `dead`, `true` when a path it names is gone |
| `deny` | Each `permissions.deny` entry |
| `allow` | The count of `permissions.allow` entries |
| `dead_allow` | Each allow entry that names a path that is gone |
| `env` | Each environment variable name the file sets |
| `model`, `effort` | The model and effort the file sets, or `null` |

## scan --from

| Key | Value |
|---|---|
| `machine`, `config` | As in `scan` |
| `from` | The directory a session starts in |
| `checkout` | The checkout that holds it, or `null` |
| `tools` | One object per tool: `tool`, `order` (`documented` or `unknown`), `note`, and `files`, lowest precedence first |
| `conflicts` | Each deny that blocks an allow in another file: `rule`, `deny` (its file), and `allow` (its file) |
| `edit` | The line that opens the most specific applying file in `$EDITOR`, or `null` |

Each object in `files` is a settings object, plus `scope` (`user`, `project`, or `local`) and `applies`, `false` when the file does not parse.

## skills

| Key | Value |
|---|---|
| `count` | How many skills tools can load |
| `skills` | One record per skill, below |
| `links` | Each link in the user skills directory: `path` and `target` |
| `duplicates.same_name` | One object per name in more than one place: `name`, `identical`, and `places`, each with `scope` and `dir` |
| `duplicates.same_body` | One object per body under more than one name: `names` and `dirs` |

Each record:

| Key | Value |
|---|---|
| `name` | The frontmatter name, or the directory name |
| `dir` | The skill directory |
| `scope` | `yours`, `managed`, or `project` |
| `source` | `home`, `claude-user`, `codex-user`, `claude-synced`, `claude-plugin`, or `project` |
| `project` | The checkout root for scope `project`, else `null` |
| `owner` | `metadata.owner`, or `null` |
| `frontmatter` | `null` when `SKILL.md` has none; else `name`, `description`, `license`, `compatibility`, `metadata`, `allowed_tools`, and `unknown_keys` |
| `lint` | Each finding: `rule`, `level` (`error` or `warn`), and `text` |
| `body` | `lines` of the whole file, `bytes` and `hash` of the body after the frontmatter |
| `files` | Counts of `scripts`, `references`, `assets`, and `other`, and their total `bytes` |
| `refs` | `ok`, a count, and `broken`, each relative reference that does not resolve |
| `copy` | `copy of <path> (declared)`, `copy of <path> (identical)`, `source missing`, or `null` |

## memory

| Key | Value |
|---|---|
| `root` | Where the tool keeps its auto memory |
| `repositories` | How many repositories have notes |
| `count` | How many notes |
| `covered` | How many notes the home covers |
| `notes` | One object per note, below |
| `same_note` | One object per note file in more than one repository: `file` and `repositories` |

Each object in `notes`:

| Key | Value |
|---|---|
| `name` | The frontmatter name, or the file stem |
| `repository` | The memory directory that holds the note |
| `file` | The note |
| `type` | The frontmatter type |
| `lines` | The size of the note |
| `description` | The frontmatter description |
| `verdict` | `covered`, `similar`, or `none` |
| `match` | The place in the home that matches best, or `null` |
| `score` | The overlap score of that match, or `null` |

## check

| Key | Value |
|---|---|
| `ok` | `true` when the home has no fault |
| `always_on` | `bytes`, the size of principles plus every rule file, the `cap` it must fit, and `cap_source`, `default cap` or `cap from context/upstream.json` |
| `faults` | One object per fault: `path`, `line`, `rule`, `message`, and `fix`, the same fields as the text line. [check.md](check.md) lists every rule |
