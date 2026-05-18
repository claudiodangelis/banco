# Tasks

Tasks are Markdown files with a numeric prefix — e.g. `0042 - Fix login bug.md`. The prefix is
assigned automatically by Banco and increments with each new item.

## Status

Each task has a **status**. For the local provider, status maps to a subdirectory:

| Status     | Directory               |
| ---------- | ----------------------- |
| `awaiting` | `tasks/local/backlog/`  |
| `doing`    | `tasks/local/doing/`    |
| `done`     | `tasks/local/done/`     |

For remote providers (GitHub, GitLab), status is stored in a YAML frontmatter block at the top
of the file:

```markdown
---
status: open
tags:
  - bug
---

# Fix login bug

Description here...
```

`status` is `open` or `closed`. `tags` mirrors the issue's labels on the remote provider. Both
fields are updated automatically on each `banco sync` without touching the rest of the file.

## Parameters

| Parameter | Type                            | Required | Description     |
| --------- | ------------------------------- | -------- | --------------- |
| `status`  | enum: `awaiting`/`doing`/`done` | yes      | Task status     |
