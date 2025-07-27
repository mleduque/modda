

use std::collections::HashMap;
use std::hash::Hash;
use std::io::{BufReader, Result as IoResult};
use std::fs::File;
use std::path::{Path, PathBuf};

use anyhow::{bail, Ok, Result};
use log::{debug, warn};
use serde::{Deserialize, Serialize};

use crate::canon_path::CanonPath;
use crate::lowercase::LwcString;
use crate::obtain::get_options::StrictReplaceAction;
use crate::progname::PROGNAME;

pub const ARCHIVE_CACHE_ENV_VAR: &'static str = "MODDA_ARCHIVE_CACHE";
pub const EXTRACT_LOCATION_ENV_VAR: &'static str = "MODDA_EXTRACT_LOCATION";
pub const WEIDU_PATH_ENV_VAR: &'static str = "MODDA_WEIDU_PATH";
pub const IGNORE_CURRENT_DIR_WEIDU_ENV_VAR: &'static str = "MODDA_IGNORE_CURRENT_DIR_WEIDU";
pub const CODE_EDITOR_ENV_VAR: &'static str = "MODDA_CODE_EDITOR";

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
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

    /// Path to the code editor program.<br>
    /// Used with the `config edit` subcommands.
    pub code_editor: Option<String>,

    pub defaults: Option<DefaultOptions>,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct DefaultOptions {
    /// What reaction to have when a replace doesn't work expected
    #[serde(default)]
    pub check_replace: Option<StrictReplaceAction>,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct ExtractorCommand {
    pub command: String,
    pub args: Vec<String>,
}

pub fn global_conf_dir() -> Option<PathBuf> {
    if let Some(proj_dir) = directories::ProjectDirs::from("", "", PROGNAME) {
        Some(proj_dir.config_dir().to_path_buf())
    } else {
        None
    }
}

#[derive(Debug, Default, Clone)]
pub struct Settings {
    pub global: Option<ConfigSource>,
    pub local: Option<ConfigSource>,
    pub env_config: ConfigSource,
    pub combined: Config,
}

#[derive(Debug, Default, Clone)]
pub struct ConfigSource {
    pub id: String,
    pub config: Option<Config>,
}

impl Settings {
    pub fn read_settings(game_dir: &CanonPath) -> Result<Settings> {
        let global = match global_conf_dir() {
            Some(path_buf) => match Self::read_config_in_dir(&path_buf) {
                Result::Ok(None) => None,
                Result::Ok(Some(config_source)) => Some(config_source),
                Err(error) => bail!("Error reading app global config\n  {error}"),
            }
            None => None,
        };
        let local = match Self::read_config_in_dir(&game_dir.to_path_buf()) {
            Result::Ok(None) => None,
            Result::Ok(Some(config_source)) => Some(config_source),
            Err(error) => bail!("Error reading app local config\n  {error}"),
        };
        let env_config = Self::read_env_config()?;

        Ok(Settings {
            global: global.clone(),
            local: local.clone(),
            env_config: env_config.clone(),
            combined: combine(
                match global {
                    None => None,
                    Some(ConfigSource { config: None, .. }) => None,
                    Some(ConfigSource { config: Some(config), ..}) => Some(config),
                },
                match local {
                    None => None,
                    Some(ConfigSource { config: None, .. }) => None,
                    Some(ConfigSource { config:Some(config), ..}) => Some(config),
                },
                match env_config {
                    ConfigSource { config: None, .. } => None,
                    ConfigSource { config:Some(config), ..} => Some(config),
                },
            ),
        })
    }

    pub fn find_config_in_dir(dir: &Path) -> Result<Option<PathBuf>> {
        let yml_name = format!("{prog_name}.yml", prog_name = PROGNAME);
        let yaml_name = format!("{prog_name}.yaml", prog_name = PROGNAME);
        let yml_path = dir.join(yml_name.to_string());
        let yaml_path = dir.join(yaml_name.to_string());
        let yml_file_exists = yml_path.exists();
        let yaml_file_exists = yaml_path.exists();
        match (yml_file_exists, yaml_file_exists) {
            (false, false) => Ok(None),
            (true, false) => Ok(Some(yml_path)),
            (false, true) => Ok(Some(yaml_path)),
            (true, true) =>
                bail!("Both {yml_name} and {yaml_name} files are present in {dir:?} and I can't choose.\nPlease delete one of those.")
        }
    }

    pub fn read_config_in_dir(dir: &Path) -> Result<Option<ConfigSource>> {
        let candidate = Settings::find_config_in_dir(dir)?;
        match candidate {
            None => Ok(None),
            Some(path) => {
                let file = match File::open(&path) {
                    IoResult::Ok(file) => file,
                    Err(error) => bail!("Could not open config file at {:?}\n  {error}", path),
                };
                let path_as_str = path.as_os_str().to_string_lossy().to_string();
                debug!("found config file at {path_as_str}");

                let reader = BufReader::new(file);
                let config = match serde_yaml::from_reader(reader) {
                    std::result::Result::Ok(config) => Some(config),
                    std::result::Result::Err(err) => {
                        warn!("Could not read config file at {path_as_str}\n{err}");
                        None
                    }
                };
                debug!("Config read at {path_as_str}: {config:?}");
                Ok(Some(ConfigSource {
                    id: path_as_str,
                    config
                }))
            }
        }
    }

    fn read_env_config() -> Result<ConfigSource> {
        let ignore_current_dir_weidu = match std::env::var(IGNORE_CURRENT_DIR_WEIDU_ENV_VAR) {
            Err(_) => None,
            Result::Ok(s) if s == "true" => Some(true),
            Result::Ok(s) if s == "false" => Some(false),
            _ => bail!("Incorrect value for {IGNORE_CURRENT_DIR_WEIDU_ENV_VAR} env var")
        };
        Ok(ConfigSource {
            id: "environment".to_string(),
            config: Some(Config {
                archive_cache: std::env::var(ARCHIVE_CACHE_ENV_VAR).ok(),
                extract_location: std::env::var(EXTRACT_LOCATION_ENV_VAR).ok(),
                weidu_path: std::env::var(WEIDU_PATH_ENV_VAR).ok(),
                ignore_current_dir_weidu,
                // Setting extractor not supported for now
                extractors: HashMap::new(),
                code_editor: std::env::var(CODE_EDITOR_ENV_VAR).ok(),
                // Setting default not supporte for now
                defaults: None,
            })
        })
    }

}

fn combine(global: Option<Config>, local: Option<Config>, env_config: Option<Config>) -> Config {
    let global = global.unwrap_or_else(|| Config::default());
    let local = local.unwrap_or_else(|| Config::default());
    let env_config = env_config.unwrap_or_else(|| Config::default());
    Config {
        archive_cache: env_config.archive_cache.or(local.archive_cache).or(global.archive_cache),
        extract_location: env_config.extract_location.or(local.extract_location).or(global.extract_location),
        weidu_path: env_config.weidu_path.or(local.weidu_path).or(global.weidu_path),
        ignore_current_dir_weidu: env_config.ignore_current_dir_weidu.or(local.ignore_current_dir_weidu).or(global.ignore_current_dir_weidu),
        extractors: merge_maps(&global.extractors, &local.extractors, &env_config.extractors),
        code_editor: env_config.code_editor.or(local.code_editor).or(global.code_editor),
        defaults: Some(combine_defaults(global.defaults, local.defaults, env_config.defaults)),
    }
}

fn combine_defaults(global: Option<DefaultOptions>, local: Option<DefaultOptions>, env_config: Option<DefaultOptions>) -> DefaultOptions {
    let global = global.unwrap_or_else(|| DefaultOptions::default());
    let local = local.unwrap_or_else(|| DefaultOptions::default());
    let env_config = env_config.unwrap_or_else(|| DefaultOptions::default());
    DefaultOptions {
        check_replace: env_config.check_replace.or(global.check_replace).or(local.check_replace),
    }
}

fn merge_maps<K, V>(bottom: &HashMap<K, V>, middle: &HashMap<K, V>, top: &HashMap<K, V>) -> HashMap<K, V>
        where K: Eq + Hash + Clone, V: Clone {
    bottom.into_iter().chain(middle).chain(top).map(|(k, v)| (k.clone(), v.clone())).collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::config::merge_maps;

    #[test]
    fn test_merger_maps() {
        let bottom = HashMap::from([
            ("a", "1"),
            ("b", "1"),
            ("c", "1"),
        ]);
        let middle = HashMap::from([
            ("a", "2"),
            ("b", "2"),
            ("d", "2"),
            ("e", "2"),
        ]);
        let top = HashMap::from([
            ("a", "3"),
            ("b", "3"),
            ("d", "3"),
            ("f", "3"),
        ]);

        let expected = HashMap::from([
            ("a", "3"),
            ("b", "3"),
            ("c", "1"),
            ("d", "3"),
            ("e", "2"),
            ("f", "3"),
        ]);

        assert_eq!(
            merge_maps(&bottom, &middle, &top),
            expected
        )
    }
}
