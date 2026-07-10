# serve

Serves a read-only web interface for the current project.

```sh
banco serve
```

By default it binds to `127.0.0.1:1985` and opens the page in your browser. The server runs
until you stop it with `Ctrl+C`.

## Options

| Flag | Default | Description |
|---|---|---|
| `--port <PORT>` | `1985` | Port to bind to |
| `--bind <ADDR>` | `127.0.0.1` | Address to bind to |
| `--no-open` | off | Do not open the browser automatically |

```sh
banco serve --bind 0.0.0.0 --port 8080 --no-open
```

## Dashboard

The landing page lists every provider that has at least one non-empty module. Providers and
their modules are rendered as collapsible sections; the collapsed/expanded state is remembered
per section in the browser's `localStorage`.

- **notes / tasks** link to the rendered document.
- **tasks** are grouped by project (when present) and then by status.
- **bookmarks** link out to their stored URL.
- **repos** link to their [git history](#git-history).

## File view

Clicking a note or task opens its rendered Markdown. The renderer supports GFM tables, task
lists, strikethrough, and footnotes. Front matter is surfaced as a status badge and tags above
the document.

The page polls the file's modification time every two seconds and reloads automatically when
the file changes on disk, so an open tab stays in sync with edits made elsewhere.

### Go to popup

A **Go to** button in the bottom-right corner (or pressing `g`) opens a popup with:

- **Go to top** / **Go to bottom** — smooth-scroll shortcuts.
- **Table of contents** — built from the document's `#` through `######` headings, indented by
  level. Clicking an entry jumps to that heading.

Press `Escape`, click outside the popup, or press `g` again to close it.

## Git history

Clicking a repo opens a read-only history view listing its most recent commits, each showing
the subject, abbreviated hash, author, and date. At most 250 commits are shown; a notice appears
when the history is truncated.

## Search

The **Search →** link (or pressing `/` from any page) opens a full-text search over `notes` and
`tasks`, plus repositories matched by name. Document matches are ranked and show a short snippet
around the first hit and link to the file view; repo matches link to the [git history](#git-history).

The search is also available as a JSON endpoint at `/api/search?q=<query>`.

## Keyboard shortcuts

| Key | Where | Action |
|---|---|---|
| `/` | any page | Focus the search box, or open the search page |
| `g` | file view | Toggle the **Go to** popup |
| `Escape` | file view | Close the **Go to** popup |
