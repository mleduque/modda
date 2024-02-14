use std::io::{BufRead, BufReader};
use std::path::Path;
use anyhow::{Result, anyhow, bail};
use lazy_static::lazy_static;
use regex::Regex;

use crate::canon_path::CanonPath;

lazy_static! {
    static ref LANG_DIR_REGEX: Regex = Regex::new(r##"(?i)lang_dir(\s)+=(\s)+([a-z_]+)"##).unwrap();
}

pub fn check_weidu_conf_lang(game_dir: &CanonPath, lang: &str) -> Result<()> {
    if !Path::new("weidu.conf").exists() {
        return Ok(())
    }
    let file = match std::fs::File::open(game_dir.join_path("weidu.conf")) {
        Err(error) => return Err(
            anyhow!(format!("Could not open weidu.conf - {:?}", error)
        )),
        Ok(file) => file,
    };
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line?;
        if let Some(caps) = LANG_DIR_REGEX.captures_iter(&line).next() {
            if caps[3].to_lowercase() == lang.to_lowercase() {
                return Ok(())
            } else {
                bail!(
                    "lang_dir (in manifest) {} doesn't match value in weidu.conf {}",
                    lang, &caps[3]
                );
            }
        }
    }
    Ok(())
}

pub fn read_weidu_conf_lang_dir(game_dir: &CanonPath) -> Result<Option<String>> {
    if !Path::new("weidu.conf").exists() {
        return Ok(None)
    }
    let file = match std::fs::File::open(game_dir.join_path("weidu.conf")) {
        Err(error) => return Err(
            anyhow!(format!("Could not open weidu.conf - {:?}", error)
        )),
        Ok(file) => file,
    };

    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line?;
        if let Some(caps) = LANG_DIR_REGEX.captures_iter(&line).next() {
            return Ok(Some(caps[3].to_lowercase()))
        }
    }
    Ok(None)
}
