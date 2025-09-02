use crate::objects::{Kind, Object};
use anyhow::Context;
use std::fs;
use std::io::Cursor;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::cmp::Ordering;


fn write_tree_for(path: &Path) -> anyhow::Result<Option<[u8; 20]>> {
    // Read directory entries and collect metadata early so we can sort and decide behavior.
    let mut dir = fs::read_dir(path).with_context(|| format!("failed to read directory {}", path.display()))?;
    let mut entries = Vec::new();

    while let Some(entry) = dir.next() {
        let entry = entry.with_context(|| format!("failed to read entry in {}", path.display()))?;
        let name = entry.file_name();
        let meta = entry.metadata().with_context(|| format!("failed to read metadata for {}", name.to_string_lossy()))?;
        entries.push((entry, name, meta));
    }

    // Sort entries using Git's tree sort order:
    // - Compare names as raw bytes.
    // - If one name is a prefix of the other, pretend the directory has a trailing '/'.
    // - This ensures 'a' comes before 'a.txt', but 'a/' is ordered relative to 'a' via '/'.
    entries.sort_unstable_by(|a, b| {
        let afn = &a.1;
        let afn = afn.as_encoded_bytes();
        let bfn = &b.1;
        let bfn = bfn.as_encoded_bytes();
        let common_len = std::cmp::min(afn.len(), bfn.len());
        match afn[..common_len].cmp(&bfn[..common_len]) {
            Ordering::Equal => {}
            o => return o,
        }
        if afn.len() == bfn.len() {
            return Ordering::Equal;
        }
        let c1 = if let Some(c) = afn.get(common_len).copied() {
            Some(c)
        } else if a.2.is_dir() {
            Some(b'/')
        } else {
            None
        };
        let c2 = if let Some(c) = bfn.get(common_len).copied() {
            Some(c)
        } else if b.2.is_dir() {
            Some(b'/')
        } else {
            None
        };
        c1.cmp(&c2)
    });

    // Build the raw tree payload following the "tree" object format.
    // For each entry:
    //   "<mode> <name>\0<20-byte raw hash>"
    let mut tree_object = Vec::new();
    for (entry, file_name, meta) in entries {
        // Never include the repository's own .git directory in the tree.
        if file_name == ".git" {
            continue; // Skip the .git directory
        }

        // Compute the Git file mode for the entry.
        // - 40000: directory
        // - 120000: symbolic link
        // - 100755: executable file (any exec bit set)
        // - 100644: regular file
        let mode = if meta.is_dir() {
            "40000"
        } else if meta.is_symlink() {
            "120000"
        } else if (meta.permissions().mode() & 0o111) != 0 {
            // has at least one executable bit set
            "100755"
        } else {
            "100644"
        };

        let path = entry.path();
        // Determine the object hash to reference:
        // - For directories: recursively write a tree (skip if empty).
        // - For files/symlinks: create a blob object if necessary and store it.
        let hash = if meta.is_dir() {
            let Some(hash) = write_tree_for(&path)?  else {
                // If the directory produced no entries, do not include it in this tree.
                continue; // Skip empty directories
            };
            hash
        } else {
            // Create a blob for the file and write it to .git/objects. We stream the file
            // through our Object API into a temporary file and then move it in place.
            //
            // NOTE: This uses a fixed temp file name, which is fine for single-threaded usage
            // but not safe for concurrent writes. A unique temp name would be safer.
            let tmp = "temporary";
            let hash = Object::blob_from_file(&path)
                .context("open blob input file")?
                .write(std::fs::File::create(tmp).context("construct temporary file for blob")?)
                .context("stream file into blob")?;
            let hash_hex = hex::encode(hash);
            fs::create_dir_all(format!(".git/objects/{}/", &hash_hex[..2]))
                .context("create subdir of .git/objects")?;
            std::fs::rename(
                tmp,
                format!(".git/objects/{}/{}", &hash_hex[..2], &hash_hex[2..]),
            )
            .context("move blob file into .git/objects")?;
            hash
        };

        // Append one tree entry: "<mode> <name>\0<hash>"
        tree_object.extend(mode.as_bytes());
        tree_object.push(b' ');
        // Names are written as raw bytes; this matches Git behavior on Unix.
        tree_object.extend(file_name.as_encoded_bytes());
        tree_object.push(0);
        // The hash is the raw 20-byte object id, not hex.
        tree_object.extend(hash);
    }

    // If nothing was added, the directory is empty: propagate None upward.
    if tree_object.is_empty() {
        Ok(None)
    } else {
        // Wrap the payload in a Tree object header and write to .git/objects.
        Ok(Some(
            Object {
                kind: Kind::Tree,
                size: tree_object.len() as u64,
                reader: Cursor::new(tree_object),
            }
            .write_to_objects()
            .context("write tree object")?,
        ))
    }
}

pub fn invoke() -> anyhow::Result<()> {
    // Build a tree for the current working directory.
    let Some(hash) = write_tree_for(Path::new(".")).with_context(|| "failed to write tree")? else {
        anyhow::bail!("no files to write to the tree");
    };

    // Print the tree id in hex, matching `git write-tree` output.
    println!("{}", hex::encode(hash));

    Ok(())
}