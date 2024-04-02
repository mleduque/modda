
use std::{borrow::Cow, fs::{copy, rename, File, OpenOptions}, io::{Read, Write}, path::PathBuf};

use anyhow::{Result, bail};

use globwalk::{GlobWalker, GlobWalkerBuilder};
use log::{debug, error, info, warn};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::utils::pathext::append_extension;

#[derive(Deserialize, Serialize, Debug, PartialEq, Default, Clone)]
pub struct ReplaceSpec {
    /// List of 'globs' that describe files to process _inside the mod root directory_.
    /// https://git-scm.com/docs/gitignore#_pattern_format
    pub file_globs: Vec<String>,
    /// A regexp (syntax: https://docs.rs/regex/latest/regex/#syntax)
    /// Assumes UTF-8 content.
    pub replace: String,
    /// the replacement string (may use capture group as positional/integer or named capture group)
    pub with: String,
    pub max_depth: Option<usize>,
    #[serde(default)]
    pub string: bool,
}

impl ReplaceSpec {
    fn find_matching_files(&self, root: &PathBuf) -> Result<GlobWalker> {
        let walker = GlobWalkerBuilder::from_patterns(root, &self.file_globs)
            .case_insensitive(true)
            .max_depth(self.max_depth.unwrap_or(0))
            .file_type(globwalk::FileType::FILE);

        let walker = match self.max_depth {
            None => walker,
            Some(depth) => walker.max_depth(depth),
        };

        Ok(walker.build()?)
    }

    fn apply_replace(&self, file_path: &PathBuf, regex: &Regex) -> Result<String> {
        let mut file = match File::open(file_path) {
            Err(err) => bail!("apply_replace - fail to open old file {:?} - {}", file_path, err),
            Ok(file) => file,
        };
        let mut buf = vec![];
        if let Err(err) = file.read_to_end(&mut buf) {
            bail!("apply_replace: could not read content of file {:?} - {}", file, err)
        }

        let content = match String::from_utf8(buf) {
            Err(err)  => bail!("apply_replace: content of {:?} does not appear to be UTF8 - {}", file, err),
            Ok(what) => what,
        };
        Ok(regex.replace_all(&content, &self.with).to_string())
    }

    /// Replaces the content of `<file_name>` with the new content
    /// `<file_name>` is renamed to `file_name>.replaced`
    /// a new `<file_name>` file is created with the new content inside.
    fn swap_file_content(&self, file_path: &PathBuf, new_content: &str) -> Result<()> {
        let new_file_path = append_extension("new", file_path);
        debug!("swap_file_content will write new version in temporary file {:?}", new_file_path);
        let mut new_file = match OpenOptions::new().create(true).write(true).truncate(true).open(&new_file_path) {
            Ok(file) => file,
            Err(err) => bail!("apply_replace: could not create temp file - {}", err),
        };
        if let Err(err) = write!(new_file, "{}", new_content) {
            bail!("apply_replace: could not write new data to temp file - {}", err);
        }
        if let Err(err) = new_file.flush() {
            bail!("apply_replace: could not flush new file - {}", err);
        }
        let replaced_file_path = append_extension("replaced", file_path);
        debug!("swap_file_content will copy old version to {:?}", replaced_file_path);
        if let Err(err) = copy(file_path, &replaced_file_path) {
            bail!("apply_replace: could not rename old file to {:?}- {}", replaced_file_path, err);
        }
        debug!("swap_file_content will rename new (temp) version to old file name {:?}", file_path);
        if let Err(err) = rename(new_file_path, file_path) {
            bail!("apply_replace: could not rename new file to {:?} - {}", file_path, err);
        }

        Ok(())
    }

    pub fn exec(&self, root: &PathBuf) -> Result<()> {
        info!("ReplaceSpec.exec on {:?} - {} => {}", &self.file_globs, &self.replace, &self.with);
        let walker = self.find_matching_files(root)?;
        let pattern = if self.string {
            Cow::Owned(regex::escape(&self.replace))
        } else {
            Cow::Borrowed(&self.replace)
        };
        debug!("actual regex is {:?}", pattern);
        let regex = match Regex::new(&pattern) {
            Err(err) => bail!("Incorrect regex {} - {}", &self.replace, err),
            Ok(regex) => regex,
        };
        let result = self.do_exec(&regex, walker);
        match result {
            Err(ref err) => error!("ReplaceSpec.exec failure - {}", err),
            Ok(_) => info!("ReplaceSpec.exec success"),
        }
        result
    }

