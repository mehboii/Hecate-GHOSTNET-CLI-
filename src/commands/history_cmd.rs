use crate::history;
use anyhow::Result;

pub fn run(limit: usize, json: bool) -> Result<()> {
    let entries = history::recent(limit)?;
    if json {
        println!("{}", serde_json::to_string(&entries)?);
        return Ok(());
    }
    for entry in entries {
        println!(
            "{}  {} {} ({} bytes, {})",
            entry.timestamp, entry.direction, entry.peer, entry.bytes, entry.outcome
        );
    }
    Ok(())
}
