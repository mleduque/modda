

use std::path::PathBuf;

use anyhow::{anyhow, bail, Result};
use globwalk::GlobWalkerBuilder;
use log::debug;

use crate::canon_path::CanonPath;
use crate::lowercase::{LwcString, lwc};

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
pub fn find_tp2(from_base: &CanonPath, module_name: &LwcString) -> Result<PathBuf> {
    match find_glob_casefold(from_base, module_name, 2) {
        Err(error) => bail!("Failed to search tp2 file for module {}\n -> {:?}", module_name, error),
        Ok(paths) => {
            if !paths.is_empty() {

                let short_tp2 = Some(lwc!(&format!("{module_name}.tp2")));
                let long_tp2 = Some(lwc!(&format!("setup-{module_name}.tp2")));
                let mod_dir = Some(module_name.clone());

                for path in paths {
                    let components = path.components().map(|component|
                        component.as_os_str().to_str().map(|os_str| lwc!(os_str))
                    ).collect::<Vec<_>>();
                    match &components[..] {
                        &[ref single] => if single == &short_tp2 || single == &long_tp2 { return Ok(path.clone()) },
                        &[ref first, ref second] =>
                                        if first == &mod_dir && (second == &short_tp2 || second == &long_tp2) {
                                            return Ok(path.clone())
                                        }
                        _ => {}
                    }
                }
                debug!("find_tp2, none of the candidates matched.");
            } else {
                debug!("find_tp2, no candidates found.");
            }
            bail!("No tp2 file for mod {}", module_name);
        }
    }
}

pub fn find_tp2_str(from_base: &CanonPath, module_name: &LwcString) -> Result<String> {
    let tp2_path = find_tp2(from_base, module_name)?;
    match tp2_path.to_str() {
        Some(name) => Ok(name.to_owned()),
        None => Err(anyhow!("invalid tp2 filename for module {}", module_name)),
    }
}

fn find_glob_casefold(from_base: &CanonPath, module_name: &LwcString, depth: usize) -> Result<Vec<PathBuf>> {
    debug!("search tp2 for module {} from {:?} depth={}", module_name, from_base, depth);
    let patterns = vec![
        format!("{module_name}.tp2"),
        format!("setup-{module_name}.tp2"),
        format!("{module_name}/{module_name}.tp2"),
        format!("{module_name}/setup-{module_name}.tp2"),
    ];
    let walker = match GlobWalkerBuilder::from_patterns(from_base, &patterns)
        .case_insensitive(true)
        .max_depth(depth)
        .build() {
            Ok(glob) => glob,
            Err(error) => bail!("Failed to build glob {:?}\n-> {:?}", patterns, error),
        }.into_iter()
        .filter_map(Result::ok);
    let mut result = vec![];
    for item in walker {
        debug!("find_glob_casefold got {:?}", item);
        result.push(item.path().strip_prefix(from_base).unwrap().to_owned())
    }

    debug!("find_glob_casefold result={:?}", result);
    Ok(result)
}


pub fn find_game_tp2(from_base: &CanonPath) -> Result<Vec<LwcString>> {
    debug!("search tp2 from {:?}", from_base);
    let patterns = vec![
        "*.tp2",
    ];
    let walker = match GlobWalkerBuilder::from_patterns(from_base, &patterns)
        .case_insensitive(true)
        .max_depth(2)
        .build() {
            Ok(glob) => glob,
            Err(error) => bail!("Failed to build glob {:?}\n-> {:?}", patterns, error),
        }.into_iter()
        .filter_map(Result::ok);

    let mut result = vec![];
    for item in walker {
        debug!("find_game_tp2 got {:?}", item);
        let stripped = match item.path().strip_prefix(from_base) {
            Err(error) => bail!("Could not strip path from  {:?}\n  {:?}", item.path(), error),
            Ok(stripped) => stripped
        };
        let stripped_str = match stripped.to_str(){
            None => bail!("Could not lowercase path for {:?}", item.path()),
            Some(path) => lwc!(path),
        };

        let name = match stripped.file_stem() {
            Some(name) => match name.to_str() {
                None => bail!("Could not lowercase file name for {:?}", item.path()),
                Some(name) => lwc!(name),
            }
            None => bail!("Could not determine file name for {:?}", item.path())
        };
        debug!("name is {name}, compare to {stripped_str}");
        if stripped_str == format!("{name}.tp2") || stripped_str == format!("setup-{name}.tp2")
            || stripped_str == format!("{name}/{name}.tp2") || stripped_str == format!("{name}/setup-{name}.tp2") {
                result.push(name)
            }
    }
    Ok(result)
}

#[test]
fn find_simplest_tp2_location() {
    use std::str::FromStr;
    let test_base = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), "resources/test/tp2");
    let base = CanonPath::new(PathBuf::from_str(&test_base).unwrap()).unwrap();
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
    let base = CanonPath::new(PathBuf::from_str(&test_base).unwrap()).unwrap();
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
    let base = CanonPath::new(PathBuf::from_str(&test_base).unwrap()).unwrap();
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
    let base = CanonPath::new(PathBuf::from_str(&test_base).unwrap()).unwrap();
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
    let base = CanonPath::new(PathBuf::from_str(&test_base).unwrap()).unwrap();
    let found = find_tp2(&base, &lwc!("someothermodule")).unwrap();
    assert_eq!(
        found,
        PathBuf::from_str("someOtherModule/Setup-someothermodule.tp2").unwrap()
    )
}
