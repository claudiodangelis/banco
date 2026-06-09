# Providers

A **provider** is a source of items. Providers implement one or more modules and are responsible
for creating, reading, and (for remote providers) synchronizing items with an external service.

## Configuration

Provider configuration is stored in `.banco/config.yml` within the project directory. The local
provider is added automatically on `banco init`; additional providers are added interactively via
`banco provider add`.

Each provider entry supports a common `enabled` field (default: `true`) — set it to `false` to
disable a provider without removing its configuration.

It also supports an optional `disabled_modules` field: a list of module names to turn off for
that provider while leaving its other modules active. Omitted (or empty), every module the
provider implements is on. For example, a GitHub provider with `disabled_modules: [repos]` syncs
issues as tasks but does not clone repositories; disabled modules are also omitted from
`banco context` and `banco browse`. This applies to the built-in `local` provider too — disabling
a local module additionally makes `banco new <module>` refuse to create items in it. Existing
files on disk for a disabled module are left in place (and `banco check` will not flag them as
extraneous), so you can re-enable the module later without losing data.

## Environment variable expansion

String values in the config support environment variable expansion: use `$VAR` or `${VAR}` to
avoid storing tokens in plain text. For example:

```yaml
api_key: $GITLAB_TOKEN
```

## Aliases

You can use aliases when configuring the same provider type more than once — for example, two
separate GitLab instances. An alias is required if a provider of the same kind is already
configured; Banco will prompt for one interactively during `banco provider add`.

The alias is used in directory paths and `banco sync <alias>` calls in place of the provider
name.

## Available providers

- [Local](../providers/local.md)
- [GitHub](../providers/github.md)
- [GitLab](../providers/gitlab.md)
- [JIRA](../providers/jira.md)
