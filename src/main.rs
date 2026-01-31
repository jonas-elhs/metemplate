mod config;
mod generate;
mod list;

use crate::config::Config;
use crate::generate::generate;
use crate::list::list;
use anyhow::Result;
use clap::{ArgGroup, Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List available projects or available project values
    List {
        /// Project to list the possible values for
        #[arg(short, long, value_name = "PROJECT")]
        project: Option<String>,
    },

    /// Generate template files
    #[command(
        group(
            ArgGroup::new("target")
                .required(true)
                .args(["values", "random"])
        )
    )]
    Generate {
        /// Project to generate the templates from
        project: String,

        /// Values to supply to the templates
        values: Option<String>,

        /// Pick a random values file
        #[arg(short, long)]
        random: bool,

        /// Only generate these templates
        #[arg(short, long, value_name = "TEMPLATES")]
        templates: Option<String>,
    },
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load("/home/jonas/dev/metemplate/config")?;

    match cli.command {
        Commands::List { project } => list(project, &config)?,
        Commands::Generate {
            project,
            values,
            random: _,
            templates,
        } => generate(project, values, templates, &config)?,
    }

    Ok(())
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{:#}", err);

        std::process::exit(1);
    }
}
