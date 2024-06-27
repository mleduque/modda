
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::OnceLock;

use anyhow::{bail, Result};
use itertools::{FoldWhile, Itertools};
use log::warn;
use regex::{Captures, Regex};
use serde::{Deserialize, Serialize};

use crate::canon_path::CanonPath;

use super::manifest_conditions::ManifestConditions;


#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
pub enum DisableCondition {
    /// Just disable this module and provides a reason.<br>
    /// This means it always evaluate as "disabled"
    Because { because: String },
    /// Disables the module if the environment variable exists and is not empty.
    EnvVar { env_is_set: String },
    /// Check condition in a text file.<br>
    /// The location of the file is `<path_of_manifest_yaml>/<in_file>`<br>
    /// The file should be in UTF-8 and should follow the format
    /// ```text
    /// my_key_1=false; because I don't feel like it now
    /// my_key_2=false
    /// my_key_3=true
    /// ```
    ///
    /// - If the file doesn't exist, the condition  is evaluated as "don't disable".<br>
    /// - if the file doesn't contain the key, the condition  is evaluated as "don't disable".
    File { in_file: String, key: String },
    /// Disables the module if any of the conditions inside evaluates to "disabled".<br>
    /// (If there is no sub-condition, it never disables)
    Any { any: Vec<DisableCondition> },
    /// Disables the module if all of the conditions inside evaluates to "disabled"..<br>
    /// (If there is no sub-condition, it always disables)
    All { all: Vec<DisableCondition> },
    /// Disables the module iff the condition inside evaluates to "not disabled".
    Not { not: Box<DisableCondition> },
    /// Disables the module on a condition defined in the manifest
    ManifestCondition { manifest_condition: String },
}

pub trait DisableCheck {
    fn check(&self, manifest_root: &CanonPath, manifest_conditions: &ManifestConditions) -> Result<DisableOutCome>;
}

impl DisableCheck for DisableCondition {
    fn check(&self, manifest_root: &CanonPath, manifest_conditions: &ManifestConditions) -> Result<DisableOutCome> {
        match self {
            Self::Because { ref because } => Ok(DisableOutCome::Yes(because.to_string())),
            Self::EnvVar { env_is_set: if_env_set } => {
                match std::env::var_os(if_env_set) {
                    Some(s) if s.is_empty() =>
                        Ok(DisableOutCome::No(Some(format!("Environment variable '{if_env_set}' is empty")))),
                    None =>
                        Ok(DisableOutCome::No(Some(format!("Environment variable '{if_env_set}' is not set")))),
                    Some(value) => Ok(DisableOutCome::Yes(
                        format!("Environment variable '{if_env_set}' is set to '{}'", value.to_string_lossy())
                    )),
                }
            }
            Self::File { in_file, key } => evaluate_file(in_file, key, manifest_root),
            Self::Any { any } => evaluate_any(any, manifest_root, manifest_conditions),
            Self::All { all } => evaluate_all(all, manifest_root, manifest_conditions),
            Self::Not { not } => {
                use DisableOutCome::{Yes, No};
                match not.check(manifest_root, manifest_conditions) {
                    Err(error) => Err(error),
                    Ok(No(None)) => Ok(Yes("Negation of 'not disabled'".to_string())),
                    Ok(No(Some(reason))) =>
                        Ok(Yes(format!("Negation of condition:\n  not disabled because '{reason}'"))),
                    Ok(Yes(reason)) =>
                        Ok(No(Some(format!("Negation of condition:\n  disabled because '{reason}'")))),
                }
            }
            Self::ManifestCondition { manifest_condition } => match manifest_conditions.get(&manifest_condition) {
                None => Ok(DisableOutCome::No(Some(format!("manifest global condition {} is not present", manifest_condition)))),
                Some(condition) => condition.check(manifest_root, manifest_conditions)
            }
        }
    }
}

impl DisableCheck for Option<DisableCondition> {
    fn check(&self, manifest_root: &CanonPath, manifest_conditions: &ManifestConditions) -> Result<DisableOutCome> {
        match self {
            None => Ok(DisableOutCome::No(None)),
            Some(condition) => condition.check(manifest_root, manifest_conditions)
        }
    }
}

fn disable_file_regex() -> &'static Regex {
    static DISABLE_FILE_REGEX: OnceLock<Regex> = OnceLock::new();
    DISABLE_FILE_REGEX.get_or_init(|| {
        Regex::new(r#"^(?<key>\w+)\s*=\s*(?<value>\w+)(?:\s*;\s*(?<reason>.*))?$"#).unwrap()
    })
}


