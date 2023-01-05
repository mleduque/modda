
use std::{path::PathBuf, fs::{File, OpenOptions, rename, copy, FileType}, io::{Read, Seek, Write}};

use anyhow::{Result, bail};
use chardetng::EncodingDetector;
use encoding_rs::UTF_8;
use globwalk::{GlobWalker, GlobWalkerBuilder};
use log::{info, error, warn};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::pathext::append_extension;



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
    pub max_depth: usize,
}

impl ReplaceSpec {
    fn find_matching_files(&self, root: &PathBuf) -> Result<GlobWalker> {
        let walker = GlobWalkerBuilder::from_patterns(root, &self.file_globs)
        .case_insensitive(true)
            .max_depth(self.max_depth)
            .file_type(globwalk::FileType::FILE)
            .build()?;
        Ok(walker)
    }

    fn apply_replace(&self, file_path: &PathBuf, regex: &Regex) -> Result<String> {
        let mut file = match File::open(file_path) {
            Err(err) => bail!("apply_replace - fail to open old file {:?} - {}", file_path, err),
            Ok(file) => file,
        };
        let mut buf = vec![];
        if let Err(err) = file.read(&mut buf) {
            bail!("apply_replace: could not read content of file {:?} - {}", file, err)
        }

        let content = match String::from_utf8(buf) {
            Err(err)  => bail!("apply_replace: content of {:?} doesnot appear to be UTF8 - {}", file, err),
            Ok(what) => what,
        };
        Ok(regex.replace_all(&content, &self.with).to_string())
    }

    /// Replaces the content of `<file_name>` with the new content
    /// `<file_name>` is renamed to `file_name>.replaced`
    /// a new `<file_name>` file is created with the new content inside.
    fn swap_file_content(&self, file_path: &PathBuf, new_content: &str) -> Result<()> {
        let new_file_path = append_extension("new", file_path);
        let mut new_file = match OpenOptions::new().truncate(true).open(&new_file_path) {
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
        if let Err(err) = copy(file_path, &replaced_file_path) {
            bail!("apply_replace: could not rename old file to {:?}- {}", replaced_file_path, err);
        }
        if let Err(err) = rename(new_file_path, file_path) {
            bail!("apply_replace: could not rename new file to {:?} - {}", file_path, err);
        }

        Ok(())
    }
    pub fn exec(&self, root: &PathBuf) -> Result<()> {
        info!("ReplaceSpec.exec on {:?} - {} => {}", &self.file_globs, &self.replace, &self.with);
        let walker = self.find_matching_files(root)?;
        let regex = match Regex::new(&self.replace) {
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
                    let new_content = self.apply_replace(&file_path, &regex)?;
                    self.swap_file_content(&file_path, &new_content)?;
            } else {
                warn!("ReplaceSpec.exec - ignore matching file {:?}", dir_entry.path())
            }
        }
        Ok(())
    }
}
