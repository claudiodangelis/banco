# JIRA

The JIRA provider syncs tasks (issues) from a configured JIRA project into the local filesystem.
It does not call the JIRA REST API directly — instead it delegates to the
[`claude` CLI](https://claude.ai/code), which uses the Atlassian Rovo MCP server to fetch issues
on your behalf. No API token is required in Banco.

## Prerequisites

- The `claude` CLI must be installed and available in your `PATH`
- The Atlassian Rovo MCP server must be configured in your `claude` setup
- If using Claude via AWS Bedrock, run `claude` interactively at least once to complete
  authentication before running `banco sync`

## Configuration

Set interactively via `banco provider add`.

| Parameter       | Required | Description                                                                                  |
| --------------- | -------- | -------------------------------------------------------------------------------------------- |
| `host`          | yes      | JIRA instance URL (e.g. `https://yourorg.atlassian.net`) — used to construct browse URLs     |
| `project`       | yes      | Project key to sync (e.g. `ENG`)                                                             |
| `labels`        | no       | List of labels to filter issues by — passed to the agent at fetch time; omit to fetch all    |
| `agent_backend` | yes      | Agent backend to use; only `claude` is supported                                             |

By default, only issues **assigned to the calling user** and **not in Done status** are synced.
There are no config fields for these — they are built-in defaults.

## Directory structure

Tasks are synced flat under `tasks/<provider>/`:

```
tasks/
└── jira/
    ├── ENG-1 - Fix login bug.md
    └── ENG-42 - Improve onboarding flow.md
```

If an alias is configured, the alias is used in place of `jira`.

## Task file format

Each task file carries a YAML frontmatter block with the issue metadata, followed by the file
body (populated from a template if one exists, otherwise empty):

```markdown
---
status: In Progress
type: Task
parent_id: ENG-10
---
```

`parent_id` is omitted for top-level issues. All three fields are updated automatically on each
`banco sync` without touching the rest of the file.

### Issue types

`type` reflects the JIRA issue type name exactly as returned by the Rovo MCP server. Common
values include `Epic`, `Story`, `Task`, `Sub-task`, and `Bug`, but custom issue types are also
supported.

## Browsing

`banco browse` constructs issue URLs as `{host}/browse/{id}` using the `host` config value and
the issue ID parsed from each filename.

## Templates

New task files are initialized from the first matching template found under `.banco/templates/tasks/`:
`jira/TEMPLATE.md` → `tasks/TEMPLATE.md`.
See [Templates](../templates.md) for details.

## Incremental sync

After a successful sync, banco stores the timestamp in `.banco/sync-state/<provider>` and
includes it in the prompt sent to the agent backend, asking for only issues updated since that
point. The first sync always fetches everything. See [`banco sync`](../commands/sync.md) for
details.

## Syncing

```sh
banco sync           # sync all providers
banco sync jira      # sync only the jira provider (or use its alias)
```

If `claude` exits with an error, Banco will display the error output. If authentication is
pending (e.g. first run with Bedrock), run `claude` interactively to complete login, then retry.
