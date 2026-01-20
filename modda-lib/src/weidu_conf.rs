
use std::fs::File;
use std::io::{BufRead, BufReader};
use anyhow::{Result, anyhow, bail};
use lazy_static::lazy_static;
use log::debug;
use regex::Regex;

use crate::canon_path::CanonPath;
use crate::utils::insensitive::find_insensitive;

lazy_static! {
    static ref LANG_DIR_REGEX: Regex = Regex::new(r##"(?i)lang_dir(\s)+=(\s)+([a-z_]+)"##).unwrap();
}

fn open_weidu_conf(game_dir: &CanonPath) -> Result<Option<File>> {
    let weidu_conf = match find_insensitive(game_dir, "weidu.conf") {
        Err(error) => bail!("Could not look for weidu.conf in {game_dir:?}\n  {error:?}"),
        Ok(None) => return Ok(None),
        Ok(Some(weidu_conf)) => {
            debug!("found conf file {weidu_conf:?}");
            game_dir.join_path(weidu_conf)
        }
    };
    match std::fs::File::open(weidu_conf) {
        Err(error) => return Err(
            anyhow!(format!("Could not open weidu.conf - {:?}", error)
        )),
        Ok(file) => Ok(Some(file)),
    }
}

pub fn check_weidu_conf_lang(game_dir: &CanonPath, lang: &str) -> Result<()> {
    let file = match open_weidu_conf(game_dir)? {
        None => return Ok(()),
        Some(file) => file,
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
    let file = match open_weidu_conf(game_dir)? {
        None => return Ok(None),
        Some(file) => file,
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
