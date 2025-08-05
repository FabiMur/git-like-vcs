use anyhow::Context;
use crate::objects::{Kind, Object};

pub fn invoke(pretty_print: bool, object_hash: String) -> anyhow::Result<()> {
    // Ensure the "-p" flag is provided
    anyhow::ensure!(pretty_print, "the -p flag is required to use this command");

    // Read the Git object using the new Object::read method
    let mut object = Object::read(&object_hash)?;
    match object.kind {
        Kind::Blob => {
            // obtain stdout and lockit to avoid race conditions
            let stdout = std::io::stdout();
            let mut stdout = stdout.lock();

            // Copy read data to stdout and ensure the size is coherent
            let n = std::io::copy(&mut object.reader, &mut stdout)
                .context("write .git/objects file to stdout")?;
            anyhow::ensure!(
                n == object.size,
                ".git/object file was not the expected size (expected: {}, actual: {})",
                object.size, n
            );  
        }
        Kind::Tree => {
            println!("Tree");
        }
        Kind::Commit => {
            println!("Commit");
        }
    }
    
    Ok(())
}