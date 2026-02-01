use clap::{ArgGroup, Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
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
