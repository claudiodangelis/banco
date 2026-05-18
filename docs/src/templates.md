# Templates

When creating an item, Banco looks for a template file inside `.banco/templates/`. The template
path mirrors the item's storage path, and the lookup is hierarchical — the most specific template
wins, falling back to less specific ones.

```
.banco/
└── templates/
    ├── tasks/
    │   └── local/
    │       └── TEMPLATE.md        ← applies to all local tasks
    └── notes/
        └── local/
            ├── TEMPLATE.md        ← applies to all local notes
            └── meetings/
                └── TEMPLATE.md    ← applies to notes in meetings/ (takes precedence)
```

When creating a note in `notes/local/meetings/`, Banco checks:

1. `.banco/templates/notes/local/meetings/TEMPLATE.md`
2. `.banco/templates/notes/local/TEMPLATE.md`

The first match found is used as the initial content of the new file. If no template is found,
the file is created empty.

Use [`banco template`](commands/template.md) to create or edit templates interactively.
