# init

Run `banco init` in an empty directory to set up a new Banco project.

Banco creates:

- The directory skeleton for each module (e.g. `tasks/local/backlog`, `tasks/local/doing`,
  `tasks/local/done`)
- A `misc/` directory for files not managed by banco
- `.banco/BANCO.md`, `AGENTS.md`, and `CLAUDE.md` for agentic workflows

```sh
banco init
```

## Flags

### `--update`

Overwrites `.banco/BANCO.md` in an existing project with the version built into
the installed binary. All other project files are left untouched.

Useful after upgrading banco to pick up improvements to the agent instructions.

```sh
banco init --update
```
