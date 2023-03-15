
use anyhow::bail;
use anyhow::Result;

use crate::module::weidu_mod::WeiduMod;
use crate::run_weidu::list_available_languages;
use crate::settings::Config;

#[derive(Clone, Debug)]
pub struct LanguageOption {
    pub index: u32,
    pub name: String,
}

#[derive(Clone, Debug)]
pub enum LanguageSelection {
    Selected(u32),
    NoPrefSet(Vec<LanguageOption>),
    NoMatch(Vec<LanguageOption>),
}

pub fn select_language(tp2:&str, module: &WeiduMod, lang_preferences: &Option<Vec<String>>, config: &Config) -> Result<LanguageSelection> {
    use LanguageSelection::*;

    if let Some(idx) = module.language {
        Ok(Selected(idx))
    } else {
        let available = match list_available_languages(tp2, module, config) {
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
