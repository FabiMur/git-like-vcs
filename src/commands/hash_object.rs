use std::io::Write;
use std::path::{Path, PathBuf};
use std::fs;
use anyhow::Context;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use sha1::{Sha1, Digest};
use hex;

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

pub fn invoke(write: bool, file: PathBuf) -> anyhow::Result<()> {
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
    Ok(())
}
