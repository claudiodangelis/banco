# Banco

Banco Management System, or simply **Banco**, is an opinionated project management tool for the command line that helps you **organize notes, tasks, bookmarks and documents** for your projects.

Banco objects _(notes, tasks, bookmarks, etc)_ are stored in the **filesystem**, implemented as plain text files and folders within the root of the project, so you won't need to install or run any database or server. This enables you to easily create archives and backups, move projects around the filesystem, use command line tools, or keep track of changes by using version control.

The name "Banco" is a tribute to [Banco Del Mutuo Soccorso](http://www.progarchives.com/artist.asp?id=36), the greatest Italian progressive rock band of all time.

## Agents

Banco supports agentic workflows. Upon initialization, an AGENTS.md file referenced by a CLAUDE.md file is created explaining all the directories.
Agents can make use of the `banco context` command to have an overview of the project state.

Example prompts:

- How many tasks do I have in the backlog?
- Move task "Write full specs" to done
- Create a note about today's meeting in the meetings folder
- Show me all bookmarks tagged under tools/rust
- What tasks are currently in progress?

# Installation

Banco requires [Rust](https://rustup.rs). To install the `banco` binary:

```sh
cargo install --path .
```

# Concepts

In banco, everything is a file and files are organized in structured directories.
Items (notes, tasks, bookmarks, etc.) are grouped in modules and provided by providers.
One default provider exists, called "local".
When running a banco command, if no provider is specified, the local provider is used.

For each module, the default provider can be configured — for example, you can use the JIRA provider as the default provider for tasks.
You can use aliases when using the same provider multiple times.

Here is how a project managed by Banco looks on the filesystem:

```
├── notes
│   └── local
│       ├── meetings
│       │   ├── 20260101 Kickstart meeting.md
│       │   └── 20260102 Client call.md
│       └── project-requirements.md
├── repos
│   ├── github
│   │   ├── frontend
│   │   └── backend
│   └── local
│       ├── poc
│       └── mvp
└── tasks
    ├── jira
    │   ├── MYPROJECT-0001 Implement MVP
    │   │   └── MYPROJECT-0001.md
    │   └── MYPROJECT-0002 Assess network requirements
    │       ├── client-config.md
    │       ├── diagram.graphml
    │       └── MYPROJECT-0002.md
    └── local
        ├── awaiting
        │   └── 0003 - Write full specs.md
        ├── doing
        └── done
            ├── 0001 - Schedule kickstart meeting.md
            └── 0002 - Write project requirements.md
```

## Providers

| Provider | Modules   | Status    |
| -------- | --------- | --------- |
| local    | notes     | available |
|          | tasks     |           |
|          | bookmarks |           |
|          | repos     |           |
| jira     | tasks     | planned   |
| github   | tasks     | planned   |
|          | repos     |           |
| gitlab   | repos     | planned   |
|          | tasks     |           |
| gerrit   | repos     | planned   |

# Providers

## local

Items provided by the local provider are not synchronized with any external service — they are plain files and directories on your filesystem, managed entirely by you.

| Module    | Directory          | Items                             | Parameters                                                                 |
| --------- | ------------------ | --------------------------------- | -------------------------------------------------------------------------- |
| tasks     | `tasks/local/`     | Markdown files, prefixed `0001 -` | `status` (enum: `awaiting` / `doing` / `done`)                             |
| notes     | `notes/local/`     | Markdown files                    | `label` (string, optional — nested tag, e.g. `meetings/2026`)              |
| bookmarks | `bookmarks/local/` | Markdown files                    | `label` (string, optional — nested tag, e.g. `tools/rust`), `url` (string) |
| repos     | `repos/local/`     | Directories, `git init` on create | —                                                                          |

> **Note:** GitHub, GitLab, and Gerrit providers are planned and will be available soon — those may be a better fit for repositories hosted on a remote platform.

# Commands

Banco supports the following commands:

- init
- new
- edit
- template
- context

## init

Run `banco init` in an empty directory to set up a new banco project. Banco creates the directory skeleton for each module (e.g. `tasks/local/backlog`, `tasks/local/doing`, `tasks/local/done`) and generates `CLAUDE.md` and `AGENTS.md` for agentic workflows.

If a new provider is enabled after initialization, run `banco init --update` to update the project structure.

## new

If a module has the "new" capability, you can use the command line to create a new item:

```sh
banco new note -l 'label=some/nested/path' -n 'My note'
```

Pass `-n` for the item name and `-l key=value` for each label. Run without flags to use the interactive TUI, which prompts for all required fields and offers to open the new item in `$EDITOR` when done.

When passing a value for an `enum` label via `-l`, the value must be one of the allowed strings defined by the module. Passing an invalid value will cause the command to fail with an error.

## edit

Opens an existing item in `$EDITOR`. Requires `$EDITOR` to be set.

```sh
banco edit        # or: banco e
banco edit note
banco edit task
```

Without a module argument, banco first prompts you to pick a module. Then it presents a fuzzy-searchable list of all items in that module. Select one with the arrow keys or by typing to filter, and banco opens it in `$EDITOR`.

## template

Creates or edits a template interactively.

```sh
banco template
```

Banco reads the current module structure and presents a selection of available paths (e.g. `notes/local`, `notes/local/meetings`, `tasks/local`). After selecting a path, banco creates `.banco/templates/<path>/TEMPLATE.md` if it does not already exist, then opens it in `$EDITOR`. Save and close the editor to finish. The template will be used as the initial content when creating new items under that path.

## context

Outputs a JSON summary of the project state to stdout. Intended primarily for agents — run it to give an AI assistant full context about the project contents.

```sh
banco context  # or: banco ctx
```

```json
{
    "project": "name of the dir",
    "providers": [
        {
            "name": "local",
            "modules": [
                {
                    "name": "notes",
                    "parameters": [
                        {
                            "name": "label",
                            "type": "string",
                            "description": "Optional nested path used as a tag (e.g. meetings/2026)"
                        }
                    ],
                    "items": [
                        {
                            "name": "My first note",
                            "label": "meetings"
                        }
                    ]
                }
            ]
        }
    ]
}
```

# Templates

When creating an item, banco looks for a template file inside `.banco/templates/`. The template path mirrors the item's storage path, and the lookup is hierarchical — the most specific template wins, falling back to less specific ones.

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

When creating a note in `notes/local/meetings/`, banco checks:

1. `.banco/templates/notes/local/meetings/TEMPLATE.md`
2. `.banco/templates/notes/local/TEMPLATE.md`

The first match found is used as the initial content of the new file. If no template is found, the file is created empty.

Use `banco template` to create or edit templates interactively.

# Author

Banco is built by [Claudio d'Angelis](https://github.com/claudiodangelis). The structure and conventions behind it reflect the way Claudio has been organizing his own notes, tasks, and bookmarks across projects over the last few years. Banco is an attempt to codify that workflow into a tool others can use too.
