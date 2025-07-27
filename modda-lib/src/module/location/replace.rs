
use std::{borrow::Cow, fs::{copy, rename, OpenOptions}, io::{Write}, path::PathBuf};

use anyhow::{Result, bail};

use globwalk::{GlobWalker, GlobWalkerBuilder};
use indoc::formatdoc;
use log::{debug, error, info, warn};
use regex::{Regex, Replacer};
use serde::{Deserialize, Serialize};

use crate::{apply_patch, obtain::get_options::{GetOptions, StrictReplaceAction}, patch_source::PatchEncoding, utils::pathext::append_extension};

use super::strict_replace::CheckReplace;

#[derive(Deserialize, Serialize, Debug, PartialEq, Default, Clone)]
pub struct ReplaceSpec {
    /// List of 'globs' that describe files to process _inside the mod root directory_.
    /// https://git-scm.com/docs/gitignore#_pattern_format
    pub file_globs: Vec<String>,
    /// A regexp or plain string to search and replace.
    /// Assumes UTF-8 content.
    pub replace: String,
    /// the replacement string (may use capture group as positional/integer or named capture group)
    pub with: String,
    /// If set, put a limit of the depth (from mod root) where the file to modify are found.
    pub max_depth: Option<usize>,
    /// If true, The ` replace` property is a regular expression<br>
    /// Syntax: <https://docs.rs/regex/latest/regex/#syntax><br>
    /// <https://regex101.com/> has a `rust` flavour.
    #[serde(default)]
    pub regex: bool,
    /// Tells to check something was actually replaced.<br>
    /// - If absent or `false` no check is done
    /// - if `true`` checks something was replaced (at least once)
    /// - if set to a (strict positive) integer, checks things were replaced _exactly_ this number of times.
    /// - if set to `>XXX` (for example `>123`) check there were more than XXX replacements done.
    #[serde(default)]
    pub check: CheckReplace,
    /// Encoding of the file to modify
    #[serde(default)]
    pub encoding: PatchEncoding,
}