    fn do_exec(&self, regex: &Regex, walker: GlobWalker) -> Result<()> {
        for dir_entry in walker.into_iter().filter_map(Result::ok) {
            if dir_entry.file_type().is_file() {
                let file_path = dir_entry.into_path();
                debug!("process matching file {:?}", file_path);
                let new_content = self.apply_replace(&file_path, &regex)?;
                self.swap_file_content(&file_path, &new_content)?;
            } else {
                warn!("ReplaceSpec.exec - ignore matching file {:?}", dir_entry.path())
            }
        }
        Ok(())
    }
}


#[cfg(test)]
mod replace_tests {
    use std::path::{Path, PathBuf};

    use crate::utils::read_all::read_all;
    use super::ReplaceSpec;

    #[test]
    fn replace_regex() {
        let _ = env_logger::builder().is_test(true).filter_level(log::LevelFilter::Debug).try_init();

        let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let test_dir = project.join("target").join("replace").join("replace_regex");
        std::fs::create_dir_all(&test_dir).unwrap();

        let origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/replace/input.txt");

        let target_file_path = test_dir.join("input_regex.txt");
        std::fs::copy(origin, &target_file_path).unwrap();

        let replace_spec = ReplaceSpec {
            file_globs: vec!["input_regex.txt".to_string()],
            replace: "[A-Z]".to_string(),
            with: "11".to_string(),
            max_depth: Some(1),
            string: false,
        };
        replace_spec.exec(&test_dir).unwrap();

        let expected_origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/replace/expected_regex.txt");
        let expected = read_all(&expected_origin).unwrap().join("\n");

        let result = read_all(&target_file_path).unwrap().join("\n");

        assert_eq!(
            expected,
            result,
        )
    }

    #[test]
    fn replace_regex_with_captured() {
        let _ = env_logger::builder().is_test(true).filter_level(log::LevelFilter::Debug).try_init();

        let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let test_dir = project.join("target").join("replace").join("replace_regex_capture");
        std::fs::create_dir_all(&test_dir).unwrap();

        let origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/replace/input.txt");

        let target_file_path = test_dir.join("input_regex_capture.txt");
        std::fs::copy(origin, &target_file_path).unwrap();

        let replace_spec = ReplaceSpec {
            file_globs: vec!["input_regex_capture.txt".to_string()],
            replace: "(abc)".to_string(),
            with: "$1$1".to_string(),
            max_depth: Some(1),
            string: false,
        };
        replace_spec.exec(&test_dir).unwrap();

        let expected_origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/replace/expected_regex_capture.txt");
        let expected = read_all(&expected_origin).unwrap().join("\n");

        let result = read_all(&target_file_path).unwrap().join("\n");

        assert_eq!(
            expected,
            result,
        )
    }

    #[test]
    fn replace_no_regex() {
        let _ = env_logger::builder().is_test(true).filter_level(log::LevelFilter::Debug).try_init();

        let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let test_dir = project.join("target").join("replace").join("replace_no_regex");
        std::fs::create_dir_all(&test_dir).unwrap();

        let origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/replace/input.txt");

        let target_file_path = test_dir.join("input_no_regex.txt");
        std::fs::copy(origin, &target_file_path).unwrap();

        let replace_spec = ReplaceSpec {
            file_globs: vec!["input_no_regex.txt".to_string()],
            replace: "(abc)".to_string(),
            with: "[11]".to_string(),
            max_depth: Some(1),
            string: true,
        };
        replace_spec.exec(&test_dir).unwrap();

        let expected_origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/replace/expected_no_regex.txt");
        let expected = read_all(&expected_origin).unwrap().join("\n");

        let result = read_all(&target_file_path).unwrap().join("\n");

        assert_eq!(
            expected,
            result,
        )
    }
}
