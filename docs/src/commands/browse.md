# browse

Opens a URL from the project in the system browser.

```sh
banco browse
```

Presents a fuzzy-select menu: provider → module → item → page (if the item has more than one
page). The selected URL is opened via `$BROWSER`, `xdg-open`, `open`, or `cmd /c start`, tried
in that order.

## Browseable items

| Provider | Module      | Pages                                   | URL source                                                        |
| -------- | ----------- | --------------------------------------- | ----------------------------------------------------------------- |
| `local`  | `bookmarks` | default                                 | First line of the bookmark `.md` file                            |
| `github` | `tasks`     | default                                 | Derived from filename: `{host}/{owner}/{repo}/issues/{n}` ¹      |
| `github` | `repos`     | Repository · Pull Requests · Actions    | Derived from the cloned repo's `git remote origin` URL           |
| `gitlab` | `repos`     | Repository · Merge Requests · Pipelines | Derived from the cloned repo's `git remote origin` URL           |
| `gitlab` | `tasks`     | default                                 | Derived from the repo's remote URL: `{repo_url}/-/issues/{iid}` ¹ |

¹ **Virtual items** — the URL is not stored anywhere in the task file; it is reconstructed at
browse time from the filename (issue number and title) and the provider's host or the matching
repo's git remote URL. A GitLab task item is only surfaced if a matching cloned repository is
found under `repos/<provider>/`, since that is where the namespace URL is resolved from.
