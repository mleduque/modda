
use std::io::{BufReader};

use anyhow::{anyhow, Result};
use serde::{Deserialize};

#[derive(Deserialize, Debug)]
pub struct Module {
    pub name: String,
    /// Which language index to use (has precedence over manifest-level lang_prefs)
    pub language: Option<u32>,
    /// List of components to be auto-installed. In None or empty list, run interactively
    pub components: Option<Vec<u32>>,
    #[serde(default)]
    pub ignore_warnings: bool,
    pub add_conf: Option<ModuleConf>,
}

#[derive(Deserialize, Debug)]
pub struct Manifest {
    #[serde(rename = "lang_dir")]
    pub game_language: String,
    /// List of language _names_ that should be selected if available, in decreasing order of priority
    /// items in the list are used as regexp (case insensitive by default)
    /// - the simplest case is just putting the expected language names 
    ///   ex. [français, french, english]
    /// - items in the list that start with `#rx#`are interpreted as regexes
    ///   syntax here https://docs.rs/regex/1.5.4/regex/#syntax
    ///   ex. ["#rx#^fran[cç]ais", french, english]
    pub lang_preferences: Option<Vec<String>>,
    pub modules: Vec<Module>,
}


#[derive(Deserialize, Debug)]
pub struct ModuleConf {
    pub file_name:String,
    pub content: ModuleContent,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum ModuleContent {
    Content(String),
    Prompt(String),
}

pub fn read_manifest(path: &str) -> Result<Manifest> {
    let file = match std::fs::File::open(path) {
        Err(error) => return Err(
            anyhow!(format!("Could not open manifest file {} - {:?}", path, error)
        )),
        Ok(file) => file,
    };
    let reader = BufReader::new(file);
    let manifest: Manifest = serde_yaml::from_reader(reader)?;
    Ok(manifest)
}
