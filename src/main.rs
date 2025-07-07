use anyhow::Context;
use clap::{Parser, Subcommand};
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use hex;
use sha1::{Digest, Sha1};
use std::ffi::CStr;
use std::fs;
use std::io::BufReader;
use std::io::prelude::*;
use std::path::Path;
use std::path::PathBuf;

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
enum Kind {
    Blob,
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

            // Read the Git object header, until the first null byte (\0)
            let mut buf = Vec::new();
            z.read_until(0, &mut buf)
                .context("read header from .git/objects")?;

            // Convert header from bytes to valid UTF-8 string
            let header = CStr::from_bytes_with_nul(&buf)
                .expect("know there is exactly one null byte, and it's at the end");
            let header = header
                .to_str()
                .context(".git/objects file header isn't valid UTF-8")?;

            // Extract the type and size of the object
            let Some((kind, size)) = header.split_once(' ') else {
                anyhow::bail!(
                    ".git/objects file header did not start with a known type: '{header}'"
                );
            };

            // Convert object type string into an enum variant
            let kind = match kind {
                "blob" => Kind::Blob,
                _ => anyhow::bail!("print for {kind} not implemented yet"),
            };

            // Convert size string into number
            let size = size
                .parse::<u64>()
                .context(".git/objects file header has invalid size: {size}")?;

            // Read exactly de number of bytes indicated by size
            let mut z = z.take(size);
            match kind {
                Kind::Blob => {
                    // obtain stdout and lockit to avoid race conditions
                    let stdout = std::io::stdout();
                    let mut stdout = stdout.lock();

                    // Copy read data to stdout and ensure the size is coherent
                    let n = std::io::copy(&mut z, &mut stdout)
                        .context("write .git/objects file to stdout")?;
                    anyhow::ensure!(
                        n == size,
                        ".git/object file was not the expected size (expected: {size}, actual: {n})"
                    );
                }
            }
        }
        Command::HashObject { write, file } => {
            // Writes a blob in the Git format and returns its SHA-1 hash
            fn write_blob<W>(file: &Path, writer: W) -> anyhow::Result<String>
            where
                W: Write,
            {
                // Obtain the file metadata
                let stat =
                    std::fs::metadata(&file).with_context(|| format!("stat {}", file.display()))?;

                // Create a Zlib compressor to write in Git format
                let writer = ZlibEncoder::new(writer, Compression::default());

                // Wrap the writer
                let mut writer = HashWriter {
                    writer,
                    hasher: Sha1::new(),
                };

                // Write the blob header: "blob <size>\0"
                write!(writer, "blob {}\0", stat.len())?;

                // Open the file and copy its contents to the writer
                let mut file = std::fs::File::open(&file)
                    .with_context(|| format!("open {}", file.display()))?;
                std::io::copy(&mut file, &mut writer).context("stream file into blob")?;

                // Finish the writing and obtain the SHA-1 hash
                let _ = writer.writer.finish()?;
                let hash = writer.hasher.finalize();
                Ok(hex::encode(hash))
            }

            // Determine if the object should be written to the file system based on the write parameter
            let hash = if write {
                let tmp = "temporary";

                // Write blob to a temporary file and obtain its hash
                let hash = write_blob(
                    &file,
                    std::fs::File::create(tmp).context("construct temporary file for blob")?,
                )
                .context("write out blob object")?;

                // Create the appropriate directory for the blob object based on its hash
                fs::create_dir_all(format!(".git/objects/{}/", &hash[..2]))
                    .context("create subdir of .git/objects")?;

                // Move the temporary file to its final location
                std::fs::rename(tmp, format!(".git/objects/{}/{}", &hash[..2], &hash[2..]))
                    .context("move blob file into .git/objects")?;
                hash
            } else {
                // If there's no writing going to be done just calculate the hash without saving the file
                write_blob(&file, std::io::sink()).context("write blob object")?
            };

            // Print the object's hash
            println!("{}", hash);
        }
    }

    Ok(())
}

// Struct to write data and compute its hash
struct HashWriter<W> {
    writer: W,
    hasher: Sha1,
}

// Implement writing for the HashWriter
impl<W> Write for HashWriter<W>
where
    W: Write,
{
    // Write data in 'writer' and update the hash
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = self.writer.write(buf)?;
        self.hasher.update(&buf[..n]);
        Ok(n)
    }

    // Force the writing of pending data
    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}