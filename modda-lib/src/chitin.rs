
use std::env::set_current_dir;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use log::{debug, error, info};

use crate::utils::insensitive::find_insensitive;

pub fn ensure_chitin_key(path: &Path) -> Result<()> {
    match has_chitin_key(&PathBuf::from(path))? {
        true => info!("./chitin.key found"),
        false => {
            info!("trying ./game/chitin.key ");
            let game =  path.join("game");
            match has_chitin_key(&game) {
                Err(error) => {
                    error!("failed chitin.key lookup\n  {:?}", error);
                    bail!("failed chitin.key lookup\n  {:?}", error);
                }
                Ok(true) => if let Err(err) = set_current_dir(game) {
                    bail!("Could not enter game directory 'game' {:?}", err)
                } else {
                    info!("./game/chitin.key found, entered game subdir");
                },
                Ok(false) => bail!("no chitin.key or game/chitin.key file"),
            }
        }
    }
    Ok(())
}

pub fn has_chitin_key(path: &Path) -> Result<bool> {
    match find_insensitive(path, "chitin.key") {
        Err(error) => bail!("Could not look for chitin.key in {path:?}\n  {error:?}"),
        Ok(None) => Ok(false),
        Ok(Some(chitin)) => {
            debug!("found file {chitin:?}");
            Ok(true)
        }
    }
}
