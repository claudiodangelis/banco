# provider

Manages providers configured for the current project. Provider configuration is stored in
`.banco/config.yml`.

```sh
banco provider add   # add a new provider (interactive)
banco provider list  # list configured providers
```

## provider add

Presents a list of available providers to choose from. After selecting one, you are prompted for
an optional alias — an alias is required if a provider of the same kind is already configured.
Banco then walks through each configuration parameter for the chosen provider and saves the
result to `.banco/config.yml`.

## provider list

Prints the name (or alias) of each configured provider.

## Editing in the dashboard

The [dashboard](dashboard.md) also offers an interactive config editor (press `c`) for adding,
removing, enabling, and reconfiguring providers without editing `.banco/config.yml` by hand.
