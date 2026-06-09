# Banco Project

Banco is an open-source CLI project management tool that organizes notes, tasks,
bookmarks, and repositories as plain files on the filesystem.

Banco source and documentation: https://github.com/claudiodangelis/banco

---

## Providers

Providers are configured in `.banco/config.yml`. String values may reference
environment variables using `$VAR` or `${VAR}` — banco expands them at runtime.
Add a provider interactively with `banco provider add`, or edit the config file
directly. Add an `alias` field when configuring multiple providers of the same kind.

Every provider entry accepts two optional top-level fields: `enabled` (default
`true`; set `false` to switch a provider off without deleting its config) and
`disabled_modules` (a list of module names to turn off for that provider while
leaving the rest active — e.g. sync GitHub issues but not repositories). This
applies to the built-in `local` provider too.

### GitHub

```yaml
providers:
  - name: github
    # alias: github-work        # optional; required when adding a second github provider
    # disabled_modules: [repos] # optional; modules to turn off (tasks, repos)
    config:
      api_key: $GITHUB_TOKEN    # required
      # host: https://github.mycompany.com  # optional; for GitHub Enterprise
      projects:                 # required — or use projects_pattern (mutually exclusive)
        - myorg/my-project
      # projects_pattern: myorg/.*
```

### GitLab

```yaml
providers:
  - name: gitlab
    # alias: gitlab-work
    # disabled_modules: [repos] # optional; modules to turn off (tasks, repos)
    config:
      api_key: $GITLAB_TOKEN    # required
      # host: https://gitlab.mycompany.com  # optional; for self-hosted instances
      projects:                 # required — or use projects_pattern (mutually exclusive)
        - mygroup/my-project
      # projects_pattern: mygroup/.*
```

### JIRA

Delegates to the `claude` CLI via the Atlassian Rovo MCP server — no API token
required in banco. The `claude` CLI must be installed and authenticated.

```yaml
providers:
  - name: jira
    # alias: jira-sre
    config:
      host: https://yourorg.atlassian.net  # required
      project: ENG                         # required; project key
      agent_backend: claude                # required; only supported value
      # labels:                            # optional; filter issues by label
      #   - SRE
```

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
`-l key=value` for each label. Derive available modules and label values from
`banco context`.

```sh
banco new note -n "My note" -l "label=meetings"
banco new task -n "Fix login bug" -l "status=backlog"
```

**Do not invent label names or values.** Only use what the context reports.

### `banco sync [provider]`

Pulls data from configured remote providers and writes it to the filesystem.

```sh
banco sync                                    # sync all providers
banco sync github                             # sync a specific provider by name or alias
banco sync github --module tasks              # sync only the tasks module
banco sync github --module repos              # sync only the repos module
banco sync github --pattern "myorg/frontend"  # sync only projects matching regex
```

Use `--module` and `--pattern` together to narrow a sync to exactly the data you need.

---

## Directory structure

{DIRECTORY_STRUCTURE}
