use clap::{Parser, Subcommand};
use clap_complete::Shell;

#[derive(Parser)]
#[command(name = "banco", about = "Banco project management tool")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize banco in the current directory
    Init,
    /// Create a new item
    New {
        /// Module name (note, task)
        module: String,
        /// Item name
        #[arg(short = 'n', long)]
        name: Option<String>,
        /// Label in key=value format (repeatable)
        #[arg(short = 'l', long = "label")]
        labels: Vec<String>,
    },
    /// Create or edit a template (interactive)
    Template,
    /// Output a JSON summary of the project state (intended for agents)
    #[command(alias = "ctx")]
    Context {
        /// Pretty-print the JSON output
        #[arg(long)]
        pretty: bool,
    },
    /// Manage providers
    Provider {
        #[command(subcommand)]
        action: ProviderAction,
    },
    /// Sync configured providers
    Sync {
        /// Provider name or alias to sync (syncs all if omitted)
        provider: Option<String>,
    },
    /// Generate shell completion scripts
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },
}

#[derive(Subcommand)]
pub enum ProviderAction {
    /// Add a new provider (interactive)
    Add,
    /// List configured providers
    List,
}