fn evaluate_file(in_file: &str, key: &str, manifest_root: &CanonPath) -> Result<DisableOutCome> {
    let file_path = manifest_root.join(in_file)?;
    if !file_path.starts_with(manifest_root) {
        bail!("File for disable condition '{in_file:?}' is not under the manifest location.");
    }
    if !file_path.path().exists() {
        return Ok(DisableOutCome::No(Some(format!("File '{in_file}' does not exist"))));
    }
    let file = File::open(file_path)?;
    let buf = BufReader::new(file);
    let entries = buf.lines().enumerate();
    for line in entries {
        if let (num, Ok(line)) = line {
            if let Some(parts) = disable_file_regex().captures(&line) {
                match parts.name("key") {
                    Some(s) if s.as_str() == key => {
                        let disabled = parts.name("value");
                        return match disabled {
                            Some(s) if s.as_str() == "true" => Ok(file_outcome(parts, true, in_file)),
                            Some(s) if s.as_str() == "false" => Ok(file_outcome(parts, false, in_file)),
                            Some(s) => bail!("Invalid value in {in_file} ; expected true or false but got {value} (key={key})", value = s.as_str()),
                            None => bail!("Invalid line in '{in_file}' ; value is missing (key={key})"),
                        }
                    }
                    None => warn!("Unexpected result for regex match ; found a program error?"),
                    _ => {}
                }
            } else {
                warn!("Invalid format in file {in_file} at line {num}")
            }
        } // else ignore and continue
    }
    // end of file, we did not find any matching line
    Ok(DisableOutCome::No(Some(format!("Key '{key}' not present in file '{in_file}'"))))
}

fn file_outcome(captures: Captures, disabled: bool, file_name: &str) -> DisableOutCome {
    let reason = captures.name("reason")
        .map(|m| m.as_str().to_owned());
    if disabled {
        DisableOutCome::Yes(reason.unwrap_or_else(|| format!("disabled in file '{file_name}'")))
    } else {
        DisableOutCome::No(reason.or_else(|| Some(format!("not disabled in file '{file_name}'"))))
    }
}

fn evaluate_all(conditions: &[DisableCondition], manifest_root: &CanonPath, manifest_conditions: &ManifestConditions) -> Result<DisableOutCome> {
    conditions.iter().fold_while(
        Ok(DisableOutCome::Yes("all conditions filled".to_string())),
        |acc, condition| {
            match condition.check(manifest_root, manifest_conditions) {
                Err(error) => FoldWhile::Done(Err(error)),
                Ok(DisableOutCome::Yes(_yes)) => FoldWhile::Continue(acc),
                Ok(no) => FoldWhile::Done(Ok(no)),
            }
        }
    ).into_inner()
}

fn evaluate_any(conditions: &[DisableCondition], manifest_root: &CanonPath, manifest_conditions: &ManifestConditions) -> Result<DisableOutCome> {
    conditions.iter().fold_while(
        Ok(DisableOutCome::No(None)),
        |acc, condition| {
            match condition.check(manifest_root, manifest_conditions) {
                Err(error) => FoldWhile::Done(Err(error)),
                Ok(DisableOutCome::No(_)) => FoldWhile::Continue(acc),
                Ok(yes) => FoldWhile::Done(Ok(yes)),
            }
        }
    ).into_inner()
}

#[derive(Debug, PartialEq, Clone)]
pub enum DisableOutCome {
    Yes(String),
    No(Option<String>),
}

impl DisableOutCome {
    pub fn is_yes(&self) -> bool {
        match self {
            Self::No(_) => false,
            _ => true,
        }
    }

    pub fn is_no(&self) -> bool {
        match self {
            Self::Yes(_) => false,
            _ => true,
        }
    }
}

#[cfg(test)]
mod test {

    use std::collections::HashMap;
    use std::path::Path;

    use crate::canon_path::CanonPath;
    use crate::module::disable_condition::{DisableCheck, DisableCondition, DisableOutCome};
    use crate::module::manifest_conditions::ManifestConditions;

    #[test]
    fn evaluate_unconditional_disable() {
        let because = "I'm testing things".to_string();
        assert_eq!(
            DisableCondition::Because { because: because.clone() }
                .check(&CanonPath::new("").unwrap(), &ManifestConditions::default()).unwrap(),
            DisableOutCome::Yes(because),
        )
    }

