# tidy

Reports module data that the current configuration no longer backs — repositories dropped from a
provider, task trees whose syncing was turned off, and local items you may want to retire. The
output is JSON, intended for agents (see the [tidy skill](skills.md#bundled-skills)).

```sh
banco tidy [--module <name>] [--pretty]
```

`tidy` is **detection only** — it never deletes anything. It compares what is on disk against
`.banco/config.yml` and prints what no longer belongs, leaving every removal decision to you.

Unlike [`check`](check.md), which flags content that was never part of a module, `tidy` flags
content that *was* synced or created but has since become stale because the configuration changed.

## Options

| Flag                | Description                                                          |
| ------------------- | -------------------------------------------------------------------- |
| `--module <name>`   | Limit the scan to one module: `repos`, `tasks`, `notes`, `bookmarks` |
| `--pretty`          | Pretty-print the JSON output                                         |

Local modules (`notes`, `bookmarks`) are only scanned when requested explicitly with `--module`,
since they have no configuration to compare against — the scan instead surfaces their content for
your review.

## Output

A JSON object with three arrays: `repos`, `tasks`, and `local`.

### repos

Each synced repository directory that no longer matches the configuration, with a `reason` and a
`git` safety summary so nothing is lost on removal.

```json
{
  "provider": "github",
  "name": "old-service",
  "path": "repos/github/old-service",
  "reason": "removed_from_config",
  "git": {
    "uncommitted_changes": false,
    "untracked_files": 2,
    "unpushed_commits": 1,
    "local_only_branches": ["spike"],
    "stashes": 0,
    "safe_to_remove": false
  }
}
```

`reason` is one of:

| Reason                       | Meaning                                                        |
| ---------------------------- | -------------------------------------------------------------- |
| `removed_from_config`        | Dropped from the provider's explicit `projects` list           |
| `no_longer_matches_pattern`  | No longer matched by the provider's `projects_pattern`         |
| `provider_disabled`          | The provider is configured with `enabled: false`              |
| `provider_removed`           | The provider is gone from `.banco/config.yml` entirely         |

`safe_to_remove` is `true` only when the working copy holds nothing that would be lost. When git
cannot be inspected (e.g. not a git repository), a `git.error` field is set and `safe_to_remove`
stays `false`.

### tasks

Each task directory whose issues are no longer synced, with file counts split by status.

```json
{
  "provider": "github",
  "path": "tasks/github",
  "reason": "sync_disabled",
  "files": 12,
  "open": 5,
  "closed": 7
}
```

`reason` is one of `sync_disabled` (the provider's `sync_issues` is `false`),
`removed_from_config`, `provider_disabled`, or `provider_removed`. Per-project detection
(`removed_from_config`) applies to GitHub projects and to GitLab projects configured via an
explicit `projects` list.

### local

When `--module notes` or `--module bookmarks` is passed, each item is listed with hints about
whether it holds content worth keeping.

```json
{
  "module": "bookmarks",
  "path": "bookmarks/local/tools/ripgrep.md",
  "has_url": true,
  "body_lines": 3,
  "modified": "2026-05-14"
}
```

## Removal is yours

`tidy` never deletes. Review the report and remove only what you choose — the bundled
[`tidy` skill](skills.md) drives an agent through exactly this: it presents the findings, warns
about anything that would be lost, and removes only what you confirm.
