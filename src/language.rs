
use std::io::BufRead;
use std::process::Command;

use anyhow::{bail};
use anyhow::Result;

use crate::manifest::{ Module };

#[derive(Clone, Debug)]
pub struct LanguageOption {
    index: u32,
    name: String,
}

#[derive(Clone, Debug)]
pub enum LanguageSelection {
    Selected(u32),
    NoPrefSet(Vec<LanguageOption>),
    NoMatch(Vec<LanguageOption>),
}

pub fn select_language(tp2:&str, module: &Module, lang_preferences: &Option<Vec<String>>) -> Result<LanguageSelection> {
    use LanguageSelection::*;

    if let Some(idx) = module.language {
        Ok(Selected(idx))
    } else {
        let available = match list_available_languages(tp2, module) {
            Ok(result) => result,
            Err(error) =>  bail!(
                "Couldn't get list of available language for module {} - {:?}",
                module.name,
                error,
            )
        };
        match lang_preferences {
            None => Ok(NoPrefSet(available)),
            Some(names) if names.is_empty() => Ok(NoPrefSet(available)),
            Some(candidates) => {
                for candidate in candidates {
                    let candidate = candidate.trim();
                    if candidate.is_empty() {
                        continue;
                    }
                    match candidate.strip_prefix("#rx#") {
                        Some(reg) => {
                            let lang_re = regex::Regex::new(&format!("(?i){}", reg)).unwrap();
                            for lang in &available {
                                let LanguageOption { index, name } = &lang;
                                if lang_re.is_match(name) {
                                    return Ok(Selected(*index));
                                }
                            }
                        }
                        None => {
                            // use candidate for exact search
                            for lang in &available {
                                let LanguageOption { index, name } = &lang;
                                if candidate.to_lowercase() == name.to_lowercase() {
                                    return Ok(Selected(*index));
                                }
                            }                            
                        }
                    }
                }
                // tried everything, no match
                Ok(NoMatch(available))
            }
        }
    }
}

fn list_available_languages(tp2: &str, module: &Module) -> Result<Vec<LanguageOption>> {
    let mut command = Command::new("weidu");
    let args = vec![
        "--no-exit-pause".to_owned(),
        "--list-languages".to_owned(),
        tp2.to_owned(),
    ];
    command.args(&args);
    let output = command.output()?;

    // the first line show a version string starting with [weidu], then some lines 
    // with [<some file name>] ...
    // then n language lines in the form
    // <integer>COLON<string(language name)>
    let lines = output.stdout.lines();
    for line in output.stderr.lines() {
        println!("on stderr {:?}", line);
    }


    use lazy_static::lazy_static;
    use regex::Regex;
    lazy_static! {
        static ref language_regex: Regex = Regex::new("^([0-9]*):(.*)$").unwrap();
    }
    let mut lines_ok = vec![];
    for line in lines {
        match line {
            Err(err) => bail!("Couldn't obtain language list for module{} [error reading output] _ {:?}", 
                            module.name, err),
            Ok(line) => {
                lines_ok.push(line);
            }
        }
    }
    let entries = lines_ok.iter().filter_map(|line| match language_regex.captures(line) {
        None => None,
        Some(cap) => {
            match (cap.get(1), cap.get(2)) {
                (Some(index), Some(name)) => match u32::from_str_radix(index.as_str(), 10) {
                    Ok(index) => Some((index, name.as_str().to_owned())),
                    Err(_err) => None
                }
                _  => None
            }
        }
    }).collect::<Vec<_>>();
    Ok(entries.into_iter().map(|(index, name)| LanguageOption { index, name }).collect())
}
