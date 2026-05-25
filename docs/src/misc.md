# misc

The `misc/` directory is created by `banco init` at the root of every Banco project.
It is **not managed by Banco** — no command reads from or writes to it.
Its purpose is to give you a free-form space for files and directories that don't belong to
any module (notes, tasks, bookmarks, repos).

Typical uses include scratch files, one-off scripts, reference documents, or any other
content you want to keep co-located with your Banco project without it being picked up by
`context`, `browse`, or `sync`.

```
my-project/
├── .banco/
├── misc/          ← unmanaged; store anything here
├── notes/
├── tasks/
└── ...
```
