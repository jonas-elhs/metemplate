use clap::{ArgGroup, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(short, long, value_name = "PATH")]
    pub config: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List available projects or available project values
    List {
        /// Project to list available values for
        #[arg(short, long, value_name = "NAME")]
        project: Option<String>,

        /// Do not print available values for each project
        #[arg(short = 'V', long)]
        no_values: bool,
    },

    /// Generate template files
    #[command(
        group(
            ArgGroup::new("values_source")
                .required(true)
                .args(["values", "random"])
        )
    )]
    Generate {
        /// Project to generate the templates from
        project: String,

        /// Values to supply to the templates
        #[arg(short, long, value_name = "NAME")]
        values: Option<String>,

        /// Pick a random values file
        #[arg(short, long)]
        random: bool,

        /// Only generate this template
        #[arg(short, long, value_name = "NAME")]
        template: Option<String>,
    },
}
