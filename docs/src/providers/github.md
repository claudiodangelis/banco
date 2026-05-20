# GitHub

The GitHub provider syncs tasks (issues) and repositories from configured GitHub projects into
the local filesystem. Repositories are cloned via SSH and kept up to date with `git fetch`. Pull
requests are excluded from tasks.

## Configuration

Set interactively via `banco provider add`.

| Parameter          | Required | Description                                                                              |
| ------------------ | -------- | ---------------------------------------------------------------------------------------- |
| `api_key`          | yes      | GitHub personal access token                                                             |
| `host`             | no       | GitHub instance URL (default: `https://github.com`) — set for GitHub Enterprise Server  |
| `sync_issues`      | no       | Sync issues as tasks (default: `true`)                                                   |
| `projects`         | no †     | Explicit list of project paths in `owner/repo` format                                    |
| `projects_pattern` | no †     | Regex matched against `owner/repo` — e.g. `myorg/.*`                                    |

† Exactly one of `projects` or `projects_pattern` must be set; they are mutually exclusive.

## Directory structure

Tasks are synced flat under `tasks/<provider>/<owner>/<repo>/`:

```
tasks/
└── github/
    └── myorg/
        └── my-project/
            └── 0042 - Fix login bug.md
```

Repos are cloned under `repos/<provider>/`.

## Task file format

Each task file carries a YAML frontmatter block followed by the issue title and description:

```markdown
---
status: open
tags:
  - bug
---

# Fix login bug

Description here...
```

`status` is `open` or `closed`. `tags` mirrors the issue's labels on GitHub. Both fields are
updated automatically on each `banco sync` without touching the rest of the file.

## Templates

New task files are initialized from the first matching template found under `.banco/templates/tasks/`:
`github/<owner>/<repo>/TEMPLATE.md` → `github/<owner>/TEMPLATE.md` → `github/TEMPLATE.md` → `tasks/TEMPLATE.md`.
See [Templates](../templates.md) for details.

## Incremental sync

After a successful sync, banco stores the timestamp in `.banco/sync-state/<provider>` and passes
it as the `since` parameter on subsequent API calls. Only issues updated after that point are
fetched. See [`banco sync`](../commands/sync.md) for details.
