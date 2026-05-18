# Notes

Notes are plain Markdown files. They support an optional **label** parameter, which acts as a
nested tag and maps directly to a subdirectory path.

For example, creating a note with label `meetings/2026` places the file under
`notes/<provider>/meetings/2026/`.

| Parameter | Type   | Required | Description                                                |
| --------- | ------ | -------- | ---------------------------------------------------------- |
| `label`   | string | no       | Nested path used as a tag (e.g. `meetings/2026`)           |

Notes have no special frontmatter — the file content is freeform Markdown.
