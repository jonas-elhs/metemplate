mod cli;
mod config;
mod generate;
mod list;

use crate::cli::{Cli, Commands};
use crate::config::Config;
use crate::generate::generate;
use crate::list::list;
use anyhow::Result;
use clap::Parser;

fn run() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::parse(&cli)?;

    match cli.command {
        Commands::List { project, no_values } => list(project, no_values, &config),
        Commands::Generate {
            project,
            values,
            random: _, // inferred in generate() by values being None
            template,
        } => generate(project, values, template, &config),
    }
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{:#}", err);

        std::process::exit(1);
    }
}
