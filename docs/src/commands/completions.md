# completions

Prints a shell completion script to stdout. Supported shells: `bash`, `fish`, `zsh`, `elvish`,
`powershell`.

```sh
banco completions bash   # generate bash completions
banco completions zsh    # generate zsh completions
banco completions fish   # generate fish completions
```

To enable completions for your current shell session, source the output directly:

```sh
source <(banco completions bash)
```

To persist completions, write the output to the appropriate location for your shell. For example,
on most Linux systems for bash:

```sh
banco completions bash > ~/.local/share/bash-completion/completions/banco
```
