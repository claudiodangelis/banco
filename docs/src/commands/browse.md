# browse

Opens a URL from the project in the system browser.

```sh
banco browse
```

Presents a fuzzy-select menu: provider → module → item → page (if the item has more than one
page). The selected URL is opened with the configured browser, the `$BROWSER` environment
variable, `xdg-open`, `open`, or `cmd /c start`, tried in that order.

## Configuring the browser

By default, `banco browse` delegates to the `$BROWSER` environment variable or the OS default
opener. You can override this in `.banco/config.yml`.

### Global (all providers)

```yaml
browse:
  command: firefox
  args: ['-P', 'my-profile']
```

### Per-provider

```yaml
providers:
  - name: gitlab
    enabled: true
    browse:
      command: chromium
      args: ['--profile-directory=Work']
```

The per-provider setting takes precedence over the global one. Both take precedence over
`$BROWSER` and the OS default.

`args` is optional and defaults to an empty list. The URL is always appended as the last argument.

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
