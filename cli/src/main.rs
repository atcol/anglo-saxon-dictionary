use anyhow;
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;
use tokio::sync::oneshot;
use tokio::time::{interval, Duration};
use url;
use std::io::{self, Write};

/// The command line options
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// The HTML file to parse
    #[arg(long, short)]
    file: Option<PathBuf>,

    /// The HTML file to parse
    #[arg(long, short)]
    url: Option<url::Url>,

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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let (tx, mut rx) = oneshot::channel();
    let mut intv = interval(Duration::from_millis(500));

    tokio::spawn(async move {
        let dict = if let Some(url) = cli.url {
            anglo_saxon_dict_parser::parse_url(url).await.expect("Couldn't parse HTML")
        } else if let Some(file) = cli.file {
            anglo_saxon_dict_parser::parse(&file).expect("Couldn't parse HTML")
        } else {
            todo!()
        };

        let _ = tx.send(dict);
    });

    loop {
        tokio::select! {
            _ = intv.tick() => {
                std::io::stdout().flush().expect("Flushing stdout");
            },
            result = &mut rx => {
                if let Ok(dict) = result {
                    match &cli.command {
                        Commands::Search { term } => {
                            println!("{}: {}", "Search".bold().underline().blue(), term.bold());
                            let results = dict.search(&term, None).expect("Couldn't search index");

                            for result in results {
                                println!("{} - {}", result.word.bold().blue(), result.definition);
                            }
                        }
                        Commands::Define { term } => {
                            println!("{}: {}", "Define".bold().underline().blue(), term.bold());
                            let results = dict.define(&term).expect("Couldn't define term");

                            for result in results {
                                println!("{} - {}", result.word.bold().blue(), result.definition);
                            }
                        }
                    }
                } else {
                    println!("Failed to load dictionary");
                }
                break;
            }
        }
    }
    Ok(())
}
