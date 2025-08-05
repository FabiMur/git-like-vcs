use anyhow::Context;
use flate2::read::ZlibDecoder;
use std::ffi::CStr;
use std::fmt;
use std::io::prelude::*;
use std::io::BufReader;

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