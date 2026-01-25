mod config;

use crate::config::Config;
use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    List {
        /// Project to list the possible values for
        project: Option<String>,
    },
    Generate {
        /// Project to generate the templates from
        project: String,
        /// Values to supply to the templates
        values: String,
    },
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load("/home/jonas/dev/metemplate/config")?;

    match cli.command {
        Commands::List { project } => {
            println!("Listed '{}'", project.unwrap_or_else(|| "null".into()))
        }
        Commands::Generate { project, values } => {
            println!("Generated project: '{}', values: '{}'", project, values)
        }
    }

    Ok(())
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{:#}", err);

        std::process::exit(1);
    }
}
