use anyhow::Context;
use clap::{Parser, Subcommand};
use flate2::read::ZlibDecoder;
use std::ffi::CStr;
use std::fs;
use std::io::BufReader;
use std::io::prelude::*;

#[derive(Parser, Debug)]
#[command(version,about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Kind {
    Blob
}

#[derive(Debug, Subcommand)]
enum Command {
    Init,
    CatFile {
        #[clap(short = 'p')]
        pretty_print: bool,
        object_hash: String,
    },
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Init => {
            fs::create_dir(".git").unwrap();
            fs::create_dir(".git/objects").unwrap();
            fs::create_dir(".git/refs").unwrap();
            fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();
            println!("Initialized git directory")
        }
        Command::CatFile {
            pretty_print,
            object_hash,
        } => {
            // Ensure the "-p" flag is provided
            anyhow::ensure!(pretty_print, "the -p flag is required to use this command");

            // Build the Git object file path (based on a hash)
            let f = std::fs::File::open(format!(
                ".git/objects/{}/{}",
                &object_hash[..2],
                &object_hash[2..]
            ))
            .context("open in .git/objects")?;

            // Decompress the Git object
            let z = ZlibDecoder::new(f);
            let mut z = BufReader::new(z);

            // Read the Git object heder, until the first null byte (\0)
            let mut buf = Vec::new();
            z.read_until(0, &mut buf)
                .context("read header from, .git/bojects")?;

            // Convert heder from bytes to valid UTF-8 string
            let header = CStr::from_bytes_with_nul(&buf)
                .expect("know the is exactly one nul, and it's at the end");
            let header = header
                .to_str()
                .context(".git/objects file header isn't valid UTF-8")?;

            // Extract the type and size of the object
            let Some((kind, size)) = header.split_once(' ') else {
                anyhow::bail!(
                    ".git/objects file headder did not start with a known type: '{header}'"
                );
            };

            // Convert object type string into an enum variant
            let kind = match kind {
                "blob" => Kind::Blob,
                _ => anyhow::bail!("print for {kind} not implementd yet"),
            };

            // Convert size string into number
            let size = size
                .parse::<usize>()
                .context(".git/objetcs file header has invalid size: {size}")?;

            // Read the object data into a buffer
            buf.clear();
            buf.resize(size, 0);
            z.read_exact(&mut buf[..])
                .context("read true contents of .git/objects file")?;

            // Valideate that there is no extra data at the end of the file
            let n = z
                .read(&mut [0])
                .context("validate EOF in .git/object file")?;
            anyhow::ensure!(n == 0, ".git/object file had {n} trailing bytes");

            // Print the object content to stdout
            let stdout = std::io::stdout();
            let mut stdout = stdout.lock();
            match kind {
                Kind::Blob => stdout
                    .write_all(&buf)
                    .context("write object contents to stdout")?,
            }
        }
    }

    Ok(())
}