    #[test]
    fn evaluate_env_is_set_with_env_really_set() {
        let env_var = "MY_ENV_VAR";
        let value = "some value".to_string();
        temp_env::with_var(env_var, Some(&value), || {
            assert_eq!(
                DisableCondition::EnvVar { env_is_set: "MY_ENV_VAR".to_string() }
                    .check(&CanonPath::new("").unwrap(), &ManifestConditions::default()).unwrap(),
                DisableOutCome::Yes(format!("Environment variable '{}' is set to '{}'", env_var, value)),
            )
        })
    }

    #[test]
    fn evaluate_env_is_set_with_env_set_to_empty() {
        let env_var = "MY_ENV_VAR";
        temp_env::with_var(env_var, Some(""), || {
            assert_eq!(
                DisableCondition::EnvVar { env_is_set: "MY_ENV_VAR".to_string() }
                    .check(&CanonPath::new("").unwrap(), &ManifestConditions::default()).unwrap(),
                DisableOutCome::No(Some(format!("Environment variable '{}' is empty", env_var))),
            )
        })
    }

    #[test]
    fn evaluate_env_is_set_with_env_not_set() {
        let env_var = "MY_ENV_VAR";
        temp_env::with_var(env_var, None::<String>, || {
            assert_eq!(
                DisableCondition::EnvVar { env_is_set: "MY_ENV_VAR".to_string() }
                    .check(&CanonPath::new("").unwrap(), &ManifestConditions::default()).unwrap(),
                DisableOutCome::No(Some(format!("Environment variable '{}' is not set", env_var))),
            )
        })
    }

    #[test]
    fn evaluate_file_condition_with_file_absent() {
        let base_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test/disable");
        assert_eq!(
            DisableCondition::File { in_file: "missing".to_string(), key: "my_key".to_string() }
                .check(&CanonPath::new(base_path).unwrap(), &ManifestConditions::default()).unwrap(),
            DisableOutCome::No(Some(format!("File '{}' does not exist", "missing"))),
        )
    }

    #[test]
    fn evaluate_file_condition_with_missing_key_in_file() {
        let base_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test/disable");
        let key = "my_key".to_string();
        let file_name = "example".to_string();
        assert_eq!(
            DisableCondition::File { in_file: file_name.clone(), key: key.clone() }
                .check(&CanonPath::new(base_path).unwrap(), &ManifestConditions::default()).unwrap(),
            DisableOutCome::No(Some(format!("Key '{}' not present in file '{}'", key, file_name))),
        )
    }

    #[test]
    fn evaluate_file_condition_with_key_and_value_true_and_no_comments() {
        let base_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test/disable");
        let key = "my_key_1".to_string();
        let file_name = "example".to_string();
        assert_eq!(
            DisableCondition::File { in_file: file_name.clone(), key: key.clone() }
                .check(&CanonPath::new(base_path).unwrap(), &ManifestConditions::default()).unwrap(),
            DisableOutCome::Yes(format!("disabled in file '{file_name}'")),
        )
    }

    #[test]
    fn evaluate_file_condition_with_key_and_value_false_and_no_comments() {
        let base_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test/disable");
        let key = "my_key_2".to_string();
        let file_name = "example".to_string();
        assert_eq!(
            DisableCondition::File { in_file: file_name.clone(), key: key.clone() }
                .check(&CanonPath::new(base_path).unwrap(), &ManifestConditions::default()).unwrap(),
            DisableOutCome::No(Some(format!("not disabled in file '{file_name}'"))),
        )
    }

    #[test]
    fn evaluate_file_condition_with_key_and_value_true_and_comments() {
        let base_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test/disable");
        let key = "my_key_3".to_string();
        let file_name = "example".to_string();
        assert_eq!(
            DisableCondition::File { in_file: file_name.clone(), key: key.clone() }
                .check(&CanonPath::new(base_path).unwrap(), &ManifestConditions::default()).unwrap(),
            DisableOutCome::Yes(format!("I don't want this")),
        )
    }

    #[test]
    fn evaluate_file_condition_with_key_and_value_false_and_comments() {
        let base_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test/disable");
        let key = "my_key_4".to_string();
        let file_name = "example".to_string();
        assert_eq!(
            DisableCondition::File { in_file: file_name.clone(), key: key.clone() }
                .check(&CanonPath::new(base_path).unwrap(), &ManifestConditions::default()).unwrap(),
            DisableOutCome::No(Some(format!("but this one is ok"))),
        )
    }

