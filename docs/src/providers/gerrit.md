# Gerrit

The Gerrit provider syncs repositories from a Gerrit instance into the local filesystem.
Repositories are cloned via SSH and kept up to date with `git fetch`. Gerrit reviews are
code-review changes rather than issues, so this provider is repos-only — it implements no `tasks`
module.

## Configuration

Set interactively via `banco provider add`.

| Parameter          | Required | Description                                                                              |
| ------------------ | -------- | ---------------------------------------------------------------------------------------- |
| `host`             | yes      | Gerrit instance URL (e.g. `https://review.leboncoin.ci`)                                 |
| `username`         | yes      | Gerrit/LDAP username (`firstname.lastname`), used for the SSH clone URL                  |
| `ssh_port`         | no       | Gerrit SSH port (default: `29418`)                                                       |
| `projects`         | yes      | Explicit list of project paths — e.g. `core/infra/my-service`                            |
| `namespaced`       | no       | Clone into the full project path instead of a flat leaf dir (default: `false`)           |

The Gerrit provider implements a single module, `repos`.

### Example configuration

```yaml
providers:
  - name: gerrit
    config:
      host: https://review.leboncoin.ci
      username: firstname.lastname
      projects:
        - core/infra/my-service
        - core/infra/another-service
```

With a namespaced layout:

```yaml
providers:
  - name: gerrit
    config:
      host: https://review.leboncoin.ci
      username: firstname.lastname
      namespaced: true
      projects:
        - core/infra/my-service
```

## Directory structure

Repos are cloned under `repos/<provider>/`. By default the layout is flat — each project is
cloned into a directory named after its leaf segment:

```
repos/
└── gerrit/
    └── my-service/
```

Gerrit projects are commonly organized into deep, slash-separated paths (`core/infra/my-service`),
where different scopes can share a leaf name. Set [`namespaced: true`](#namespaced-layout) to clone
into the full project path instead:

```
repos/
└── gerrit/
    └── core/
        └── infra/
            └── my-service/
```

### Namespaced layout

`namespaced` defaults to `false` for backwards compatibility and to keep paths short. Turn it on
when your projects share leaf names across scopes — otherwise the second clone would collide with
the first. This flag is shared by the GitHub and GitLab providers as well.

## Commit-msg hook

Gerrit rejects a push to `refs/for/*` unless every commit carries a `Change-Id` footer, which is
stamped by Gerrit's standard `commit-msg` hook. After cloning, banco fetches the hook from
`<host>/tools/hooks/commit-msg` and installs it into any repo that lacks one
(`.git/hooks/commit-msg`, mode `0755`). This is:

- **automatic** — no manual `curl`/`scp` step and no `git review -s` needed;
- **idempotent** — a repo that already has the hook is left untouched;
- **non-fatal** — if the hook cannot be fetched (e.g. the host is unreachable), banco warns and
  the sync still succeeds.

## Browse

Repos support the `browse` command. Each repo exposes two pages:

| Page          | URL                                              |
| ------------- | ------------------------------------------------ |
| Repository    | `<host>/admin/repos/<project>`                   |
| Open reviews  | `<host>/q/project:<project>+status:open`         |

Under the flat layout the on-disk directory holds only the leaf name, so the full project path is
recovered from the clone's git remote URL to build these links.

## Sync

After a successful sync, banco stores the timestamp in `.banco/sync-state/<provider>`.

See [`banco sync`](../commands/sync.md) for the full sync flow.
