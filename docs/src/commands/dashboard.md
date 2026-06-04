# Dashboard

Running `banco` with no subcommand opens a full-screen, read-only dashboard for the current
project.

```sh
banco
```

Press `q` or `Ctrl+C` to exit.

## Layout

```
━━━━━━━━━━━━━    Status:     2026-01-15 09:30:00 (3d ago)  ·  ok
 ┃  banco  ┃    Providers:  local, github
━━━━━━━━━━━━━

┌ local ──────────────┬──────────────────┬──────────────┐
│ notes (2)           │ tasks (4)        │ bookmarks (1)│
│   My first note     │   backlog (2)    │   Rust docs  │
│   Meeting 2026-01   │     Fix login    │              │
│                     │     Add tests    │              │
│                     │   doing (1)      │              │
│                     │     Review PR    │              │
└─────────────────────┴──────────────────┴──────────────┘
```

### Header

The top row shows:

| Field | Source |
|---|---|
| **Status** | Most recent sync timestamp with relative age, and config check result |
| **Providers** | Enabled providers from `.banco/config.yml`, always starting with `local` |

### Provider sections

Each provider with at least one non-empty module gets a box. Modules with zero items are
hidden. Each visible module is a column:

- **notes / bookmarks / repos** — header with item count, then up to 5 most recent names
- **tasks** — header with total count, then items grouped by status with up to 5 items per
  group; task number prefixes are stripped from displayed names

If the terminal is too narrow to show item details usefully (less than 16 characters per
column), the columns collapse to headers-only — counts remain visible.

The focused provider's box is drawn in orange. The focused module's column header is also
orange.

#### Collapsing providers

Press `v` to collapse the focused provider. A collapsed provider is replaced by a single
summary bar showing its name and per-module item counts, freeing the vertical space for the
expanded providers. Press `v` again on it to expand it back.

```
│ ▸ github  tasks (4) · repos (2)                          │
```

This is a per-project view preference: it is remembered per project and persists across
restarts. It is stored outside the repository in `$XDG_STATE_HOME/banco/state.yml` (falling
back to `~/.local/state/banco/state.yml`), not in `.banco/config.yml`, so it never appears in
diffs and never leaks between users.

## Keyboard shortcuts

| Key | Action |
|---|---|
| `j` / `k` | Next / previous provider |
| `Tab` | Next module (overflows to first module of next provider) |
| `Shift+Tab` | Previous module (underflows to last module of previous provider) |
| `Space` | Browse all items in the focused module |
| `v` | Collapse / expand the focused provider |
| `d` | Open check panel |
| `Ctrl+S` | Sync |
| `?` | Toggle shortcuts overlay |
| `Esc` | Close overlay |
| `q`, `Ctrl+C` | Quit |

Press `?` at any time to show the shortcuts panel as an overlay over the dashboard.

## Item browser

Pressing `Space` on a focused module opens a full-screen list of all items in that module.

```
┌ local: tasks (12 items) ────────────────────────────────┐
│ filter: _                                                │
├──────────────────────────────────────────────────────────┤
│ backlog                                                  │
│   Fix login bug                                          │
│   Add tests                                             │
│ doing                                                    │
│   Review PR                                             │
└──────────────────────────────────────────────────────────┘
  Esc/q close  ↑↓/Tab navigate  type to filter  Enter edit
```

### Filtering

Type any characters to fuzzy-filter items. Label/group headers (e.g. status names) are always
shown for groups that have at least one matching item, and hidden when all their items are
filtered out.

### Navigation

| Key | Action |
|---|---|
| `↑` / `↓` | Previous / next item (wraps around) |
| `Tab` / `Shift+Tab` | Next / previous item (wraps around) |

### Editing

Pressing `Enter` opens the selected item in `$EDITOR` (falls back to `vi`). The dashboard
suspends while the editor runs and resumes when it exits.

Editing is supported for all modules that map to files on disk:

- `local` provider: `notes`, `tasks`
- Remote providers (`jira`, `github`, `gitlab`): `tasks`

## Check panel

Pressing `d` opens a compact overlay summarising the output of [`banco check`](check.md).

```
         ┌ check ──────────────────────────────┐
         │                                     │
         │  2 issues found:                    │
         │                                     │
         │  Extraneous directories             │
         │    ✗  ./archive                     │
         │                                     │
         │  Extraneous module paths            │
         │    ✗  ./notes/local/scratch.txt     │
         │                                     │
         │                         Esc  close  │
         └─────────────────────────────────────┘
```

When there are no issues the panel shows a single green confirmation line:

```
         ┌ check ──────────────┐
         │                     │
         │  ✓  No issues found │
         │                     │
         │           Esc close │
         └─────────────────────┘
```

The panel is sized to fit its content and centered on the terminal. It is drawn as an overlay
on top of the dashboard — the underlying view is not redrawn until the panel is closed.

Press `Esc` or `q` to close.
