use clap::{Parser, Subcommand};
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

        /// Override value manually
        #[arg(
            short = 's',
            long = "set",
            value_name = "KEY=VALUE",
            value_parser = parse_key_val
        )]
        value_overrides: Vec<(String, String)>,
    },
}

fn parse_key_val(s: &str) -> Result<(String, String), String> {
    let (key, value) = s.split_once('=').ok_or("expected KEY=VALUE")?;

    if key.is_empty() {
        return Err("Key cannot be empty".into());
    }

    Ok((key.to_string(), value.to_string()))
}
