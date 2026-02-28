
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use log::debug;

/// Looks for a file matching the argument, allowing case-insensitive matches.
///
/// - `base` is the path where we're looking for. It's an exact path.
/// - `searched` is a path from the `base`, potentially cas insensitive.
pub fn find_insensitive<P>(base: P, searched: &str) -> Result<Option<PathBuf>>
    where
        P: AsRef<Path> + std::fmt::Debug {
    let candidates = find_all_insensitive(&base, searched)?;
    match candidates[..] {
        [] => {
            debug!("Found no matches for {searched} in {base:?}");
            Ok(None)
        },
        [ref name] => Ok(Some(PathBuf::from(name))),
        _ => {
            let msg = format!("More than one candidate ({count}) for lookup of {searched:?} in {base:?}",
                                    count = candidates.len());
            debug!("{msg}");
            bail!(msg)
        },
    }
}

pub fn find_all_insensitive<P>(base: P, searched: &str) -> Result<Vec<PathBuf>>
    where
        P: AsRef<Path> + std::fmt::Debug {

    match &base.as_ref().metadata() {
        Err(err) => match  err.kind() {
            ErrorKind::NotFound => bail!("Base directory {base:?} doesn't exist"),
            _ => bail!("Could not obtain fs metadata for {base:?} - {err:?}")
        }
        Ok(meta) if !meta.is_dir() => bail!("Base {base:?} is not a directory"),
        _ => {}
    }
    let as_pathbuf: PathBuf = PathBuf::from(searched);
    let searched_components = as_pathbuf.components().collect::<Vec<_>>();
    let partial_paths = searched_components.iter().take(searched_components.len() - 1)
        .try_fold(vec![], |mut acc: Vec<PathBuf>, curr| {
            let part = match curr {
                std::path::Component::Normal(part) => part,
                _ => bail!("Path component is not allowed: `{curr:?}` (in `{searched:?}`")
            };
            let next = match acc.last() {
                None => PathBuf::from(part),
                Some(last) => last.join(part),
            };
            acc.push(next);
            Ok(acc)
        })?;

    let partial_paths_as_str= partial_paths.iter().map(|path| {
        match path.as_os_str().to_str() {
            None => bail!("Unsupported character in searched path: `{path:?}` (in `{searched:?}`"),
            Some(path_string) => Ok(format!("^{}$", regex::escape(path_string))),
        }
    })
    .collect::<Result<Vec<_>>>()?;

    let partial_path_regexes: Vec<regex::bytes::Regex> = partial_paths_as_str.iter().map(|escaped| {
        Ok(regex::bytes::RegexBuilder::new(&escaped).case_insensitive(true).build()?)
    }).collect::<Result<Vec<_>>>()?;

    let full_regex_as_str =  format!("^{}$", regex::escape(searched));
    debug!("search (insensitive) for {searched} in {base:?}\n  escaped pattern is {full_regex_as_str}\n  parent patterns are\n    - {parents}",
            parents = partial_paths_as_str.join("\n    - "));
    let full_regex = regex::bytes::RegexBuilder::new(&full_regex_as_str).case_insensitive(true).build()?;

    let result = walkdir::WalkDir::new(&base)
        .min_depth(1) // root is depth 0
        .into_iter()
        .filter_entry(|dir_entry| {
            let path = dir_entry.path();
            match path.strip_prefix(&base) {
                Err(_) => false,
                Ok(stripped_path) => {
                    let path_as_bytes = stripped_path.as_os_str().as_encoded_bytes();
                    if path.is_dir() {
                        // allow descending in this directory if it matches partially
                        let accepted = partial_path_regexes.iter().any(|regex| regex.is_match(path_as_bytes))
                            || full_regex.is_match(path_as_bytes);
                        debug!("dir is [{stripped_path:?}] => {accepted} (descending)");
                        accepted
                    } else {
                        let accepted = full_regex.is_match(path_as_bytes);
                        debug!("path is [{stripped_path:?}] => {accepted}");
                        accepted
                    }
                }
            }
        })
        .filter_map(|entry| entry.ok())
        // remove parent directories
        .filter(|candidate| {
            let path = candidate.path();
            match path.strip_prefix(&base) {
                Err(_) => false,
                Ok(stripped_path) => full_regex.is_match(stripped_path.as_os_str().as_encoded_bytes())
            }
        })
        .map(|dir_entry| base.as_ref().join(dir_entry.into_path()))
        .collect::<Vec<_>>();
    Ok(result)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::{Path, PathBuf};

    use crate::utils::insensitive::{find_all_insensitive, find_insensitive};

    #[test]
    fn find_chitin_key_base_doesnt_exist_in_same_case() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/file_lookup/LOWER");
        println!("{:?}", find_insensitive(&base, "chitin.key"));
        assert!(find_insensitive(&base, "chitin.key").is_err());
    }

    #[test]
    fn find_chitin_key_lowercase_param_lower() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/file_lookup/lower");
        assert_eq!(find_insensitive(&base, "chitin.key").unwrap(), Some(PathBuf::from(base).join("chitin.key")));
    }

    #[test]
    fn find_chitin_key_lowercase_param_upper() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/file_lookup/lower");
        assert_eq!(find_insensitive(&base, "CHITIN.KEY").unwrap(), Some(PathBuf::from(base).join("chitin.key")));
    }

    #[test]
    fn find_chitin_key_lowercase_param_mixed() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/file_lookup/lower");
        assert_eq!(find_insensitive(&base, "Chitin.Key").unwrap(), Some(PathBuf::from(base).join("chitin.key")));
    }

    #[test]
    fn find_chitin_key_uppercase_param_lower() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/file_lookup/UPPER");
        assert_eq!(find_insensitive(&base, "chitin.key").unwrap(), Some(PathBuf::from(base).join("CHITIN.KEY")));
    }

    #[test]
    fn find_chitin_key_uppercase_param_upper() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/file_lookup/UPPER");
        assert_eq!(find_insensitive(&base, "CHITIN.KEY").unwrap(), Some(PathBuf::from(base).join("CHITIN.KEY")));
    }

    #[test]
    fn find_chitin_key_uppercase_param_mixed() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/file_lookup/UPPER");
        assert_eq!(find_insensitive(&base, "Chitin.Key").unwrap(), Some(PathBuf::from(base).join("CHITIN.KEY")));
    }

    #[test]
    fn find_chitin_key_mixed_case_param_lower() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/file_lookup/MiXed");
        assert_eq!(find_insensitive(&base, "chitin.key").unwrap(), Some(PathBuf::from(base).join("Chitin.Key")));
    }

    #[test]
    fn find_chitin_key_mixed_case_param_upper() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/file_lookup/MiXed");
        assert_eq!(find_insensitive(&base, "CHITIN.KEY").unwrap(), Some(PathBuf::from(base).join("Chitin.Key")));
    }

    #[test]
    fn find_chitin_key_mixed_case_param_mixed() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/file_lookup/MiXed");
        assert_eq!(find_insensitive(&base, "Chitin.Key").unwrap(), Some(PathBuf::from(base).join("Chitin.Key")));
    }

    #[test]
    fn find_with_subdir_lower_param_lower() {
        let _ = env_logger::builder().is_test(true).filter_level(log::LevelFilter::Debug).try_init();
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/file_lookup/lower");
        assert_eq!(find_insensitive(&base, "mod1/file1.txt").unwrap(), Some(PathBuf::from(base).join("mod1").join("file1.txt")));
    }

    #[test]
    fn find_with_subdir_lower_param_upper() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/file_lookup/lower");
        assert_eq!(find_insensitive(&base, "MOD1/FILE1.TXT").unwrap(), Some(PathBuf::from(base).join("mod1").join("file1.txt")));
    }

    #[test]
    fn find_with_subdir_lower_param_mixed() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/file_lookup/lower");
        assert_eq!(find_insensitive(&base, "Mod1/File1.Txt").unwrap(), Some(PathBuf::from(base).join("mod1").join("file1.txt")));
    }

    #[test]
    fn find_with_subdir_upper_param_lower() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/file_lookup/UPPER");
        assert_eq!(find_insensitive(&base, "mod1/file1.txt").unwrap(), Some(PathBuf::from(base).join("MOD1").join("FILE1.TXT")));
    }

    #[test]
    fn find_with_subdir_mixed_param_lower() {
        let _ = env_logger::builder().is_test(true).filter_level(log::LevelFilter::Debug).try_init();
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/file_lookup/MiXed");
        assert_eq!(find_insensitive(&base, "mod1/file1.txt").unwrap(), Some(PathBuf::from(base).join("Mod1").join("File1.txt")));
    }

    #[test]
    fn find_all_multiple_with_different_case() {
        let _ = env_logger::builder().is_test(true).filter_level(log::LevelFilter::Debug).try_init();
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/file_lookup/multiple");
        assert_eq!(
            find_all_insensitive(&base, "file2.txt").unwrap(),
            vec![
                PathBuf::from(&base).join("File2.txt"),
                PathBuf::from(&base).join("File2.Txt"),
            ],
        );
    }

    #[test]
    fn find_multiple_with_different_case_and_folders() {
        let _ = env_logger::builder().is_test(true).filter_level(log::LevelFilter::Debug).try_init();
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/file_lookup/multiple");
        assert!(find_insensitive(&base, "mod1/file1").is_err());
    }

    #[test]
    fn find_all_multiple_with_different_case_and_folders() {
        let _ = env_logger::builder().is_test(true).filter_level(log::LevelFilter::Debug).try_init();
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/file_lookup/multiple");
        assert_eq!(
            find_all_insensitive(&base, "mod1/file1").unwrap().iter().collect::<HashSet<_>>(),
            vec![
                PathBuf::from(&base).join("Mod1").join("File1"),
                PathBuf::from(&base).join("mod1").join("file1"),
            ].iter().collect::<HashSet<_>>(),
        );
    }

    #[test]
    fn find_directory_with_mismatched_case() {
        let _ = env_logger::builder().is_test(true).filter_level(log::LevelFilter::Debug).try_init();
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/file_lookup/UPPER");
        assert_eq!(find_insensitive(&base, "mod1").unwrap(), Some(PathBuf::from(base).join("MOD1")));
    }

    #[test]
    fn find_multiple_directories() {
        let _ = env_logger::builder().is_test(true).filter_level(log::LevelFilter::Debug).try_init();
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/file_lookup/multiple");
        assert_eq!(
            find_all_insensitive(&base, "mod1").unwrap().iter().collect::<HashSet<_>>(),
            vec![
                PathBuf::from(&base).join("Mod1"),
                PathBuf::from(&base).join("mod1"),
            ].iter().collect::<HashSet<_>>(),
        );
    }

    #[test]
    fn find_file_and_dir_with_same_name_except_case() {
        let _ = env_logger::builder().is_test(true).filter_level(log::LevelFilter::Debug).try_init();
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/file_lookup/multiple");
        assert_eq!(
            find_all_insensitive(&base, "mod2/file").unwrap().iter().collect::<HashSet<_>>(),
            vec![
                PathBuf::from(&base).join("mod2").join("file"),
                PathBuf::from(&base).join("mod2").join("File"),
            ].iter().collect::<HashSet<_>>(),
        );
    }
}
