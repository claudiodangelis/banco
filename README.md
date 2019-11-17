# Banco

## Note: it's really basic

Banco Management System, or simply Banco, is an opinionated project management tool for the command line.


Banco's features are built as modules, there are modules available for management of:
- documents
- bookmarks
- notes
- tasks

and, soon, _secrets_.

# Install Banco

Install Banco with go

```sh
go get github.com/claudiodangelis/banco
```

# Get started

You can check which modules are available by running
```sh
banco modules
```

## Initialize a new banco project

Create an empty directory, and run
```sh
banco init
```

This command will create folders (and subfolders) for each module.
For example, the `notes` folder will be created by the "notes" module, the `tasks`, `tasks/backlog`, `tasks/doing`, `tasks/done` folders will be created by the "tasks" module.



# Credits

Banco is created by Claudio d'Angelis.

Name "Banco" is a tribute to Banco Del Mutuo Soccorso, the best italian progressive rock band of all times.
