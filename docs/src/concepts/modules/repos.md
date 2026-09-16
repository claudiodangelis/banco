# Repos

Repos are directories, not files. Each repo is a git repository stored under
`repos/<provider>/`.

For the local provider, Banco runs `git init` when a new repo is created. For remote providers
(GitHub, GitLab, Gerrit), repositories are cloned via SSH and kept up to date with `git fetch` on
each `banco sync`.

By default repos are cloned flat, into a directory named after the project's last path segment.
Remote providers accept a `namespaced` option (default: `false`) that clones into the full
project path instead — see the [GitHub](../../providers/github.md),
[GitLab](../../providers/gitlab.md), and [Gerrit](../../providers/gerrit.md) provider docs.

Repos support the `browse` command — selecting a repo opens its remote URL (repository page,
pull requests, pipelines, etc.) in the system browser.

## Git status

Each repo item carries a small git status summary, surfaced in the [dashboard](../../commands/dashboard.md)
and in [`banco context`](../../commands/context.md) JSON:

| Field | Meaning |
|---|---|
| `branch` | Current branch, or `null` for a detached `HEAD` / unreadable repo |
| `dirty` | `true` when the working copy has uncommitted or untracked changes |
| `unmerged_branches` | Count of local branches not merged into the current branch |

```json
{
  "name": "my-service",
  "branch": "main",
  "dirty": true,
  "unmerged_branches": 2
}
```

In the dashboard these render compactly as `my-service (main) *↟2` — `*` for a dirty working
copy and `↟N` for unmerged branches.
