use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "banco", about = "Banco project management tool")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize banco in the current directory
    Init {
        /// Re-run initialization (e.g. after enabling new providers)
        #[arg(long)]
        update: bool,
    },
    /// Create a new item
    New {
        /// Module name (note, task)
        module: String,
        /// Item name
        #[arg(short = 'n', long)]
        name: Option<String>,
        /// Parameter in key=value format (repeatable)
        #[arg(short = 'p', long = "param")]
        params: Vec<String>,
    },
    /// Create or edit a template (interactive)
    Template,
    /// Output a JSON summary of the project state (intended for agents)
    #[command(alias = "ctx")]
    Context,
}
