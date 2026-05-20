mod cli;
mod config;
mod context;
mod module;
mod provider;
mod providers;
mod sync_state;
mod template;
mod tui;

use std::collections::HashMap;
use std::path::Path;

use anyhow::Context;
use dialoguer::theme::ColorfulTheme;
use clap::Parser;

use clap::CommandFactory;
use clap_complete::generate;

use cli::{Cli, Commands, ProviderAction};
use dialoguer::FuzzySelect;
use module::BrowseItem;
use context::{ContextOutput, ProviderContext};
use provider::Provider;
use providers::local::LocalProvider;
use providers::github::GitHubProvider;
use providers::gitlab::GitLabProvider;
use providers::jira::JiraProvider;

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
    let banco_md = include_str!("banco_template.md")
        .replace("{DIRECTORY_STRUCTURE}", &descriptions.join("\n\n"));
    std::fs::write(root.join(".banco/BANCO.md"), banco_md)?;
    let agents_path = root.join("AGENTS.md");
    if !agents_path.exists() {
        std::fs::write(&agents_path, "@.banco/BANCO.md\n")?;
    }
    let claude_path = root.join("CLAUDE.md");
    if !claude_path.exists() {
        std::fs::write(&claude_path, "@AGENTS.md\n")?;
    }
    Ok(())
}

fn open_browser(url: &str) -> anyhow::Result<()> {
    if let Ok(browser) = std::env::var("BROWSER") {
        std::process::Command::new(&browser)
            .arg(url)
            .status()
            .with_context(|| format!("failed to launch browser '{}'", browser))?;
        return Ok(());
    }
    #[cfg(target_os = "linux")]
    std::process::Command::new("xdg-open").arg(url).status().context("failed to open browser")?;
    #[cfg(target_os = "macos")]
    std::process::Command::new("open").arg(url).status().context("failed to open browser")?;
    #[cfg(target_os = "windows")]
    std::process::Command::new("cmd").args(["/c", "start", url]).status().context("failed to open browser")?;
    Ok(())
}


const AVAILABLE_PROVIDERS: &[&str] = &["github", "gitlab", "jira"];

fn provider_add(root: &Path) -> anyhow::Result<()> {
    let theme = ColorfulTheme::default();

    let idx = dialoguer::Select::with_theme(&theme)
        .with_prompt("Provider")
        .items(AVAILABLE_PROVIDERS)
        .default(0)
        .interact()?;
    let name = AVAILABLE_PROVIDERS[idx];

    let project_config = config::load(root)?;
    let alias_required = project_config.providers.iter().any(|p| p.name == name);

    let alias_input: String = if alias_required {
        dialoguer::Input::with_theme(&theme)
            .with_prompt("Alias (required: a provider of this kind already exists)")
            .allow_empty(false)
            .interact_text()?
    } else {
        dialoguer::Input::with_theme(&theme)
            .with_prompt("Alias (leave blank if not needed)")
            .allow_empty(true)
            .interact_text()?
    };
    let alias = if alias_input.trim().is_empty() {
        None
    } else {
        Some(alias_input.trim().to_string())
    };

    let schema = match name {
        "github" => GitHubProvider::available_config_schema(),
        "gitlab" => GitLabProvider::available_config_schema(),
        "jira"   => JiraProvider::available_config_schema(),
        _ => vec![],
    };

    let mut cfg_map: std::collections::HashMap<String, serde_yaml::Value> =
        std::collections::HashMap::new();

    for param in &schema {
        let prompt = if param.required {
            format!("{} ({})", param.name, param.description)
        } else {
            format!("{} ({}) [optional]", param.name, param.description)
        };

        match param.kind {
            provider::ConfigParamKind::String => {
                let value: String = dialoguer::Input::with_theme(&theme)
                    .with_prompt(&prompt)
                    .allow_empty(!param.required)
                    .interact_text()?;
                if !value.is_empty() {
                    cfg_map.insert(param.name.to_string(), serde_yaml::Value::String(value));
                }
            }
            provider::ConfigParamKind::Bool => {
                let value = dialoguer::Confirm::with_theme(&theme)
                    .with_prompt(&prompt)
                    .default(true)
                    .interact()?;
                cfg_map.insert(param.name.to_string(), serde_yaml::Value::Bool(value));
            }
            provider::ConfigParamKind::List => {
                println!("{} (enter one per line, empty line to finish):", prompt);
                let mut items = Vec::new();
                loop {
                    let item: String = dialoguer::Input::with_theme(&theme)
                        .with_prompt("  >")
                        .allow_empty(true)
                        .interact_text()?;
                    if item.trim().is_empty() {
                        break;
                    }
                    items.push(serde_yaml::Value::String(item.trim().to_string()));
                }
                if !items.is_empty() {
                    cfg_map.insert(param.name.to_string(), serde_yaml::Value::Sequence(items));
                }
            }
        }
    }

    let entry = config::ProviderEntry {
        name: name.to_string(),
        alias,
        enabled: true,
        config: cfg_map,
    };

    let mut project_config = project_config;
    project_config.providers.push(entry);
    config::save(root, &project_config)?;

    println!("Provider '{}' added.", name);
    Ok(())
}