impl ReplaceSpec {
    fn find_matching_files(&self, root: &PathBuf) -> Result<GlobWalker> {
        let walker = GlobWalkerBuilder::from_patterns(root, &self.file_globs)
            .case_insensitive(true)
            .file_type(globwalk::FileType::FILE);

        let walker = match self.max_depth {
            None => walker,
            Some(depth) => walker.max_depth(depth),
        };

        Ok(walker.build()?)
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

    pub fn exec(&self, root: &PathBuf, get_options: &GetOptions) -> Result<()> {
        info!("ReplaceSpec.exec on {:?} - '{}' => '{}' from {:?}", &self.file_globs, &self.replace, &self.with, root);
        let walker = self.find_matching_files(root)?;
        let pattern = if self.regex {
            Cow::Borrowed(&self.replace)
        } else {
            Cow::Owned(regex::escape(&self.replace))
        };
        debug!("actual regex is {:?}", pattern);
        let regex = match Regex::new(&pattern) {
            Err(err) => bail!("Incorrect regex {} - {}", &self.replace, err),
            Ok(regex) => regex,
        };
        let result = self.exec_with_walker(&regex, walker, get_options);
        match result {
            Err(ref err) => error!("ReplaceSpec.exec failure - {}", err),
            Ok(_) => info!("ReplaceSpec.exec success"),
        }
        result
    }

    fn exec_with_walker(&self, regex: &Regex, walker: GlobWalker, get_options: &GetOptions) -> Result<()> {
        let mut replace_count = 0;
        for dir_entry in walker.into_iter().filter_map(Result::ok) {
            if dir_entry.file_type().is_file() {
                let file_path = dir_entry.into_path();
                debug!("process matching file {:?}", file_path);
                let ReplaceResult { count, replaced } = self.apply_replace(&file_path, &regex)?;
                replace_count += count;
                self.swap_file_content(&file_path, &replaced)?;
            } else {
                warn!("ReplaceSpec.exec - ignore matching file {:?}", dir_entry.path())
            }
        }
        self.check_replace_condition(replace_count, get_options)
    }

    fn check_replace_condition(&self, replace_count: u32, get_options: &GetOptions) -> Result<()> {
        let upheld = match self.check {
            CheckReplace::BoolValue(false) => true,
            CheckReplace::BoolValue(true) => replace_count > 0,
            CheckReplace::Exact(exact) => replace_count == exact.get(),
            CheckReplace::MoreThan(value) => replace_count > value.get(),
        };

        use StrictReplaceAction::{Ignore, Ask, Fail};
        match (upheld, get_options.strict_replace) {
            (true, _) => Ok(()),
            (_, Ignore) => Ok(()),
            (false, Fail) => bail!("strict `replace` condition {:?} was not upheld (actual replacements: {replace_count}", self.check),
            (false, Ask) => {
                let prompt = formatdoc!(r#"
                Replace action has condition {:?} that was broken (actual count {})
                Do you which to ignore the issue and continue the installation?
                "#, self.check, replace_count);
                match dialoguer::Confirm::new().with_prompt(prompt).interact()? {
                    true => Ok(()),
                    false => bail!("User interrupted installation (reason: condition {:?} was not upheld (actual replacements: {replace_count}", self.check),
                }
            }
        }
    }

    fn apply_replace(&self, file_path: &PathBuf, regex: &Regex) -> Result<ReplaceResult> {
        let content = match apply_patch::decode_file(&file_path, self.encoding) {
            Ok(content) => content,
            Err(err) => bail!("apply_replace: could not read content of file {file_path:?}\n  {err}")
        };
        self.apply_replace_content(regex, &content)
    }

    fn apply_replace_content(&self, regex: &Regex, content: &str) -> Result<ReplaceResult> {
        let mut counting_replacer = CountingReplacer { count: 0, with: self.with.to_owned() };
        let replaced = regex.replace_all(content, counting_replacer.by_ref()).to_string();
        Ok(ReplaceResult { replaced, count: counting_replacer.count })
    }
}

pub struct ReplaceResult {
    pub count: u32,
    pub replaced: String,
}

struct CountingReplacer{
    with: String,
    count: u32,
}

impl Replacer for CountingReplacer {
    fn replace_append(&mut self, caps: &regex::Captures<'_>, dst: &mut String) {
        self.count += 1;
        self.with.replace_append(caps, dst)
    }

    fn no_expansion<'r>(&'r mut self) -> Option<Cow<'r, str>> {
        // Needs to be None to force replace_all to call replace_append
        None
    }
}

#[cfg(test)]
mod replace_tests {
    use std::num::NonZeroU32;
    use std::path::{Path, PathBuf};

