use std::path::PathBuf;

use clap::{Parser, Subcommand};
use colored::Colorize;

///
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// The HTML file to parse
    #[arg(long, short)]
    file: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Find words by English translation
    Search { term: String },

    /// Show the definition for the given term
    Define { term: String },
}

fn main() {
    let cli = Cli::parse();
    let dict = anglo_saxon_dict_parser::parse(&cli.file).expect("Couldn't parse HTML");

    match &cli.command {
        Commands::Search { term } => {
            let results = dict.search(&term, None).expect("Couldn't search index");

            for result in results {
                println!("{} - {}", result.word.bold().blue(), result.definition);
            }
        }
        Commands::Define { term } => {
            let results = dict.define(&term).expect("Couldn't define term");

            for result in results {
                println!("{} - {}", result.word.bold().blue(), result.definition);
            }
        }
    }
}
