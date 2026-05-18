# Banco

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
providers (GitHub, GitLab) sync issues as tasks and clone repositories via SSH.

Provider configuration lives in `.banco/config.yml`. String values support `$ENV_VAR` expansion
so tokens are never stored in plain text.

## Providers

| Provider | Modules             | Notes                                      |
| -------- | ------------------- | ------------------------------------------ |
| `local`  | tasks, notes, bookmarks, repos | Built in, no configuration needed |
| `github` | tasks, repos        | Syncs issues and repositories via API + SSH |
| `gitlab` | tasks, repos        | Syncs issues and repositories via API + SSH |

## Documentation

Full documentation is available in [`docs/`](docs/).

```sh
mdbook serve docs
```

---

> The name "Banco" is a tribute to
> [Banco Del Mutuo Soccorso](http://www.progarchives.com/artist.asp?id=36), the greatest Italian
> progressive rock band of all time.
