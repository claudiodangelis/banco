# Templates

When creating an item, Banco looks for a template file inside `.banco/templates/`. The lookup is
hierarchical — the most specific template wins, falling back to progressively less specific ones.

```
.banco/
└── templates/
    ├── tasks/
    │   ├── TEMPLATE.md              ← applies to all tasks from any provider
    │   ├── local/
    │   │   └── TEMPLATE.md          ← applies to local tasks only
    │   ├── github/
    │   │   ├── TEMPLATE.md          ← applies to all GitHub tasks
    │   │   └── myorg/
    │   │       └── TEMPLATE.md      ← applies to tasks from myorg (takes precedence)
    │   ├── gitlab/
    │   │   └── TEMPLATE.md          ← applies to all GitLab tasks
    │   └── jira/
    │       └── TEMPLATE.md          ← applies to all JIRA tasks
    └── notes/
        └── local/
            ├── TEMPLATE.md          ← applies to all local notes
            └── meetings/
                └── TEMPLATE.md      ← applies to notes in meetings/ (takes precedence)
```

## Lookup order

The most specific template found is used. For example, when syncing a GitHub issue from
`myorg/my-repo`, Banco checks:

1. `.banco/templates/tasks/github/myorg/my-repo/TEMPLATE.md`
2. `.banco/templates/tasks/github/myorg/TEMPLATE.md`
3. `.banco/templates/tasks/github/TEMPLATE.md`
4. `.banco/templates/tasks/TEMPLATE.md`

For a JIRA issue:

1. `.banco/templates/tasks/jira/TEMPLATE.md`
2. `.banco/templates/tasks/TEMPLATE.md`

The first match found is used as the initial content of the new file. If no template is found,
the file is created empty. Templates are only applied when a file is first created — subsequent
syncs update frontmatter only and never touch the body.

Use [`banco template`](commands/template.md) to create or edit templates interactively.