fn provider_list(root: &Path) -> anyhow::Result<()> {
    let project_config = config::load(root)?;
    if project_config.providers.is_empty() {
        println!("No providers configured.");
        return Ok(());
    }
    for p in &project_config.providers {
        println!("{}", p.display_name());
    }
    Ok(())
}

fn remote_providers(root: &Path) -> anyhow::Result<Vec<Box<dyn Provider>>> {
    let project_config = config::load(root)?;
    let mut providers: Vec<Box<dyn Provider>> = Vec::new();
    for entry in project_config.providers {
        if !entry.enabled {
            continue;
        }
        match entry.name.as_str() {
            "github" => providers.push(Box::new(GitHubProvider::new(entry))),
            "gitlab" => providers.push(Box::new(GitLabProvider::new(entry))),
            "jira"   => providers.push(Box::new(JiraProvider::new(entry))),
            _ => {}
        }
    }
    Ok(providers)
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let root = std::env::current_dir().context("failed to get current directory")?;
    let local = LocalProvider::new();

    match cli.command {
        Commands::Init => {
            let banco_dir = root.join(".banco");
            let is_empty = std::fs::read_dir(&root)?.next().is_none();
            if !is_empty {
                anyhow::bail!("directory is not empty");
            }
            std::fs::create_dir_all(&banco_dir)?;
            config::save(&root, &config::ProjectConfig {
                providers: vec![config::ProviderEntry {
                    name: "local".to_string(),
                    alias: None,
                    enabled: true,
                    config: std::collections::HashMap::new(),
                }],
            })?;
            local.init(&root)?;
            std::fs::create_dir_all(root.join("misc"))?;
            write_agent_files(&root, &local)?;
            println!("Initialized banco in {}", root.display());
        }
        Commands::New { module, name, labels } => {
            let m = local
                .find_module(&module)
                .ok_or_else(|| anyhow::anyhow!("unknown module '{}'; available: note, task", module))?;

            let tui_mode = name.is_none() && labels.is_empty();
            let (item_name, item_params) = if tui_mode {
                tui::prompt(&m.labels())?
            } else {
                let n = name.ok_or_else(|| anyhow::anyhow!("-n/--name is required"))?;
                (n, parse_labels(&labels)?)
            };

            let path = m.create(&root, &item_name, &item_params)?;
            let rel = path.strip_prefix(&root).unwrap_or(&path);
            println!("Created {} '{}': {}", module, item_name, rel.display());

            if tui_mode {
                if let Ok(editor) = std::env::var("EDITOR") {
                    if path.is_file() && tui::confirm_open(&editor)? {
                        std::process::Command::new(&editor)
                            .arg(&path)
                            .status()
                            .with_context(|| format!("failed to launch editor '{}'", editor))?;
                    }
                }
            }
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
        Commands::Context { pretty } => {
            let project = root.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
            let mut providers = vec![ProviderContext {
                name: local.name().to_string(),
                modules: local.context(&root)?,
            }];
            for p in remote_providers(&root)? {
                providers.push(ProviderContext {
                    name: p.name().to_string(),
                    modules: p.context(&root)?,
                });
            }
            let output = ContextOutput { project, providers };
            let json = if pretty {
                serde_json::to_string_pretty(&output)?
            } else {
                serde_json::to_string(&output)?
            };
            println!("{}", json);
        }
        Commands::Sync { provider } => {
            let providers = remote_providers(&root)?;
            if providers.is_empty() {
                anyhow::bail!("no providers configured; run `banco provider add` first");
            }
            let to_sync: Vec<&dyn Provider> = match &provider {
                Some(name) => providers.iter().filter(|p| p.name() == name.as_str()).map(|p| p.as_ref()).collect(),
                None => providers.iter().map(|p| p.as_ref()).collect(),
            };
            if to_sync.is_empty() {
                anyhow::bail!("no provider named '{}'", provider.unwrap());
            }
            sync_state::ensure_dir(&root)?;
            for p in to_sync {
                println!("Syncing {}...", p.name());
                p.sync(&root)?;
            }
        }
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            let bin_name = cmd.get_name().to_string();
            generate(shell, &mut cmd, bin_name, &mut std::io::stdout());
        }
        Commands::Browse => {
            let theme = ColorfulTheme::default();

            let mut all: Vec<(String, Vec<(String, Vec<BrowseItem>)>)> = Vec::new();

            let local_modules = local.browse_modules(&root)?;
            if !local_modules.is_empty() {
                all.push(("local".to_string(), local_modules));
            }
            for p in remote_providers(&root)? {
                let modules = p.browse_modules(&root)?;
                if !modules.is_empty() {
                    all.push((p.name().to_string(), modules));
                }
            }

            if all.is_empty() {
                anyhow::bail!("no browseable items found");
            }

            let provider_idx = if all.len() > 1 {
                let names: Vec<&str> = all.iter().map(|(n, _)| n.as_str()).collect();
                FuzzySelect::with_theme(&theme)
                    .with_prompt("Provider")
                    .items(&names)
                    .default(0)
                    .interact()?
            } else {
                0
            };

            let modules = &all[provider_idx].1;

            let module_idx = if modules.len() > 1 {
                let names: Vec<&str> = modules.iter().map(|(n, _)| n.as_str()).collect();
                FuzzySelect::with_theme(&theme)
                    .with_prompt("Module")
                    .items(&names)
                    .default(0)
                    .interact()?
            } else {
                0
            };

            let items = &modules[module_idx].1;

            let item_prompt = format!("{} item", all[provider_idx].0);
            let item_idx = FuzzySelect::with_theme(&theme)
                .with_prompt(&item_prompt)
                .items(&items.iter().map(|i| i.display.as_str()).collect::<Vec<_>>())
                .default(0)
                .interact()?;

            let item = &items[item_idx];

            let url = if item.pages.len() == 1 {
                &item.pages[0].1
            } else {
                let page_names: Vec<&str> = item.pages.iter().map(|(n, _)| n.as_str()).collect();
                let page_idx = FuzzySelect::with_theme(&theme)
                    .with_prompt("Page")
                    .items(&page_names)
                    .default(0)
                    .interact()?;
                &item.pages[page_idx].1
            };

            open_browser(url)?;
        }
        Commands::Provider { action } => match action {
            ProviderAction::Add => {
                provider_add(&root)?;
            }
            ProviderAction::List => {
                provider_list(&root)?;
            }
        },
    }

    Ok(())
}

fn main() {
    ctrlc::set_handler(|| {
        eprint!("\x1b[?25h");
        std::process::exit(130);
    })
    .expect("failed to set Ctrl-C handler");

    if let Err(e) = run() {
        if let Some(io) = e.downcast_ref::<std::io::Error>() {
            if io.kind() == std::io::ErrorKind::Interrupted {
                eprint!("\x1b[?25h");
                std::process::exit(130);
            }
        }
        eprintln!("Error: {e:#}");
        std::process::exit(1);
    }
}
