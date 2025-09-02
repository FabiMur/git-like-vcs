use anyhow::Context;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use sha1::Sha1;
use std::ffi::CStr;
use std::fmt;
use std::io::{BufRead, BufReader, Write, Read};
use std::fs;
use sha1::Digest;
use std::path::Path;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Blob,
    Tree,
    Commit,
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Kind::Blob => write!(f, "blob"),
            Kind::Tree => write!(f, "tree"),
            Kind::Commit => write!(f, "commit"),
        }
    }
}

pub(crate) struct Object<R> {
    pub(crate) kind: Kind,
    pub(crate) reader: R,
    pub(crate) size: u64,
}

impl Object<()> {
    pub(crate) fn blob_from_file(file: impl AsRef<Path>) -> anyhow::Result<Object<impl Read>> {
        let file = file.as_ref();
        let stat = std::fs::metadata(file).with_context(|| format!("stat {}", file.display()))?;
        // TODO: technically there's a race here if the file changes between stat and write
        let file = std::fs::File::open(file).with_context(|| format!("open {}", file.display()))?;
        Ok(Object {
            kind: Kind::Blob,
            size: stat.len(),
            reader: file,
        })
    }

    pub(crate) fn read(hash: &str) -> anyhow::Result<Object<impl BufRead>> {
        // Build the Git object file path (based on a hash)
        let f = std::fs::File::open(format!(
            ".git/objects/{}/{}",
            &hash[..2],
            &hash[2..]
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
            anyhow::bail!(".git/objects file header did not start with a known type: '{header}'");
        };

        // Convert object type string into an enum variant
        let kind = match kind {
            "blob" => Kind::Blob,
            "tree" => Kind::Tree,
            "commit" => Kind::Commit,
            _ => anyhow::bail!("what even is a '{kind}'"),
        };

        // Convert size string into number
        let size = size
            .parse::<u64>()
            .context(".git/objects file header has invalid size: {size}")?;

        // Take the specified number of bytes from the decompressed file
        let z = z.take(size);

        // Return the object with the kind and the reader
        Ok(Object {
            kind,
            reader: z,
            size,
        })
    }
}

impl<R> Object<R>
where
    R: Read,
{
    pub(crate) fn write(mut self, writer: impl Write) -> anyhow::Result<[u8; 20]> {
        let writer = ZlibEncoder::new(writer, Compression::default());
        let mut writer = HashWriter {
            writer,
            hasher: Sha1::new(),
        };
        write!(writer, "{} {}\0", self.kind, self.size)?;
        std::io::copy(&mut self.reader, &mut writer).context("stream file into blob")?;
        let _ = writer.writer.finish()?;
        let hash = writer.hasher.finalize();
        Ok(hash.into())
    }
    pub(crate) fn write_to_objects(self) -> anyhow::Result<[u8; 20]> {
        let tmp = "temporary";
        let hash = self
            .write(std::fs::File::create(tmp).context("construct temporary file for tree")?)
            .context("stream tree object into tree object file")?;
        let hash_hex = hex::encode(hash);
        fs::create_dir_all(format!(".git/objects/{}/", &hash_hex[..2]))
            .context("create subdir of .git/objects")?;
        fs::rename(
            tmp,
            format!(".git/objects/{}/{}", &hash_hex[..2], &hash_hex[2..]),
        )
        .context("move tree file into .git/objects")?;
        Ok(hash)
    }
}

struct HashWriter<W> {
    writer: W,
    hasher: Sha1,
}
impl<W> Write for HashWriter<W>
where
    W: Write,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = self.writer.write(buf)?;
        self.hasher.update(&buf[..n]);
        Ok(n)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}