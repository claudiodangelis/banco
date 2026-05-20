# Banco Project

This is a **Banco** project. Banco is an open-source project management tool
for the command line that organizes notes, tasks, bookmarks, and repositories
as plain files on the filesystem.

Banco source and documentation: https://github.com/claudiodangelis/banco

---

## Providers

A **provider** is a source of items. Each provider contributes one or more
modules (tasks, notes, bookmarks, repos) and is responsible for storing and
syncing the data it owns.

The **local** provider is the default provider. It is always present and
enabled — no configuration or network access required. It manages items stored
entirely on the local filesystem: notes, tasks, bookmarks, and repositories.

Additional providers (GitHub, GitLab) can be added via `banco provider add`
and sync remote data (issues, repos) into the local filesystem when
`banco sync` is run.

The active providers and their modules are listed in the `providers` field of
`banco context`.

---

## Rules for agents

### 1. Always start with `banco context`

Before doing anything, run `banco context` to get a complete JSON snapshot of
the project state:

```sh
banco context
```

The output contains:

- **providers** — the list of active providers (local, github, gitlab, …)
- **modules** — each provider's modules (tasks, notes, bookmarks, repos)
- **labels** — the schema of metadata fields available on each module's items
- **items** — the current list of items in each module, each carrying its
  metadata fields

Items synced from remote providers (GitHub, GitLab) include a **metadata**
block with the following fields:

- `status` — current state of the item (`open` or `closed`)
- `tags` — labels attached to the item on the remote

Use these fields to make informed decisions: prioritize open items, filter by
tag, skip closed issues when the task is about active work, etc. The `labels`
array on each module lists every available field and its allowed values —
always read it before filtering or sorting items.

Read the context every time before answering questions about the project state
or before constructing any command. Never assume the state from memory alone.

### 2. Never create or move files directly

All items (notes, tasks, bookmarks, repositories) **must** be created through
`banco new`. Never write files or create directories by hand — banco manages
naming, numbering, and placement automatically. Bypassing banco will corrupt
the project structure.

### 3. Derive every command from the context

The `labels` array in `banco context` output specifies exactly which parameters
each module accepts. For `enum` labels it also lists the allowed values. Always
inspect the context before building a `banco new` command so you pass the
correct label keys and values.

Example context excerpt:

```json
{
  "name": "tasks",
  "labels": [
    {
      "name": "status",
      "type": "enum",
      "values": ["backlog", "doing", "done"],
      "description": "Status of the task"
    }
  ]
}
```

From this you know that `banco new task` accepts `-l status=<value>` where
`<value>` must be one of `backlog`, `doing`, or `done`. Passing an invalid
value causes the command to fail. Always derive valid values from the context
— never guess.

---

## Commands

### `banco context`

Returns a minified JSON summary of the current project state. This is the
authoritative source of truth for the project — use it before every action.

```sh
banco context           # minified JSON (default, preferred for agents)
banco context --pretty  # pretty-printed for human reading
```

### `banco new <module>`

Creates a new item in the given module. Use `-n` for the name and
`-l key=value` for each label. The list of available modules and their
accepted labels is in the `labels` field of `banco context`.

```sh
# Create a note
banco new note -n "My note" -l "label=meetings"

# Create a task
banco new task -n "Fix login bug" -l "status=backlog"

# Create a bookmark
banco new bookmark -n "Rust book" -l "label=tools/rust" -l "url=https://doc.rust-lang.org/book/"

# Create a local repository (initialized as a git repo)
banco new repo -n "my-project"
```

**Do not invent label names or values.** Only use what the context reports.

### `banco sync [provider]`

Pulls data from configured remote providers (GitHub, GitLab) and writes tasks
and repos to the filesystem. Run without arguments to sync all providers.

```sh
banco sync           # sync all providers
banco sync github    # sync a specific provider by name or alias
```

---

## Directory structure

{DIRECTORY_STRUCTURE}
