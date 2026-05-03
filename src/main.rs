mod cli;
mod context;
mod module;
mod provider;
mod providers;
mod tui;

use std::collections::HashMap;
use std::path::Path;

use anyhow::Context;
use dialoguer::theme::ColorfulTheme;
use clap::Parser;

use cli::{Cli, Commands};
use context::{ContextOutput, ProviderContext};
use provider::Provider;
use providers::local::LocalProvider;

fn parse_labels(raw: &[String]) -> anyhow::Result<HashMap<String, String>> {
    raw.iter()
        .map(|p| {
            p.split_once('=')
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .ok_or_else(|| anyhow::anyhow!("invalid label '{}': expected key=value", p))
        })
        .collect()
}

fn write_agent_files(root: &Path, local: &LocalProvider) -> anyhow::Result<()> {
    let descriptions = local.module_descriptions();
    let agents_md = format!(
        "# Banco Project\n\n\
This is a Banco project. Below is a description of the directory structure.\n\n\
Banco is an open-source project management tool for the command line: https://github.com/claudiodangelis/banco\n\n\
{}\n\n\
# Commands\n\n\
## banco context\n\n\
Run `banco context` to get a JSON summary of the project state (notes, tasks, repos, etc.).\n\n\
## banco new\n\n\
Use `banco new <module>` to create a new item. The `-n` flag sets the name and `-l` sets a label as `key=value`.\n\n\
```sh\n\
# Create a note\n\
banco new note -n \"My note\" -l \"label=meetings\"\n\n\
# Create a task\n\
banco new task -n \"Fix login bug\" -l \"status=backlog\"\n\n\
# Create a bookmark\n\
banco new bookmark -n \"Rust book\" -l \"label=tools/rust\" -l \"url=https://doc.rust-lang.org/book/\"\n\n\
# Create a local repository (initialized as a git repo)\n\
banco new repo -n \"my-project\"\n\
```\n\n\
Available modules and their labels are listed in the `labels` field of `banco context`.\n",
        descriptions.join("\n\n")
    );
    std::fs::write(root.join("AGENTS.md"), agents_md)?;
    std::fs::write(root.join("CLAUDE.md"), "@AGENTS.md\n")?;
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let root = std::env::current_dir().context("failed to get current directory")?;
    let local = LocalProvider::new();

    match cli.command {
        Commands::Init { update } => {
            let banco_dir = root.join(".banco");
            if update {
                if !banco_dir.exists() {
                    anyhow::bail!("not a banco project; run `banco init` first");
                }
            } else {
                let is_empty = std::fs::read_dir(&root)?.next().is_none();
                if !is_empty {
                    anyhow::bail!("directory is not empty; use --update to re-run initialization");
                }
                std::fs::create_dir_all(&banco_dir)?;
            }
            local.init(&root)?;
            write_agent_files(&root, &local)?;
            println!("Initialized banco in {}", root.display());
        }
        Commands::New { module, name, labels } => {
            let m = local
                .find_module(&module)
                .ok_or_else(|| anyhow::anyhow!("unknown module '{}'; available: note, task", module))?;

            let (item_name, item_params) = if name.is_none() && labels.is_empty() {
                tui::prompt(&m.labels())?
            } else {
                let n = name.ok_or_else(|| anyhow::anyhow!("-n/--name is required"))?;
                (n, parse_labels(&labels)?)
            };

            let path = m.create(&root, &item_name, &item_params)?;
            let rel = path.strip_prefix(&root).unwrap_or(&path);
            println!("Created {} '{}': {}", module, item_name, rel.display());
        }
        Commands::Template => {
            let paths = local.all_template_paths(&root);
            if paths.is_empty() {
                anyhow::bail!("no modules found; run `banco init` first");
            }
            let idx = dialoguer::Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Select path")
                .items(&paths)
                .default(0)
                .interact()?;
            let selected = &paths[idx];
            let template_path = root.join(".banco/templates").join(selected).join("TEMPLATE.md");
            std::fs::create_dir_all(template_path.parent().unwrap())?;
            if !template_path.exists() {
                std::fs::write(&template_path, "")?;
            }
            let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
            std::process::Command::new(&editor)
                .arg(&template_path)
                .status()
                .with_context(|| format!("failed to launch editor '{}'", editor))?;
        }
        Commands::Context => {
            let project = root.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
            let output = ContextOutput {
                project,
                providers: vec![ProviderContext {
                    name: local.name().to_string(),
                    modules: local.context(&root)?,
                }],
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
    }

    Ok(())
}
