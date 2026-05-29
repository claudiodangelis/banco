# check

Scans the project directory for content that does not belong to any module and reports it.

```sh
banco check
```

Exits with code `0` if no issues are found, or `1` if any are reported — making it suitable for
use in scripts or CI pipelines.

## What is checked

### Extraneous top-level directories

Any directory at the project root that is not owned by a module (`notes/`, `tasks/`,
`bookmarks/`, `repos/`) and is not `misc/` or a hidden directory (e.g. `.banco/`, `.git/`) is
flagged as extraneous.

```
extraneous directory: archive/
```

### Extraneous paths within modules

Each module defines the structure it expects under its directory. Anything on disk that falls
outside those rules is flagged:

| Module      | Flagged as extraneous                                              |
| ----------- | ------------------------------------------------------------------ |
| `notes`     | Non-`.md` files anywhere under `notes/local/`                     |
| `tasks`     | Files directly in `tasks/local/`; subdirectories other than `backlog/`, `doing/`, `done/`; non-`.md` files inside status subdirectories |
| `bookmarks` | Non-`.md` files anywhere under `bookmarks/local/`                 |
| `repos`     | Non-directory entries under `repos/local/`                        |

```
extraneous path: notes/local/scratch.txt
extraneous path: tasks/local/wip/
extraneous path: tasks/local/doing/notes.csv
extraneous path: repos/local/README.md
```

## Unmanaged content

If you need to keep files that are intentionally outside any module, place them in `misc/` —
`banco check` will never flag its contents. See [misc](../misc.md).
