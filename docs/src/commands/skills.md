# skills

Installs agent skills into the current project. Skills are bundled into the banco binary and
written out on install.

```sh
banco skills install <agent>  # install all bundled skills for an agent
banco skills list             # list installed and available skills
```

## skills install

Installs every bundled skill for the given agent. The agent argument is required and determines
where the skills are written:

| Agent    | Skills directory  |
| -------- | ----------------- |
| `claude` | `.claude/skills/` |

```sh
banco skills install claude
```

Each skill is written to `<skills-dir>/<skill-name>/SKILL.md`. For `claude`, this is the location
Claude Code discovers project skills from, so installed skills become invokable immediately.

The command is idempotent: re-running it overwrites the `SKILL.md` files so they stay up to date
with the bundled version. Passing an unknown agent fails with the list of supported agents.

## skills list

Prints the skills currently installed for `claude` and the bundled skills still available to
install.

## Bundled skills

| Skill         | Description                                                                                                            |
| ------------- | ---------------------------------------------------------------------------------------------------------------------- |
| `hello-world` | A minimal example skill that greets the user; useful for verifying that installed skills are discovered and invokable. |
| `tidy`        | Finds and removes module data the configuration no longer backs, warning about anything that would be lost first. Drives [`banco tidy`](tidy.md). |
