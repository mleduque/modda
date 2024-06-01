
use anyhow::{bail, Result};
use modda_lib::config::global_conf_dir;

pub fn open_global_config_dir() -> Result<()> {
    let directory = global_conf_dir()
        .expect("Could not determine the global config location");
    if !directory.exists() {
        if let Err(error) = std::fs::create_dir_all(&directory) {
            bail!("Could not create global config directory {dir}\n  {error}",
                    dir = directory.as_os_str().to_string_lossy())
        }
    }
    open::that_detached(directory)?;
    Ok(())
}
