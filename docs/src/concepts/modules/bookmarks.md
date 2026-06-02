# Bookmarks

Bookmarks are organized in group files stored under `bookmarks/local/`. Each group is a
Markdown file whose name identifies the group (e.g. `default.md`, `tools.md`). A file
contains a plain Markdown list, one bookmark per line:

```markdown
- [Rust documentation](https://doc.rust-lang.org)
- [crates.io](https://crates.io)
```

`banco init` creates `bookmarks/local/default.md` as the starting group. New groups are
created automatically when you pass a `group` name that does not yet exist.

Bookmarks support the `browse` command — selecting a bookmark opens its URL in the system
browser.

## Parameters

| Parameter | Type   | Required | Description                                                          |
| --------- | ------ | -------- | -------------------------------------------------------------------- |
| `group`   | string | no       | Group file to add the bookmark to (default: `default`)               |
| `url`     | string | yes      | The URL to bookmark                                                  |

## Example

```sh
banco new bookmark -n "Rust documentation" -l url=https://doc.rust-lang.org
banco new bookmark -n "crates.io" -l url=https://crates.io -l group=rust
```

The first command appends to `bookmarks/local/default.md`; the second creates (or appends
to) `bookmarks/local/rust.md`.
