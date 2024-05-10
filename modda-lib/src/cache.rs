
use std::path::{PathBuf, Path};

use anyhow::{bail, Result};

use crate::config::Config;



pub enum Cache {
    Tmp(tempfile::TempDir),
    Path(PathBuf),
}
impl Cache {
    pub fn ensure_from_config(config: &Config) -> Result<Self> {
        match &config.archive_cache {
            None => match tempfile::tempdir() {
                Err(error) => bail!("Couldn't set up archive cache\n -> {:?}", error),
                Ok(dir) => Ok(Cache::Tmp(dir),)
            }
            Some(path) => {
                let expanded = match shellexpand::full(path) {
                    Err(error) => bail!("Cache location expansion failed\n  {error}"),
                    Ok(expanded) => expanded,
                };
                if let Err(error) = std::fs::create_dir_all(&*expanded) {
                    bail!("Could not create destination dir{:?}\n -> {:?}", expanded, error);
                }
                Ok(Cache::Path(PathBuf::from(&*expanded)))
            }
        }
    }

    pub fn join<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        match self {
            Cache::Tmp(tmpdir) => tmpdir.path().join(path),
            Cache::Path(base_path) => base_path.join(path),
        }
    }
}
