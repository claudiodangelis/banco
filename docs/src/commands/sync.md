# sync

Pulls data from configured remote providers and writes it to the local filesystem.

```sh
banco sync              # sync all configured providers
banco sync <name>       # sync a specific provider by name or alias
```

Sync is non-destructive: it never deletes or overwrites existing files.

## Incremental sync

After each successful sync, banco writes a timestamp to `.banco/sync-state/<provider>`. On the
next run, only items updated since that timestamp are fetched — making subsequent syncs
significantly faster on large projects.

The first sync (or any sync where the state file is absent) always fetches everything. The state
file is only written on success, so a failed sync will retry the full window on the next run.

## Tasks (issues)

| Situation                | Action                                                  |
| ------------------------ | ------------------------------------------------------- |
| Issue not yet on disk    | Creates a new file with frontmatter and initial content |
| Issue title changed      | Renames the file; body content is untouched             |
| Status or labels changed | Updates frontmatter block; body content is untouched    |
| Issue unchanged          | Does nothing                                            |

## Repos

| Situation            | Action                         |
| -------------------- | ------------------------------ |
| Repo not yet on disk | Clones via SSH                 |
| Repo already on disk | Runs `git fetch --all --prune` |
