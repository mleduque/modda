

use std::collections::HashMap;
use std::io::BufReader;
use std::fs::File;

use anyhow::{bail, Result};
use log::debug;
use serde::Deserialize;

use crate::lowercase::LwcString;
use crate::progname::PROGNAME;

#[derive(Deserialize, Debug, Default)]
pub struct Config {

    /// The location where the archives (zip, iemod) are downloaded to.
    /// - If not set, the downloaded files are discarded after extraction.
    /// - If set, it's used as a cache, avoiding re-downloads.
    pub archive_cache: Option<String>,

    /// The (temporary) location where the archives will extracted before being moved
    /// to the game directory.
    ///
    /// This is an optimization option (avoid a copy by being on the same FS), and
    /// most probably don't need to use it.
    pub extract_location: Option<String>,

    /// If set, this is the path of the weidu binary that will be used.
    /// Supports expansion:
    /// - first environment variables are expanded (for example `/my_weidus/weidu-$WEIDU_VERSION`)
    /// - then `~` is (if present) is expanded to the user homedirectory
    /// The home directory is
    /// - `/home/<username>` on linux (by convention, but will follow $HOME if changed)
    /// - `/Users/<username>` on macos (same)
    /// - probably `C:\Users\<username>` on windows
    pub weidu_path: Option<String>,

    /// If the `weidu_path` is not set
    /// - if there is a `weidu` or `weidu.exe` file in the game directory it will be used
    /// - if this is not the case, it will expect the weidubinary to be on the path
    /// But if this options set to true, modda will ignore a weidu binary in the game directory
    /// and directly fall back to weidu-on-path
    pub ignore_current_dir_weidu: Option<bool>,

    /// Sets-up archive extractors by extension.
    /// - the key is the extension (case-insensitive)
    /// - the value contains both a`command` and an `args` properties
    /// In the `args` property, `${input}` is replaced by the archive path and `${target}`
    /// is replaced by the extraction directory.
    ///
    /// Example:
    /// ```yaml
    /// extractors:
    ///   rar:
    ///     command: unrar-nonfree
    ///     args: [ "x", "${input}", "${target}" ]
    ///   7z:
    ///     command: 7z
    ///     args: [ "x", "${input}", "-o${target}" ]
    /// ```
    #[serde(default)]
    pub extractors: HashMap<LwcString, ExtractorCommand>,
}

#[derive(Deserialize, Debug, Default)]
pub struct ExtractorCommand {
    pub command: String,
    pub args: Vec<String>,
}

pub fn read_settings() -> Result<Config> {
    let yaml_name = format!("{prog_name}.yml", prog_name = PROGNAME);
    let file = match File::open(&yaml_name) {
        Ok(file) => Some(file),
        Err(_error) => {
            if let Some(proj_dir) = directories::ProjectDirs::from("", "", PROGNAME) {
                let conf_dir = proj_dir.config_dir();
                let conf_path = conf_dir.join(&yaml_name);
                debug!("Checking settings file at {:?}", conf_path);
                match File::open(conf_path) {
                    Ok(file) => {
                        debug!("found settings file");
                        Some(file)
                    },
                    Err(_error) => None,
                }
            } else {
                None
            }
        }
    };

    match file {
        None => Ok(Config::default()),
        Some(file) => {
            let reader = BufReader::new(file);
            let config: Config = match serde_yaml::from_reader(reader) {
                Err(error) => bail!("Invalid settings file\n {error}"),
                Ok(config) => config,
            };
            debug!("Settings: {:?}", config);
            Ok(config)
        }
    }
}
