# Modules

In Banco, items are grouped into **modules**. Each module represents a category of content —
notes, tasks, bookmarks, or repos — and defines how items of that type are stored, named, and
managed on the filesystem.

Modules are provided by [providers](../providers.md). The same module type can be backed by
different providers: for example, you can have tasks from the local provider living alongside
tasks synced from GitLab, each in their own subdirectory.

The directory layout follows the pattern `<module>/<provider>/`:

```
tasks/
├── local/
│   ├── backlog/
│   ├── doing/
│   └── done/
└── gitlab/
    └── my-project/
```

The following modules are available:

- [Notes](notes.md)
- [Tasks](tasks.md)
- [Bookmarks](bookmarks.md)
- [Repos](repos.md)
