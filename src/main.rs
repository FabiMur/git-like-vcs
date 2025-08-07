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
    LsTree {
        #[clap(short = 'n')]
        name_only: bool,
        tree_hash: String,
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
        Command::LsTree { name_only, tree_hash } => {
            commands::ls_tree(name_only, &tree_hash)?;
        }
    }

    Ok(())
}

