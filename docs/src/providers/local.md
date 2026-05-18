# Local

The local provider is built in and enabled by default. It is added to `.banco/config.yml`
automatically on `banco init`. Items are not synchronized with any external service — they are
plain files and directories on your filesystem, managed entirely by you.

## Modules

| Module      | Directory            | Items                               | Parameters                                                                  |
| ----------- | -------------------- | ----------------------------------- | --------------------------------------------------------------------------- |
| `tasks`     | `tasks/local/`       | Markdown files, prefixed `0001 -`   | `status` (enum: `awaiting` / `doing` / `done`)                              |
| `notes`     | `notes/local/`       | Markdown files                      | `label` (string, optional — nested tag, e.g. `meetings/2026`)               |
| `bookmarks` | `bookmarks/local/`   | Markdown files                      | `label` (string, optional — nested tag, e.g. `tools/rust`), `url` (string) |
| `repos`     | `repos/local/`       | Directories, `git init` on create   | —                                                                           |

> **Note:** Gerrit provider is planned and will be available soon.
