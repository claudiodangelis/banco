# Repos

Repos are directories, not files. Each repo is a git repository stored under
`repos/<provider>/`.

For the local provider, Banco runs `git init` when a new repo is created. For remote providers
(GitHub, GitLab), repositories are cloned via SSH and kept up to date with `git fetch` on each
`banco sync`.

Repos support the `browse` command — selecting a repo opens its remote URL (repository page,
pull requests, pipelines, etc.) in the system browser.
