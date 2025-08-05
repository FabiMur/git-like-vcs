use clap::{Parser, Subcommand};
use std::path::PathBuf;
mod commands;
mod objects;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Init,
    CatFile {
        #[clap(short = 'p')]
        pretty_print: bool,
        object_hash: String,
    },
    HashObject {
        #[clap(short = 'w')]
        write: bool,
        file: PathBuf,
    },
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Init => {
            commands::init()?;
        }
        Command::CatFile { pretty_print, object_hash } => {
            commands::cat_file(pretty_print, object_hash)?;
        }
        Command::HashObject { write, file } => {
            commands::hash_object(write, file)?;
        }
    }

    Ok(())
}

