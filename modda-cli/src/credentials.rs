
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

use anyhow::{Result, bail};
use log::debug;
use serde::Deserialize;

use crate::progname::PROGNAME;

#[derive(Deserialize, Debug, Default)]
pub struct Credentials {
    #[serde(default)]
    pub github: Option<GithubCredentials>,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum GithubCredentials {
    PersonalAccessToken { personal_tokens: HashMap<String, String> },
    // maybe oauth + bearer auth later, if possible
}

impl Credentials {
    pub fn read() -> Result<Self> {
        let yaml_name = format!("{prog_name}-credentials.yml", prog_name = PROGNAME);
        let file = match File::open(&yaml_name) {
            Ok(file) => Some(file),
            Err(_error) => {
                if let Some(proj_dir) = directories::ProjectDirs::from("", "", PROGNAME) {
                    let conf_dir = proj_dir.config_dir();
                    let conf_path = conf_dir.join(&yaml_name);
                    debug!("Checking credentials file at {:?}", conf_path);
                    match File::open(conf_path) {
                        Ok(file) => {
                            debug!("found credentials file");
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
            None => Ok(Self::default()),
            Some(file) => {
                let reader = BufReader::new(file);
                let config: Self = match serde_yaml::from_reader(reader) {
                    Err(error) => bail!("Invalid credentials file\n {error}"),
                    Ok(config) => config,
                };
                Ok(config)
            }
        }
    }
}
