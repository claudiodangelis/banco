# Dashboard

Running `banco` with no subcommand opens a full-screen, read-only dashboard for the current
project.

```sh
banco
```

Press `q` or `Ctrl+C` to exit.

## Layout

```
━━━━━━━━━━━━━    Last sync:  2026-01-15 09:30:00 (3d ago)
 ┃  banco  ┃    Providers:  local, github
 ┃ ━━━━━━━ ┃    Version:    0.1.0
┌────────────────────────────────────┐
│ cloudio/personale/projects/myproj  │
└────────────────────────────────────┘

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

The top-left shows the banco logo. The three lines to its right are:

| Field | Source |
|---|---|
| **Last sync** | Most recent timestamp across all `.banco/sync-state/` files, with relative age |
| **Providers** | Enabled providers from `.banco/config.yml`, always starting with `local` |
| **Version** | Banco version |

Below the header, the project path is shown relative to `$HOME` (or as an absolute path if
outside home). The directory part is dimmed; the basename is bright.

### Provider sections

Each provider with at least one non-empty module gets a box. Modules with zero items are
hidden. Each visible module is a column:

- **notes / bookmarks / repos** — header with item count, then up to 5 most recent names
- **tasks** — header with total count, then items grouped by status (`backlog`, `doing`, `done`)
  with up to 5 items per group; task number prefixes are stripped from displayed names

If the terminal is too narrow to show item details usefully (less than 16 characters per
column), the columns collapse to headers-only — counts remain visible.

The focused provider's box is drawn in orange. The focused module's column header is also
orange.

## Keyboard shortcuts

| Key | Action |
|---|---|
| `j` / `k` | Next / previous provider |
| `Tab` | Next module (overflows to first module of next provider) |
| `Shift+Tab` | Previous module (underflows to last module of previous provider) |
| `?` | Toggle shortcuts overlay |
| `Esc` | Close shortcuts overlay |
| `q`, `Ctrl+C` | Quit |

Press `?` at any time to show the shortcuts panel as an overlay over the dashboard.