    #[test]
    fn evaluate_file_condition_with_multiple_occurrences() {
        let base_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test/disable");
        let key = "my_key_5".to_string();
        let file_name = "example".to_string();
        assert_eq!(
            DisableCondition::File { in_file: file_name.clone(), key: key.clone() }
                .check(&CanonPath::new(base_path).unwrap(), &ManifestConditions::default()).unwrap(),
            DisableOutCome::No(Some(format!("this one is kept"))),
        )
    }

    #[test]
    fn evaluate_file_condition_with_incorrect_value() {
        let base_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test/disable");
        let key = "my_key_6".to_string();
        let file_name = "example".to_string();
        assert!(
            DisableCondition::File { in_file: file_name.clone(), key: key.clone() }
                .check(&CanonPath::new(base_path).unwrap(), &ManifestConditions::default())
                .is_err()
        )
    }

    #[test]
    fn evaluate_file_condition_with_subdirt() {
        let base_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test/disable");
        let key = "my_key".to_string();
        let file_name = "subdir/example2".to_string();
        assert_eq!(
            DisableCondition::File { in_file: file_name.clone(), key: key.clone() }
                .check(&CanonPath::new(base_path).unwrap(), &ManifestConditions::default()).unwrap(),
            DisableOutCome::Yes(format!("disabled in file '{file_name}'")),
        )
    }

    #[test]
    fn evaluate_file_condition_with_path_not_under_manifest_root() {
        let base_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test/disable");
        let key = "my_key_6".to_string();
        let file_name = "../example".to_string();
        assert!(
            DisableCondition::File { in_file: file_name.clone(), key: key.clone() }
                .check(&CanonPath::new(base_path).unwrap(), &ManifestConditions::default()).is_err()
        )
    }

    #[test]
    fn evaluate_any_condition_zero_sub_conditions() {
        assert_eq!(
            DisableCondition::Any { any: vec![]}
                .check(&CanonPath::new("").unwrap(), &ManifestConditions::default()).unwrap(),
            DisableOutCome::No(None),
        )
    }

    #[test]
    fn evaluate_any_condition_one_true_sub_condition() {
        assert_eq!(
            DisableCondition::Any { any: vec![
                DisableCondition::Because { because: "no reason".to_string() },
            ]}.check(&CanonPath::new("").unwrap(), &ManifestConditions::default()).unwrap(),
            DisableOutCome::Yes("no reason".to_string()),
        )
    }

    #[test]
    fn evaluate_any_condition_one_false_sub_condition() {
        let base_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test/disable");

        assert_eq!(
            DisableCondition::Any { any: vec![
                DisableCondition::File { in_file: "missing".to_string(), key: "my_key".to_string() },
            ]}.check(&CanonPath::new(base_path).unwrap(), &ManifestConditions::default()).unwrap(),
            DisableOutCome::No(None),
        )
    }

    #[test]
    fn evaluate_any_condition_one_true_and_one_false_sub_conditions() {
        let base_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test/disable");

        assert_eq!(
            DisableCondition::Any { any: vec![
                DisableCondition::Because { because: "no reason".to_string() },
                DisableCondition::File { in_file: "missing".to_string(), key: "my_key".to_string() },
            ]}.check(&CanonPath::new(base_path).unwrap(), &ManifestConditions::default()).unwrap(),
            DisableOutCome::Yes("no reason".to_string()),
        )
    }

    #[test]
    fn evaluate_any_condition_one_false_and_one_true_sub_conditions() {
        let base_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test/disable");

        assert_eq!(
            DisableCondition::Any { any: vec![
                DisableCondition::File { in_file: "missing".to_string(), key: "my_key".to_string() },
                DisableCondition::Because { because: "no reason".to_string() },
            ]}.check(&CanonPath::new(base_path).unwrap(), &ManifestConditions::default()).unwrap(),
            DisableOutCome::Yes("no reason".to_string()),
        )
    }

    #[test]
    fn evaluate_all_condition_zero_sub_conditions() {
            assert!(
                DisableCondition::All { all: vec![] }
                    .check(&CanonPath::new("").unwrap(), &ManifestConditions::default()).unwrap()
                    .is_yes()
            )
    }

