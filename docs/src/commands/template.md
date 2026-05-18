# template

Creates or edits a template interactively.

```sh
banco template
```

Banco reads the current module structure and presents a selection of available paths (e.g.
`notes/local`, `notes/local/meetings`, `tasks/local`). After selecting a path, banco creates
`.banco/templates/<path>/TEMPLATE.md` if it does not already exist, then opens it in `$EDITOR`.
Save and close the editor to finish.

The template will be used as the initial content when creating new items under that path. See
[Templates](../templates.md) for details on how template lookup works.
