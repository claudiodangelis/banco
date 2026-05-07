# Banco

Banco Management System, or simply **Banco**, is an opinionated project management tool for the command line that helps you **organize notes, tasks, bookmarks and documents** for your projects.

Banco objects _(notes, tasks, bookmarks, etc)_ are stored in the **filesystem**, implemented as plain text files and folders within the root of the project, so you won't need to install or run any database or server. This enables you to easily create archives and backups, move projects around the filesystem, use command line tools, or keep track of changes by using version control.

The name "Banco" is a tribute to [Banco Del Mutuo Soccorso](http://www.progarchives.com/artist.asp?id=36), the greatest Italian progressive rock band of all time.

## Agents

Banco supports agentic workflows. Upon initialization, three files are created for agent context:

- `.banco/BANCO.md` — banco-managed file explaining the project structure and available commands; managed by banco
- `AGENTS.md` — reads `.banco/BANCO.md` and is meant for user-defined instructions; never overwritten by banco
- `CLAUDE.md` — reads `AGENTS.md`; never overwritten by banco

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
The local provider is built in and enabled by default.
You can use aliases when using the same provider multiple times.

Provider configuration is stored in `.banco/config.yml` within the project directory. Each provider entry supports a common `enabled` field (default: `true`) — set it to `false` to disable a provider without removing its configuration. String values in the config support environment variable expansion: use `$VAR` or `${VAR}` to avoid storing tokens in plain text (e.g. `api_key: $GITLAB_TOKEN`). Here is how a project managed by Banco looks on the filesystem:

```
├── .banco
│   └── config.yml              ← provider configuration
├── notes
│   └── local
│       ├── meetings
│       │   ├── 20260101 Kickstart meeting.md
│       │   └── 20260102 Client call.md
│       └── project-requirements.md
├── repos
│   ├── gitlab
│   │   └── my-project          ← cloned via SSH
│   └── local
│       ├── poc
│       └── mvp
└── tasks
    ├── gitlab
    │   └── my-project
    │       ├── 1-open
    │       │   └── 0042 - Fix login bug.md
    │       └── 2-closed
    └── local
        ├── backlog
        │   └── 0003 - Write full specs.md
        ├── doing
        └── done
            ├── 0001 - Schedule kickstart meeting.md
            └── 0002 - Write project requirements.md
```

# Providers

## local

The local provider is enabled by default and added to `.banco/config.yml` automatically on `banco init`. Items are not synchronized with any external service — they are plain files and directories on your filesystem, managed entirely by you.

| Module    | Directory          | Items                             | Parameters                                                                 |
| --------- | ------------------ | --------------------------------- | -------------------------------------------------------------------------- |
| tasks     | `tasks/local/`     | Markdown files, prefixed `0001 -` | `status` (enum: `awaiting` / `doing` / `done`)                             |
| notes     | `notes/local/`     | Markdown files                    | `label` (string, optional — nested tag, e.g. `meetings/2026`)              |
| bookmarks | `bookmarks/local/` | Markdown files                    | `label` (string, optional — nested tag, e.g. `tools/rust`), `url` (string) |
| repos     | `repos/local/`     | Directories, `git init` on create | —                                                                          |

> **Note:** Gerrit provider is planned and will be available soon.

## gitlab

The GitLab provider syncs tasks (issues) and repositories from configured GitLab projects into
the local filesystem. Issues are organized by open/closed state; repositories are cloned via SSH
and kept up to date with `git fetch`.

**Configuration parameters** (set interactively via `banco provider add`):

| Parameter          | Required | Description                                                    |
| ------------------ | -------- | -------------------------------------------------------------- |
| `api_key`          | yes      | GitLab personal access token                                   |
| `host`             | no       | GitLab instance URL (default: `https://gitlab.com`)            |
| `sync_issues`      | no       | Sync issues as tasks (default: `true`)                         |
| `projects`         | no†      | Explicit list of project paths in `namespace/project` format   |
| `projects_pattern` | no†      | Regex matched against `namespace/project` — e.g. `mygroup/.*` |

† Exactly one of `projects` or `projects_pattern` must be set; they are mutually exclusive.

**Directory structure:**

Tasks are organized under `tasks/<provider>/`:

```
tasks/
└── gitlab/
    └── my-project/
        ├── 1-open/
        │   └── 0042 - Fix login bug.md
        └── 2-closed/
```

Each task file contains the issue title and description:

```markdown
# Fix login bug

Description here...
```

Repos from configured projects are cloned under `repos/<provider>/`.

## github

The GitHub provider syncs tasks (issues) and repositories from configured GitHub projects into
the local filesystem. Issues are organized by open/closed state; repositories are cloned via SSH
and kept up to date with `git fetch`. Pull requests are excluded from tasks.

**Configuration parameters** (set interactively via `banco provider add`):

| Parameter          | Required | Description                                                                            |
| ------------------ | -------- | -------------------------------------------------------------------------------------- |
| `api_key`          | yes      | GitHub personal access token                                                           |
| `host`             | no       | GitHub instance URL (default: `https://github.com`) — set for GitHub Enterprise Server |
| `sync_issues`      | no       | Sync issues as tasks (default: `true`)                                                 |
| `projects`         | no †     | Explicit list of project paths in `owner/repo` format                                  |
| `projects_pattern` | no †     | Regex matched against `owner/repo` — e.g. `myorg/.*`                                   |

† Exactly one of `projects` or `projects_pattern` must be set; they are mutually exclusive.

**Directory structure:**

Tasks are organized under `tasks/<provider>/`:

```
tasks/
└── github/
    └── myorg/
        └── my-project/
            ├── 1-open/
            │   └── 0042 - Fix login bug.md
            └── 2-closed/
```

Repos are cloned under `repos/<provider>/`.

# Commands

Banco supports the following commands:

- init
- new
- template
- context
- provider
- sync
- completions

## init

Run `banco init` in an empty directory to set up a new banco project. Banco creates the directory skeleton for each module (e.g. `tasks/local/backlog`, `tasks/local/doing`, `tasks/local/done`) and generates `.banco/BANCO.md`, `AGENTS.md`, and `CLAUDE.md` for agentic workflows.

## new

If a module has the "new" capability, you can use the command line to create a new item:

```sh
banco new note -l 'label=some/nested/path' -n 'My note'
```

Pass `-n` for the item name and `-l key=value` for each label. Run without flags to use the interactive TUI, which prompts for all required fields and offers to open the new item in `$EDITOR` when done.

When passing a value for an `enum` label via `-l`, the value must be one of the allowed strings defined by the module. Passing an invalid value will cause the command to fail with an error.

## template

Creates or edits a template interactively.

```sh
banco template
```

Banco reads the current module structure and presents a selection of available paths (e.g. `notes/local`, `notes/local/meetings`, `tasks/local`). After selecting a path, banco creates `.banco/templates/<path>/TEMPLATE.md` if it does not already exist, then opens it in `$EDITOR`. Save and close the editor to finish. The template will be used as the initial content when creating new items under that path.

## context

Outputs a minified JSON summary of the project state to stdout. Intended primarily for agents — run it to give an AI assistant full context about the project contents.

```sh
banco context           # or: banco ctx
banco context --pretty  # pretty-print the JSON output
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

## provider

Manages providers configured for the current project. Provider configuration is stored in `.banco/config.yml`.

```sh
banco provider add   # add a new provider (interactive)
banco provider list  # list configured providers
```

### provider add

Presents a list of available providers to choose from. After selecting one, you are prompted for an optional alias — an alias is required if a provider of the same kind is already configured. Banco then walks through each configuration parameter for the chosen provider and saves the result to `.banco/config.yml`.

### provider list

Prints the name (or alias) of each configured provider.

## sync

Pulls data from configured remote providers and writes it to the local filesystem.

```sh
banco sync              # sync all configured providers
banco sync <name>       # sync a specific provider by name or alias
```

Sync is non-destructive: it never deletes or overwrites existing files.

**Tasks (issues):**

| Situation                         | Action                                                           |
| --------------------------------- | ---------------------------------------------------------------- |
| Issue not yet on disk             | Creates a new file with initial content                          |
| Issue title changed               | Renames the file; content is untouched                           |
| Issue moved to a different column | Moves the file to the new column directory; content is untouched |
| Issue unchanged                   | Does nothing                                                     |

**Repos:**

| Situation            | Action                         |
| -------------------- | ------------------------------ |
| Repo not yet on disk | Clones via SSH                 |
| Repo already on disk | Runs `git fetch --all --prune` |

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

## completions

Prints a shell completion script to stdout. Supported shells: `bash`, `fish`, `zsh`, `elvish`, `powershell`.

```sh
banco completions bash   # generate bash completions
banco completions zsh    # generate zsh completions
banco completions fish   # generate fish completions
```

To enable completions for your current shell session, source the output directly:

```sh
source <(banco completions bash)
```

To persist completions, write the output to the appropriate location for your shell. For example, on most Linux systems for bash:

```sh
banco completions bash > ~/.local/share/bash-completion/completions/banco
```

# Author

Banco is built by [Claudio d'Angelis](https://github.com/claudiodangelis). The structure and conventions behind it reflect the way Claudio has been organizing his own notes, tasks, and bookmarks across projects over the last few years. Banco is an attempt to codify that workflow into a tool others can use too.
