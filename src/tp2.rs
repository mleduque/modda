

use std::path::PathBuf;

use anyhow::{anyhow, bail, Result};
use glob::{glob_with, MatchOptions};


/**
 * Given a module name, finds a matching path to a .tp2 file
 * can be any of
 * - ${module}/${module}.tp2
 * - ${module}/setup-${module}.tp2
 * - ${module}.tp2
 * - setup-${module}.tp2
 * with case-insensitive search.
 * Search is done in this order and ignores other matches when one is found.
 */
pub fn find_tp2(module_name: &str) -> Result<PathBuf> {
    if let Some(path) = check_glob_casefold(&format!("./{}/{}.tp2", module_name, module_name))? {
        return Ok(path);
    }
    if let Some(path) = check_glob_casefold(&format!("./{}/setup-{}.tp2", module_name, module_name))? {
        return Ok(path);
    }
    if let Some(path) = check_glob_casefold(&format!("./{}.tp2", module_name))? {
        return Ok(path);
    }
    if let Some(path) = check_glob_casefold(&format!("./setup-{}.tp2", module_name))? {
        return Ok(path);
    }
    bail!("tp2 file {}.tp2 not found", module_name)
}

pub fn find_tp2_str(module_name: &str) -> Result<String> {
    let tp2_path = find_tp2(module_name)?;
    match tp2_path.to_str() {
        Some(name) => Ok(name.to_owned()),
        None => Err(anyhow!("invalid tp2 filename for module {}", module_name)),
    }
}

fn check_glob_casefold(pattern: &str) -> Result<Option<PathBuf>> {
    println!("try {}", pattern);
    let options = MatchOptions {
        case_sensitive: false,
        ..Default::default()
    };
    let mut glob_result = glob_with(pattern, options)?;
    if let Some(path) = glob_result.find_map(|item| {
        match item {
            Err(_) => None,
            Ok(value) => Some(value),
        }
    }) {
        Ok(Some(path))
    } else {
        Ok(None)
    }
}
