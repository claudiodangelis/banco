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
detection-only — it never deletes. The report has four arrays:

- `repos` — repository directories no longer matching the config (remote, and
  `repos/local` when the repos module is turned off for the local provider)
- `tasks` — task directories whose issues are no longer synced (remote, and
  `tasks/local` when the tasks module is turned off for the local provider)
- `local` — local notes/bookmarks surfaced for review (only when you pass
  `--module notes` or `--module bookmarks`)
- `modules` — whole module directories (`repos/`, `tasks/`, …) that no enabled
  provider backs anymore

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

`reason` values: `module_disabled` (the module is in the provider's
`disabled_modules`, or — for `local` — the module or whole local provider is
turned off), `removed_from_config`, `provider_disabled`, `provider_removed`.

Local findings (`provider: "local"`) are user-authored, not synced from a remote,
so there is no upstream copy to fall back on — treat them with extra care. Repo
findings still carry the `git` block; tasks still report `open`/`closed` counts.

If the user turned a module off but wants to keep the snapshot, leaving the files
in place is a valid choice — say so.

## Local — review content before retiring a module

When the user wants to stop using notes or bookmarks, run
`banco tidy --module <notes|bookmarks>` and review each item:

- `has_url: true` — a bookmark with a real URL; likely worth keeping
- `body_lines` — how much content the file holds; flag anything substantial
- `modified` — recently edited files are more likely to still matter

Summarize what looks relevant and let the user decide item by item. Never bulk
delete local content.

## Modules — a whole tree may be orphaned

Each `modules` finding means an entire top-level directory (`repos/`, `tasks/`,
`notes/`, `bookmarks/`) is no longer backed by any enabled provider — typically
because the user disabled that module everywhere, or disabled the local provider
that was its only source. `reason` is `no_provider_backs_module`; `entries` is
how many immediate items the directory holds.

Treat this as the headline, not a separate deletion target: the contents are
detailed in the `repos`/`tasks` findings for the same paths. Walk the user
through those (with their git-safety and open-count warnings) first; only offer
to remove the whole `module/` directory once everything inside it has been
accounted for and the user accepts it.

## Also check the config itself

`tidy` only finds stale *data*. A config change that stranded data may also have
left the config malformed — a typo'd key, a renamed parameter, an invalid
`disabled_modules` entry. After tidying (or if the user just edited
`.banco/config.yml`), suggest running `banco check`, which validates the config
and project layout. The two are complementary: `tidy` prunes orphaned data,
`check` flags an invalid config.

## Principles

- Detection is automatic; deletion is always manual and confirmed.
- Lead with what would be lost, not with what can be removed.
- When in doubt, keep it and tell the user why you're hesitant.
