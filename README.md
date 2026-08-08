<img src="logo.svg" alt="banco" width="200"/>

[![CI](https://github.com/claudiodangelis/banco/actions/workflows/ci.yml/badge.svg)](https://github.com/claudiodangelis/banco/actions/workflows/ci.yml)
[![license](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

Banco Management System, or simply **Banco**, is an opinionated project management tool for the
command line that helps you **organize notes, tasks, bookmarks and documents** for your projects.

Everything is stored as plain text files and folders — no database, no server. This makes it
easy to back up, move around, use with standard CLI tools, and keep under version control.

## Installation

Requires [Rust](https://rustup.rs).

```sh
cargo install --path .
```

## Concepts

Items (notes, tasks, bookmarks, repos) are grouped into **modules** and backed by **providers**.
The built-in `local` provider stores everything on disk with no external dependencies. Remote
providers (GitHub, GitLab) sync issues as tasks and clone repositories via SSH. The JIRA provider
fetches issues via the `claude` CLI and the Atlassian Rovo MCP server — no API token required.

Provider configuration lives in `.banco/config.yml`. String values support `$ENV_VAR` expansion
so tokens are never stored in plain text.

## Providers

| Provider | Modules             | Notes                                                        |
| -------- | ------------------- | ------------------------------------------------------------ |
| `local`  | tasks, notes, bookmarks, repos | Built in, no configuration needed               |
| `github` | tasks, repos        | Syncs issues and repositories via API + SSH                  |
| `gitlab` | tasks, repos        | Syncs issues and repositories via API + SSH                  |
| `jira`   | tasks               | Fetches issues via `claude` CLI + Atlassian Rovo MCP server  |

## AI agents

Banco is designed to be driven by an AI agent as naturally as by a human at the keyboard.

- `banco context` emits a single JSON snapshot — providers, modules, items, and the exact labels/enum values each module accepts — so an agent always grounds its next command in real project state instead of guessing.
- `banco init` scaffolds `.banco/BANCO.md` with agent-facing rules (always read context first, create items only through `banco new`, never touch files directly) and wires it into `AGENTS.md`/`CLAUDE.md`, so any compatible coding agent picks up the same playbook automatically.
- `banco skills install claude` drops ready-made Claude Code skills (like `tidy`, which finds orphaned data and asks before deleting anything) straight into `.claude/skills/`, turning banco-specific workflows into commands your agent can invoke on its own.

## Documentation

Full documentation is available in [`docs/`](docs/).

```sh
mdbook serve docs
```

---

> The name "Banco" is a tribute to
> [Banco Del Mutuo Soccorso](http://www.progarchives.com/artist.asp?id=36), the greatest Italian
> progressive rock band of all time.
