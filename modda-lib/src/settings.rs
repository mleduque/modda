

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
    pub archive_cache: Option<String>,
    pub extract_location: Option<String>,
    pub weidu_path: Option<String>,
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