    #[test]
    fn evaluate_all_condition_one_true_sub_condition() {
        assert_eq!(
            DisableCondition::All { all: vec![
                DisableCondition::Because { because: "no reason".to_string() },
            ]}.check(&CanonPath::new("").unwrap(), &ManifestConditions::default()).unwrap(),
            DisableOutCome::Yes("all conditions filled".to_string()),
        )
    }

    #[test]
    fn evaluate_all_condition_one_false_sub_condition() {
        let base_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test/disable");

        assert_eq!(
            DisableCondition::All { all: vec![
                DisableCondition::File { in_file: "missing".to_string(), key: "my_key".to_string() },
            ]}.check(&CanonPath::new(base_path).unwrap(), &ManifestConditions::default()).unwrap(),
            DisableOutCome::No(Some("File 'missing' does not exist".to_string())),
        )
    }

    #[test]
    fn evaluate_all_condition_one_false_and_one_true() {
        let base_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test/disable");

        assert_eq!(
            DisableCondition::All { all: vec![
                DisableCondition::File { in_file: "missing".to_string(), key: "my_key".to_string() },
                DisableCondition::Because { because: "no reason".to_string() },
            ]}.check(&CanonPath::new(base_path).unwrap(), &ManifestConditions::default()).unwrap(),
            DisableOutCome::No(Some("File 'missing' does not exist".to_string())),
        )
    }

    #[test]
    fn evaluate_all_condition_one_true_and_one_false() {
        let base_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test/disable");

        assert_eq!(
            DisableCondition::All { all: vec![
                DisableCondition::Because { because: "no reason".to_string() },
                DisableCondition::File { in_file: "missing".to_string(), key: "my_key".to_string() },
            ]}.check(&CanonPath::new(base_path).unwrap(), &ManifestConditions::default()).unwrap(),
            DisableOutCome::No(Some("File 'missing' does not exist".to_string())),
        )
    }

    #[test]
    fn evaluate_not_condition_sub_condition_is_true() {
        let not_condition = DisableCondition::Not {
            not: Box::new(DisableCondition::Because { because: "no reason".to_string() })
        };
        assert!(
            not_condition.check(&CanonPath::new("").unwrap(), &ManifestConditions::default()).unwrap()
                        .is_no()
        )
    }

    #[test]
    fn evaluate_not_condition_sub_condition_is_false() {
        let base_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test/disable");

        let not_condition = DisableCondition::Not {
            not: Box::new(DisableCondition::File { in_file: "missing".to_string(), key: "my_key".to_string() })
        };

        assert!(
            not_condition.check(&CanonPath::new(base_path).unwrap(), &ManifestConditions::default()).unwrap()
                .is_yes()
        )
    }

    #[test]
    fn evaluate_global_condition_disabled() {
        let condition = DisableCondition::ManifestCondition { manifest_condition: "my_global_var".to_string() };
        let manifest_conditions = ManifestConditions::new(HashMap::from([
            ("my_global_var".to_string(), DisableCondition::Because { because: "it doesn't suit me".to_string() }),
        ]));
        assert_eq!(
            condition.check(&CanonPath::new("").unwrap(), &manifest_conditions).unwrap(),
            DisableOutCome::Yes("it doesn't suit me".to_string()),
        )
    }

    #[test]
    fn evaluate_global_condition_enabled() {
        let condition = DisableCondition::ManifestCondition { manifest_condition: "my_global_var".to_string() };
        let manifest_conditions = ManifestConditions::new(HashMap::from([
            (
                "my_global_var".to_string(),
                DisableCondition::Not { not: Box::new(DisableCondition::Because { because: "it doesn't suit me".to_string() }) },
            ),
        ]));
        assert_eq!(
            condition.check(&CanonPath::new("").unwrap(), &manifest_conditions).unwrap(),
            DisableOutCome::No(Some("Negation of condition:\n  disabled because 'it doesn't suit me'".to_string())),
        )
    }

    #[test]
    fn evaluate_global_condition_absent() {
        let condition = DisableCondition::ManifestCondition { manifest_condition: "my_global_var".to_string() };
        let manifest_conditions = ManifestConditions::new(HashMap::from([
            ("some_other_var".to_string(), DisableCondition::Because { because: "it doesn't suit me".to_string() }),
        ]));
        assert_eq!(
            condition.check(&CanonPath::new("").unwrap(), &manifest_conditions).unwrap(),
            DisableOutCome::No(Some("manifest global condition my_global_var is not present".to_string())),
        )
    }
}
