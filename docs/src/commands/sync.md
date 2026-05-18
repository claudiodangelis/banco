# sync

Pulls data from configured remote providers and writes it to the local filesystem.

```sh
banco sync              # sync all configured providers
banco sync <name>       # sync a specific provider by name or alias
```

Sync is non-destructive: it never deletes or overwrites existing files.

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
