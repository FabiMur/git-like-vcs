use crate::objects::{Kind, Object};
use anyhow::Context;
use std::fmt::Write;
use std::io::Cursor;
use chrono::Local;

pub fn invoke(
    message: String,
    tree_hash: String,
    parent_hash: Option<String>,
) -> anyhow::Result<()> {
    let mut commit = String::new();
    writeln!(commit, "tree {tree_hash}")?;
    if let Some(parent_hash) = parent_hash {
        writeln!(commit, "parent {parent_hash}")?;
    }

    let author_name = std::env::var("GIT_AUTHOR_NAME").unwrap_or_else(|_| "Unknown".into());
    let author_email = std::env::var("GIT_AUTHOR_EMAIL").unwrap_or_else(|_| "unknown@example.com".into());
    let committer_name = std::env::var("GIT_COMMITTER_NAME").unwrap_or_else(|_| author_name.clone());
    let committer_email = std::env::var("GIT_COMMITTER_EMAIL").unwrap_or_else(|_| author_email.clone());

    let now = Local::now();
    let timestamp = now.timestamp();
    let offset_secs = now.offset().local_minus_utc();
    let sign = if offset_secs >= 0 { '+' } else { '-' };
    let abs = offset_secs.abs();
    let hours = abs / 3600;
    let minutes = (abs % 3600) / 60;
    let tz = format!("{sign}{hours:02}{minutes:02}");

    writeln!(commit, "author {} <{}> {} {}", author_name, author_email, timestamp, tz)?;
    writeln!(commit, "committer {} <{}> {} {}", committer_name, committer_email, timestamp, tz)?;
    writeln!(commit, "")?;
    writeln!(commit, "{message}")?;
    let hash = Object {
        kind: Kind::Commit,
        size: commit.len() as u64,
        reader: Cursor::new(commit),
    }
    .write_to_objects()
    .context("write commit object")?;

    println!("{}", hex::encode(hash));

    Ok(())
}