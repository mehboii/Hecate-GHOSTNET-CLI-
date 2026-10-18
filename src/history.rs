//! Local audit metadata only: no message body or seed phrase is written to disk.
use crate::config;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};

#[derive(Serialize, Deserialize)]
pub struct Entry {
    pub timestamp: u64,
    pub direction: String,
    pub peer: String,
    pub bytes: usize,
    pub outcome: String,
}

fn path() -> Result<std::path::PathBuf> {
    Ok(config::data_dir()?.join("history.ndjson"))
}

pub fn record(direction: &str, peer: &str, bytes: usize, outcome: &str) -> Result<()> {
    std::fs::create_dir_all(config::data_dir()?)?;
    let entry = Entry {
        timestamp: config::unix_time(),
        direction: direction.into(),
        peer: peer.into(),
        bytes,
        outcome: outcome.into(),
    };
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path()?)?;
    writeln!(file, "{}", serde_json::to_string(&entry)?)?;
    Ok(())
}

pub fn recent(limit: usize) -> Result<Vec<Entry>> {
    let path = path()?;
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    let mut entries = std::collections::VecDeque::new();
    let mut corrupt = 0;
    for line in BufReader::new(file).lines() {
        let line = line?;
        match serde_json::from_str::<Entry>(&line) {
            Ok(entry) if limit > 0 => {
                entries.push_back(entry);
                if entries.len() > limit {
                    entries.pop_front();
                }
            }
            Ok(_) => {}
            Err(_) => corrupt += 1,
        }
    }
    if corrupt > 0 {
        eprintln!("warning: skipped {corrupt} malformed history records");
    }
    let mut entries: Vec<Entry> = entries.into_iter().collect();
    entries.reverse();
    entries.truncate(limit);
    Ok(entries)
}