    use crate::module::location::replace::{ReplaceSpec, CheckReplace};
    use crate::obtain::get_options::{GetOptions, StrictReplaceAction};
    use crate::patch_source::PatchEncoding;
    use crate::utils::read_all::read_all;

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
            regex: true,
            check: CheckReplace::BoolValue(false),
            encoding: PatchEncoding::UTF8,
        };
        let get_options = GetOptions { strict_replace: StrictReplaceAction::Fail };
        replace_spec.exec(&test_dir, &get_options).unwrap();

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
            replace: r#"(abc)"#.to_string(),
            with: "$1$1".to_string(),
            max_depth: Some(1),
            regex: true,
            check: CheckReplace::BoolValue(false),
            encoding: PatchEncoding::UTF8,
        };
        let get_options = GetOptions { strict_replace: StrictReplaceAction::Fail };
        replace_spec.exec(&test_dir, &get_options).unwrap();

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
            regex: false,
            check: CheckReplace::BoolValue(false),
            encoding: PatchEncoding::UTF8,
        };
        let get_options = GetOptions { strict_replace: StrictReplaceAction::Fail };
        replace_spec.exec(&test_dir, &get_options).unwrap();

        let expected_origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/replace/expected_no_regex.txt");
        let expected = read_all(&expected_origin).unwrap().join("\n");

        let result = read_all(&target_file_path).unwrap().join("\n");

        assert_eq!(
            expected,
            result,
        )
    }

    #[test]
    fn replace_with_strict_true_succeeds() {
        let _ = env_logger::builder().is_test(true).filter_level(log::LevelFilter::Debug).try_init();

        let marker = "true_succeeds";
        let file_name = format!("{marker}.txt");

        let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let test_dir = project.join("target").join("replace_strict").join(marker);
        std::fs::create_dir_all(&test_dir).unwrap();

        let origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/replace/input.txt");

        let target_file_path = test_dir.join(&file_name);
        std::fs::copy(origin, &target_file_path).unwrap();

        let replace_spec = ReplaceSpec {
            file_globs: vec![file_name],
            replace: "(abc)".to_string(),
            with: "[11]".to_string(),
            max_depth: Some(1),
            regex: false,
            check: CheckReplace::BoolValue(true),
            encoding: PatchEncoding::UTF8,
        };
        let get_options = GetOptions { strict_replace: StrictReplaceAction::Fail };
        replace_spec.exec(&test_dir, &get_options).unwrap();

        let expected = "aaaaBBcc\n[11]def";

        let result = read_all(&target_file_path).unwrap().join("\n");

        assert_eq!(expected, result)
    }

    #[test]
    fn replace_with_strict_true_fails() {
        let _ = env_logger::builder().is_test(true).filter_level(log::LevelFilter::Debug).try_init();

        let marker = "true_fails";
        let file_name = format!("{marker}.txt");

        let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let test_dir = project.join("target").join("replace_strict").join(marker);
        std::fs::create_dir_all(&test_dir).unwrap();

        let origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/replace/input.txt");

        let target_file_path = test_dir.join(&file_name);
        std::fs::copy(origin, &target_file_path).unwrap();

        let replace_spec = ReplaceSpec {
            file_globs: vec![file_name],
            replace: "(aaa)".to_string(),
            with: "[11]".to_string(),
            max_depth: Some(1),
            regex: false,
            check: CheckReplace::BoolValue(true),
            encoding: PatchEncoding::UTF8,
        };
        let get_options = GetOptions { strict_replace: StrictReplaceAction::Fail };
        replace_spec.exec(&test_dir, &get_options).unwrap_err();
    }

    #[test]
    fn replace_with_strict_exact_succeeds() {
        let _ = env_logger::builder().is_test(true).filter_level(log::LevelFilter::Debug).try_init();

        let marker = "exact_succeeds";
        let file_name = format!("{marker}.txt");

        let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let test_dir = project.join("target").join("replace_strict").join(marker);
        std::fs::create_dir_all(&test_dir).unwrap();

        let origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/replace/input.txt");

        let target_file_path = test_dir.join(&file_name);
        std::fs::copy(origin, &target_file_path).unwrap();

        let replace_spec = ReplaceSpec {
            file_globs: vec![file_name],
            replace: "aa".to_string(),
            with: "[11]".to_string(),
            max_depth: Some(1),
            regex: false,
            check: CheckReplace::Exact(NonZeroU32::new(2u32).unwrap()),
            encoding: PatchEncoding::UTF8,
        };
        let get_options = GetOptions { strict_replace: StrictReplaceAction::Fail };
        replace_spec.exec(&test_dir, &get_options).unwrap();

        let expected = "[11][11]BBcc\n(abc)def";

        let result = read_all(&target_file_path).unwrap().join("\n");

        assert_eq!(expected, result)
    }

    #[test]
    fn replace_with_strict_exact_fails_not_enough() {
        let _ = env_logger::builder().is_test(true).filter_level(log::LevelFilter::Debug).try_init();

        let marker = "exact_fails_not_enough";
        let file_name = format!("{marker}.txt");

        let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let test_dir = project.join("target").join("replace_strict").join(marker);
        std::fs::create_dir_all(&test_dir).unwrap();

        let origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/replace/input.txt");

        let target_file_path = test_dir.join(&file_name);
        std::fs::copy(origin, &target_file_path).unwrap();

        let replace_spec = ReplaceSpec {
            file_globs: vec![file_name],
            replace: "aa".to_string(),
            with: "[11]".to_string(),
            max_depth: Some(1),
            regex: false,
            check: CheckReplace::Exact(NonZeroU32::new(3u32).unwrap()),
            encoding: PatchEncoding::UTF8,
        };
        let get_options = GetOptions { strict_replace: StrictReplaceAction::Fail };
        replace_spec.exec(&test_dir, &get_options).unwrap_err();
    }

    #[test]
    fn replace_with_strict_exact_fails_too_many() {
        let _ = env_logger::builder().is_test(true).filter_level(log::LevelFilter::Debug).try_init();

        let marker = "exact_fails_too_many";
        let file_name = format!("{marker}.txt");

        let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let test_dir = project.join("target").join("replace_strict").join(marker);
        std::fs::create_dir_all(&test_dir).unwrap();

        let origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/replace/input.txt");

        let target_file_path = test_dir.join(&file_name);
        std::fs::copy(origin, &target_file_path).unwrap();

        let replace_spec = ReplaceSpec {
            file_globs: vec![file_name],
            replace: "aa".to_string(),
            with: "[11]".to_string(),
            max_depth: Some(1),
            regex: false,
            check: CheckReplace::Exact(NonZeroU32::new(1u32).unwrap()),
            encoding: PatchEncoding::UTF8,
        };
        let get_options = GetOptions { strict_replace: StrictReplaceAction::Fail };
        replace_spec.exec(&test_dir, &get_options).unwrap_err();
    }

    #[test]
    fn replace_with_strict_more_than_succeeds() {
        let _ = env_logger::builder().is_test(true).filter_level(log::LevelFilter::Debug).try_init();

        let marker = "more_than_succeeds";
        let file_name = format!("{marker}.txt");

        let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let test_dir = project.join("target").join("replace_strict").join(marker);
        std::fs::create_dir_all(&test_dir).unwrap();

        let origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/replace/input.txt");

        let target_file_path = test_dir.join(&file_name);
        std::fs::copy(origin, &target_file_path).unwrap();

        let replace_spec = ReplaceSpec {
            file_globs: vec![file_name],
            replace: "c".to_string(),
            with: "[11]".to_string(),
            max_depth: Some(1),
            regex: false,
            check: CheckReplace::MoreThan(NonZeroU32::new(2u32).unwrap()),
            encoding: PatchEncoding::UTF8,
        };
        let get_options = GetOptions { strict_replace: StrictReplaceAction::Fail };
        replace_spec.exec(&test_dir, &get_options).unwrap();

        let expected = "aaaaBB[11][11]\n(ab[11])def";

        let result = read_all(&target_file_path).unwrap().join("\n");

        assert_eq!(expected, result)
    }

    #[test]
    fn replace_with_strict_more_than_fails() {
        let _ = env_logger::builder().is_test(true).filter_level(log::LevelFilter::Debug).try_init();

        let marker = "more_than_fails";
        let file_name = format!("{marker}.txt");

        let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let test_dir = project.join("target").join("replace_strict").join(marker);
        std::fs::create_dir_all(&test_dir).unwrap();

        let origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/replace/input.txt");

        let target_file_path = test_dir.join(&file_name);
        std::fs::copy(origin, &target_file_path).unwrap();

        let replace_spec = ReplaceSpec {
            file_globs: vec![file_name],
            replace: "c".to_string(),
            with: "[11]".to_string(),
            max_depth: Some(1),
            regex: false,
            check: CheckReplace::MoreThan(NonZeroU32::new(3u32).unwrap()),
            encoding: PatchEncoding::UTF8,
        };
        let get_options = GetOptions { strict_replace: StrictReplaceAction::Fail };
        replace_spec.exec(&test_dir, &get_options).unwrap_err();
    }
}
