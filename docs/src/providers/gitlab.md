# GitLab

The GitLab provider syncs tasks (issues) and repositories from configured GitLab projects into
the local filesystem. Repositories are cloned via SSH and kept up to date with `git fetch`.

## Configuration

Set interactively via `banco provider add`.

| Parameter          | Required | Description                                                    |
| ------------------ | -------- | -------------------------------------------------------------- |
| `api_key`          | yes      | GitLab personal access token                                   |
| `host`             | no       | GitLab instance URL (default: `https://gitlab.com`)            |
| `sync_issues`      | no       | Sync issues as tasks (default: `true`)                         |
| `projects`         | no †     | Explicit list of project paths in `namespace/project` format   |
| `projects_pattern` | no †     | Regex matched against `namespace/project` — e.g. `mygroup/.*` |

† Exactly one of `projects` or `projects_pattern` must be set; they are mutually exclusive.

## Directory structure

Tasks are synced flat under `tasks/<provider>/<project>/`:

```
tasks/
└── gitlab/
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

`status` is `open` or `closed`. `tags` mirrors the issue's labels on GitLab. Both fields are
updated automatically on each `banco sync` without touching the rest of the file.

## Templates

New task files are initialized from the first matching template found under `.banco/templates/tasks/`:
`gitlab/<project>/TEMPLATE.md` → `gitlab/TEMPLATE.md` → `tasks/TEMPLATE.md`.
See [Templates](../templates.md) for details.

## Incremental sync

After a successful sync, banco stores the timestamp in `.banco/sync-state/<provider>` and passes
it as the `updated_after` parameter on subsequent API calls. Only issues updated after that point
are fetched. See [`banco sync`](../commands/sync.md) for details.
