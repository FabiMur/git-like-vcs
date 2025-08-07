use crate::objects::{Kind, Object};
use anyhow::Context;
use std::{
    ffi::CStr,
    io::{BufRead, Read, Write},
};

pub fn invoke(name_only: bool, tree_hash: &str) -> anyhow::Result<()> {
    // Read the object file that corresponds to the tree hash
    let mut object = Object::read(tree_hash).context("parse out tree object file")?;

    match object.kind {
        Kind::Tree => {
            // Read the tree object file, one entry at a time
            let mut buf = Vec::new();
            let mut hashbuf = [0; 20];
            let stdout = std::io::stdout();
            let mut stdout = stdout.lock();
            loop {
                // Read the next tree entry
                buf.clear();
                let n = object
                    .reader
                    .read_until(0, &mut buf)
                    .context("read next tree object entry")?;
                if n == 0 {
                    break;
                }

                // Read the hash of the tree entry object
                object
                    .reader
                    .read_exact(&mut hashbuf[..])
                    .context("read tree entry object hash")?;

                // Parse the tree entry
                let mode_and_name =
                    CStr::from_bytes_with_nul(&buf).context("invalid tree entry")?;

                let mut bits = mode_and_name.to_bytes().splitn(2, |&b| b == b' ');
                let mode = bits.next().expect("split always yields once");
                let name = bits
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("tree entry has no file name"))?;

                let name = std::str::from_utf8(name).context("name is always valid utf-8")?;
                let mode = std::str::from_utf8(mode).context("mode is always valid utf-8")?;

                // Print the tree entry
                let hash = hex::encode(&hashbuf);
                if name_only {
                    writeln!(stdout, "{name}")?;
                } else {
                    writeln!(stdout, "{mode:0>6} tree {hash} {name}")?;
                }
            }
        }
        _ => anyhow::bail!("ls-tree is only implemented for trees, not {}", object.kind),
    }

    Ok(())
}