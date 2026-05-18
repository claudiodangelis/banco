# Introduction

Banco Management System, or simply **Banco**, is an opinionated project management tool for
the command line that helps you **organize notes, tasks, bookmarks and documents** for your
projects.

Banco objects _(notes, tasks, bookmarks, etc.)_ are stored in the **filesystem**, implemented
as plain text files and folders within the root of the project, so you won't need to install or
run any database or server. This enables you to easily create archives and backups, move projects
around the filesystem, use command line tools, or keep track of changes by using version control.

Here is how a project managed by Banco looks on the filesystem:

```
├── .banco
│   └── config.yml              ← provider configuration
├── misc                        ← not managed by banco; store anything here
├── notes
│   └── local
│       ├── meetings
│       │   ├── 20260101 Kickstart meeting.md
│       │   └── 20260102 Client call.md
│       └── project-requirements.md
├── repos
│   ├── gitlab
│   │   └── my-project          ← cloned via SSH
│   └── local
│       ├── poc
│       └── mvp
└── tasks
    ├── gitlab
    │   └── my-project
    │       └── 0042 - Fix login bug.md
    └── local
        ├── backlog
        │   └── 0003 - Write full specs.md
        ├── doing
        └── done
            ├── 0001 - Schedule kickstart meeting.md
            └── 0002 - Write project requirements.md
```

> The name "Banco" is a tribute to
> [Banco Del Mutuo Soccorso](http://www.progarchives.com/artist.asp?id=36), the greatest Italian
> progressive rock band of all time.
