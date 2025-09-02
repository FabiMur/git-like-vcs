use anyhow::Context;

pub fn invoke(url: String, dir: String) -> anyhow::Result<()> {
    let repo = git2::Repository::clone(&url, &dir)
        .with_context(|| format!("failed cloning {} into {}", url, dir))?;
    println!("Cloned {} to {}", url, repo.path().display());
    Ok(())
}