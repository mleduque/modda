
use std::env::set_current_dir;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use globwalk::GlobWalkerBuilder;
use log::{debug, info};

pub fn ensure_chitin_key() -> Result<()> {
    if !PathBuf::from("chitin.key").exists() {
        if PathBuf::from("game/chitin.key").exists() {
            if let Err(err) = set_current_dir("game") {
                bail!("Could not enter game directory 'game' {:?}", err)
            } else {
                info!("./game/chitin.key found, entered game subdir");
            }
        } else {
            bail!("no chitin.key of game/chitin.key file");
        }
    } else {
        info!("./chitin.key found");
    }
    Ok(())
}

pub fn has_chitin_key(path: &Path) -> Result<bool> {
    let glob_builder = GlobWalkerBuilder::from_patterns(path, "chitin.key")
        .case_insensitive(true)
        .max_depth(1);
    let glob = match glob_builder.build() {
        Err(error) => bail!("Could not look up chitin.key\n -> {:?}", error),
        Ok(glob) => glob,
    };
    for item in glob.into_iter().filter_map(Result::ok) {
        debug!("Found key file : '{}'", item.file_name().to_string_lossy());
        return Ok(true)
    }
    Ok(false)
}
