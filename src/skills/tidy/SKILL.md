---
name: tidy
description: Find and remove banco module data that is no longer backed by the configuration — repositories dropped from a provider, task trees whose syncing was turned off, and local items the user wants to retire. Use when the user asks to tidy, clean up, prune, or remove outdated/stale/orphaned data, or after they change provider configuration.
---

# Tidy

Help the user remove banco data that the configuration no longer backs, while
making sure nothing valuable is lost. **The user always has the last word: never
delete anything without explicit confirmation, and always surface what would be
lost first.**

## How it works

Run `banco tidy --pretty` to get a JSON report of stale data. It is
detection-only — it never deletes. The report has three arrays:

- `repos` — synced repository directories no longer matching the config
- `tasks` — task directories whose issues are no longer synced
- `local` — local notes/bookmarks surfaced for review (only when you pass
  `--module notes` or `--module bookmarks`)

To review a single module, pass `--module <repos|tasks|notes|bookmarks>`.

## Workflow

1. Run `banco tidy --pretty`.
2. If everything is empty, tell the user there is nothing to tidy and stop.
3. Group the findings by type and present them clearly (see below).
4. For each item, state the reason and **what would be lost**.
5. Ask the user what to remove. Default to keeping anything unsafe.
6. Remove only what the user confirms — `rm -rf` the reported `path`, or use the
   appropriate command. Then report what was removed and what was kept.

## Repos — check git safety before suggesting removal

Each repo finding has a `reason` and a `git` block. **Never recommend deleting a
repo whose `git.safe_to_remove` is `false`.** Instead, warn the user about each
non-empty field:

- `uncommitted_changes: true` — modified tracked files not committed
- `untracked_files: N` — N files git isn't tracking
- `unpushed_commits: N` — N commits not on any remote (would be lost)
- `local_only_branches: [...]` — branches with no upstream (may be unique work)
- `stashes: N` — N stash entries

If `git.error` is set, git couldn't be inspected — treat the repo as unsafe and
tell the user why.

`reason` values: `removed_from_config` (dropped from the provider's `projects`
list), `no_longer_matches_pattern` (no longer matched by `projects_pattern`),
`provider_disabled` (`enabled: false`), `provider_removed` (provider gone from
config entirely).

Only when a repo is clean (`safe_to_remove: true`) — or the user explicitly
accepts the loss after being warned — offer to remove its directory.

## Tasks — warn about open work

Each task finding covers a directory and reports `files`, `open`, and `closed`
counts. Highlight `open` issues especially — removing them drops the user's local
copy of issues that may still be active. Task files can also hold local edits
(notes, extra frontmatter) beyond the synced issue, which would be lost.

`reason` values: `sync_disabled` (`sync_issues: false`), `removed_from_config`,
`provider_disabled`, `provider_removed`.

If the user turned syncing off but wants to keep the snapshot, leaving the files
in place is a valid choice — say so.

## Local — review content before retiring a module

When the user wants to stop using notes or bookmarks, run
`banco tidy --module <notes|bookmarks>` and review each item:

- `has_url: true` — a bookmark with a real URL; likely worth keeping
- `body_lines` — how much content the file holds; flag anything substantial
- `modified` — recently edited files are more likely to still matter

Summarize what looks relevant and let the user decide item by item. Never bulk
delete local content.

## Principles

- Detection is automatic; deletion is always manual and confirmed.
- Lead with what would be lost, not with what can be removed.
- When in doubt, keep it and tell the user why you're hesitant.
