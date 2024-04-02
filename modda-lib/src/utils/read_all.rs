
use std::io::BufRead;
use std::path::Path;

use anyhow::{bail, Result};

pub fn read_all(path: &Path) -> Result<Vec<String>> {
    let file = std::fs::File::open(path)?;
    let buf = std::io::BufReader::new(file);
    let mut lines = vec![];
    for line in buf.lines() {
        match line {
            Ok(line) => lines.push(line),
            Err(error) => bail!("Error reading file {:?}\n -> {:?}", path, error),
        }
    }
    Ok(lines)
}
