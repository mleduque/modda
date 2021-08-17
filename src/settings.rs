

use std::io::BufReader;
use std::fs::File;

use serde::Deserialize;

use crate::progname::PROGNAME;

#[derive(Deserialize, Debug, Default)]
pub struct Config {
    pub archive_cache: Option<String>,
}

pub fn read_settings() -> Config {
    let yaml_name = format!("{prog_name}.yml", prog_name = PROGNAME);
    let file = match File::open(&yaml_name) {
        Ok(file) => Some(file),
        Err(_error) => {
            if let Some(proj_dir) = directories::ProjectDirs::from("", "", PROGNAME) {
                let conf_dir = proj_dir.config_dir();
                match File::open(conf_dir.join(&yaml_name)) {
                    Ok(file) =>Some(file),
                    Err(_error) => None,
                }
            } else {
                None
            }
        }
    };

    match file {
        None => Config::default(),
        Some(file) => {
            let reader = BufReader::new(file);
            let config: Config = match serde_yaml::from_reader(reader) {
                Err(_error) => Config::default(),
                Ok(config) => config,
            };
            config
        }
    }
}
