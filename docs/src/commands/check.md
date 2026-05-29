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

Module directories follow the layout `{module}/{provider}/{item}`. The check enforces this at
two levels.

**Provider subdirectory** — every direct subdirectory of a module root must match a configured
provider name (or alias). `local` is always valid; any other provider you have added via
`banco provider add` is also valid.

```
extraneous path: repos/banco/    ← "banco" is not a configured provider
```

**Provider contents** — inside each `{module}/local/` directory the expected structure is:

| Module      | Flagged as extraneous                                                                                                                       |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| `notes`     | Non-`.md` files anywhere under `notes/local/`                                                                                              |
| `tasks`     | Files directly in `tasks/local/`; subdirectories other than `backlog/`, `doing/`, `done/`; non-`.md` files inside status subdirectories    |
| `bookmarks` | Non-`.md` files anywhere under `bookmarks/local/`                                                                                          |
| `repos`     | Non-directory entries under `repos/local/`                                                                                                  |

```
extraneous path: notes/local/scratch.txt
extraneous path: tasks/local/wip/
extraneous path: tasks/local/doing/notes.csv
extraneous path: repos/local/README.md
```

## Unmanaged content

If you need to keep files that are intentionally outside any module, place them in `misc/` —
`banco check` will never flag its contents. See [misc](../misc.md).
