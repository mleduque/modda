

use std::path::PathBuf;

use anyhow::{anyhow, bail, Result};
use globwalk::{GlobWalkerBuilder};
use log::debug;

use crate::lowercase::{ContainsStr, LwcString, lwc};

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
pub fn find_tp2(from_base: &PathBuf, module_name: &LwcString) -> Result<PathBuf> {
    match find_glob_casefold(from_base, module_name, 2) {
        Err(error) => bail!("Failed to search tp2 file for module {}\n -> {:?}", module_name, error),
        Ok(paths) => {
            let lwc_paths = paths.iter().filter_map(|path| {
                path.as_os_str().to_str().map(|os_str| lwc!(os_str))
            }).collect::<Vec<_>>();
            let first = format!("{}/{}.tp2", module_name, module_name);
            if let Some(idx) = lwc_paths.find_str(&first) {
                return Ok(paths[idx].to_owned())
            }
            let second = format!("{}/setup-{}.tp2", module_name, module_name);
            if let Some(idx) = lwc_paths.find_str(&second) {
                return Ok(paths[idx].to_owned())
            }
            let third = format!("{}.tp2", module_name);
            if let Some(idx) = lwc_paths.find_str(&third) {
                return Ok(paths[idx].to_owned())
            }
            let last = format!("setup-{}.tp2", module_name);
            if let Some(idx) = lwc_paths.find_str(&last) {
                return Ok(paths[idx].to_owned())
            }
            bail!("No tp2 file for mod {}", module_name);
        }
    }
}

pub fn find_tp2_str(from_base: &PathBuf, module_name: &LwcString) -> Result<String> {
    let tp2_path = find_tp2(from_base, module_name)?;
    match tp2_path.to_str() {
        Some(name) => Ok(name.to_owned()),
        None => Err(anyhow!("invalid tp2 filename for module {}", module_name)),
    }
}

fn find_glob_casefold(from_base: &PathBuf, module_name: &LwcString, depth: usize) -> Result<Vec<PathBuf>> {
    debug!("search tp2 for module {} from {:?} depth={}", module_name, from_base, depth);
    let pattern =format!("**/*{module}.tp2",module = module_name);
    let walker = match GlobWalkerBuilder::new(from_base, &pattern)
        .case_insensitive(true)
        .max_depth(depth)
        .build() {
            Ok(glob) => glob,
            Err(error) => bail!("Failed to build glob {}\n-> {:?}", pattern, error),
        }.into_iter()
        .filter_map(Result::ok);
    let mut result = vec![];
    for item in walker {
        debug!("check_glob_casefold got {:?}", item);
        result.push(item.path().strip_prefix(from_base).unwrap().to_owned())
    }

    Ok(result)
}

#[test]
fn find_simplest_tp2_location() {
    use std::str::FromStr;
    let test_base = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), "resources/test/tp2");
    let base = PathBuf::from_str(&test_base).unwrap();
    let found = find_tp2(&base, &lwc!("simple")).unwrap();
    assert_eq!(
        found,
        PathBuf::from_str("simple.tp2").unwrap()
    )
}

#[test]
fn find_tp2_location_with_case_mismatch() {
    use std::str::FromStr;
    let test_base = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), "resources/test/tp2");
    let base = PathBuf::from_str(&test_base).unwrap();
    let found = find_tp2(&base, &lwc!("simplewithcase")).unwrap();
    assert_eq!(
        found,
        PathBuf::from_str("simpleWithCase.tp2").unwrap()
    )
}

#[test]
fn find_tp2_location_with_setup_prefix() {
    use std::str::FromStr;
    let test_base = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), "resources/test/tp2");
    let base = PathBuf::from_str(&test_base).unwrap();
    let found = find_tp2(&base, &lwc!("anotherModule")).unwrap();
    assert_eq!(
        found,
        PathBuf::from_str("setup-anotherModule.tp2").unwrap()
    )
}

#[test]
fn find_tp2_in_mod_subdir_simple() {
    use std::str::FromStr;
    let test_base = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), "resources/test/tp2");
    let base = PathBuf::from_str(&test_base).unwrap();
    let found = find_tp2(&base, &lwc!("SomeModule")).unwrap();
    assert_eq!(
        found,
        PathBuf::from_str("someModule/someModule.tp2").unwrap()
    )
}

#[test]
fn find_tp2_in_mod_subdir_and_setup_prefix() {
    use std::str::FromStr;
    let test_base = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), "resources/test/tp2");
    let base = PathBuf::from_str(&test_base).unwrap();
    let found = find_tp2(&base, &lwc!("someothermodule")).unwrap();
    assert_eq!(
        found,
        PathBuf::from_str("someOtherModule/Setup-someothermodule.tp2").unwrap()
    )
}
