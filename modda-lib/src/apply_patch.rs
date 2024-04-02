
use std::borrow::Cow;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use log::{debug, info, warn};
use patch::{Patch, Line};

use crate::args::Install;
use crate::canon_path::CanonPath;
use crate::lowercase::LwcString;
use crate::patch_source::{PatchDesc, PatchEncoding, PatchSource};

pub async fn patch_module(game_dir: &CanonPath, module_name: &LwcString, patch_loc: &Option<PatchDesc>, opts: &Install) -> Result<()> {
    match patch_loc {
        None => Ok(()),
        Some(patch) => {
            info!("mod {} needs patching", module_name);
            let patch_content = match &patch.patch_source {
                PatchSource::Http { http: _http } => { bail!("not implemented yet - patch from source {:?}", patch); }
                PatchSource::Relative { relative } => {
                    let diff = match read_patch_relative(relative, game_dir, opts, patch.encoding) {
                        Ok(diff) => diff,
                        Err(error) => bail!("Error reading relative patch at {} for {}\n -> {:?}",
                                                relative, module_name, error),
                    };
                    Cow::Owned(diff)
                }
                PatchSource::Inline { inline } => Cow::Borrowed(inline),
            };
            patch_module_with_content(game_dir, module_name, &*patch_content, patch.encoding)
        }
    }
}

fn patch_module_with_content(game_dir: &CanonPath, module_name: &LwcString, patch: &str, encoding: PatchEncoding) -> Result<()> {
    let diff = match Patch::from_multiple(&patch) {
        Ok(diff) => diff,
        Err(error) => bail!("Couldn't parse patch for module {}\n -> {:?}", module_name, error),
    };
    for patch in diff {
        let old_path = game_dir.join(&*patch.old.path);
        let old = match old_path {
            Ok(path) => path,
            Err(ref error) => bail!("Failed to canonicalize old file path {:?} while patching mod {}\n -> {:?}",
                                            old_path, module_name, error),
        };
        check_path(game_dir, &old)?;

        let new_path = game_dir.join(&*patch.new.path);
        let new = match new_path {
            Ok(path) => path,
            Err(ref error) => bail!("Failed to canonicalize new file path {:?} while patching mod {}\n -> {:?}",
                                            new_path, module_name, error),
        };
        check_path(game_dir, &new)?;
        if let Err(error) = patch_files(&old, &new, &patch, encoding) {
            bail!("Failed to patch file {:?} for mod {}\n -> {:?}", old, module_name, error);
        }
    }
    Ok(())
}

fn check_path(base: &CanonPath, path: &CanonPath) -> Result<()> {
    if !path.starts_with(base.path()) {
        bail!("Attempt to patch file not in game directory\n  {:?} not in {:?}", path, base);
    }
    Ok(())
}

fn decode_file(path:&CanonPath, encoding: PatchEncoding) -> Result<String> {
    let bytes = match std::fs::read(&path) {
        Ok(content) => content,
        Err(error) => bail!("Failed to read patch file {:?}\n -> {:?}", path, error),
    };
    let decoded = encoding.decode(&bytes);
    if decoded.2 {
        warn!("There were some encoding errors when decoding file {:?} with encoding {:?}",
                path, encoding);
                info!("=>\n{}", &*decoded.0);
    }
    Ok(decoded.0.into_owned())
}

fn patch_files(old: &CanonPath, new: &CanonPath, diff: &Patch, encoding: PatchEncoding) -> Result<()> {
    let old_content = get_old_content(old, encoding)?;

    let old_lines: Vec<String> = old_content.split("\n").map(From::from).collect();

    let new_lines = match apply_patch(&old_lines, diff) {
        Err(error) => bail!("Error patching file {:?}\n -> {:?}", old, error),
        Ok(new_lines) => new_lines,
    };

    let save_old_path = crate::utils::pathext::append_extension("old", &old.to_path_buf());
    if let Err(error) = std::fs::write(&save_old_path, old_lines.join("\n")) {
        bail!("Error saving old file to {:?}\n -> {:?}", save_old_path, error);
    }
    if old != new {
        std::fs::remove_file(old)?;
    }
    if let Err(error) = std::fs::write(new, new_lines.join("\n")) {
        bail!("Error writing new file to {:?}\n -> {:?}", new, error);
    }
    Ok(())
}

fn get_old_content(old: &CanonPath, encoding: PatchEncoding) -> Result<String> {
    if old.path() == PathBuf::from("/dev/null") {
        Ok("".to_string())
    } else {
        match decode_file(old, encoding) {
            Ok(lines) => Ok(lines),
            Err(error) => bail!("Error decoding file {:?}\n -> {:?}", old, error),
        }
    }
}

fn apply_patch<'a>(old_lines: &'a[String], diff: &'a Patch) -> Result<Vec<&'a str>> {
    let mut new_lines = vec![];
    let mut old_line = 0;
    for (idx, hunk) in diff.hunks.iter().enumerate() {
        info!("apply hunk {} of {}", idx + 1, diff.hunks.len());
        debug!("hunk {}", hunk);
        while old_line + 1 < hunk.old_range.start {
            new_lines.push(old_lines[old_line as usize].as_str());
            old_line += 1;
        }
        for line in &hunk.lines {
            match line {
                Line::Add(s)  => new_lines.push(s),
                Line::Context(s) => {
                    let context_line = &old_lines[old_line as usize];
                    if !str_equals_ignore_cr(context_line, s) {
                        bail!("patch hunk doesn't apply (actual context line '{:?}'):\n{}",
                                context_line.as_bytes(), hunk);
                    }
                    new_lines.push(s);
                    old_line += 1;
                }
                Line::Remove(s) => {
                    let context_line = &old_lines[old_line as usize];
                    if !str_equals_ignore_cr(context_line, s) {
                        bail!("patch hunk doesn't apply (actual deleted line '{:?}', expected '{:?}'):\n{}",
                                context_line.as_bytes(), s.as_bytes(), hunk);
                    }
                    old_line += 1;
                }
            }
        }
        debug!("at the end of the hunk, old_line is {}", old_line);
    }
    for line in old_lines.get((old_line as usize)..).unwrap_or(&[]) {
        new_lines.push(line);
    }
    match old_lines.last() {
        Some(s) if *s == "\n" => new_lines.push(""),
        _ => {}
    }
    Ok(new_lines)
}


fn read_patch_relative(relative: &str, game_dir: &CanonPath, opts: &Install, encoding: PatchEncoding) -> Result<String> {
    let relative_path = PathBuf::from(relative);
    if !relative_path.is_relative() {
        bail!("path is not relative: {:?}", relative);
    }
    match PathBuf::from(&opts.manifest_path).parent() {
        None => info!("Couldn't get manifest file parent - continue search with other locations"),
        Some(parent) => {
            let parent = match CanonPath::new(parent) {
                Ok(parent) => parent,
                Err(error) => bail!("failed to canonalize manifest parent\n -> {:?}", error),
            };
            if let Ok(diff) = read_patch_from(relative_path.as_path(), &parent, encoding) {
                return Ok(diff);
            }
        }
    }
    match read_patch_from(&relative_path, game_dir, encoding) {
        Ok(diff) => Ok(diff),
        Err(_error) => bail!("Couldn't find relative patch file {}", relative),
    }
}

fn read_patch_from(relative: &Path, base: &CanonPath, encoding: PatchEncoding) -> Result<String> {
    let complete = base.join(relative);
    if let Ok(path) = complete {
        if path.starts_with(base) {
            decode_file(&path, encoding)
        } else {
            bail!("Relative patch not in expected location")
        }
    } else {
        bail!("Could not canonalize path {:?}", complete);
    }
}

const CR: char = 13 as char;
const LF: char = 10 as char;

fn str_equals_ignore_cr(s1: &str, s2: &str) -> bool {
    let ignored: &[_] = &[CR, LF];
    return (s1 == s2)
            || (s1.trim_end_matches(ignored) == s2.trim_end_matches(ignored));
}

#[cfg(test)]
mod apply_patch_tests {
    use std::{io::BufRead, path::Path};
    use anyhow::{Result, bail};
    use indoc::indoc;
    use patch::Patch;

    use crate::apply_patch::apply_patch;
    use crate::lowercase::lwc;
    use crate::utils::read_all::read_all;

    const SIMPLEST_PATCH: &str = indoc!(r#"
        --- modulename.tp2
        +++ modulename.tp2
        @@ -1,6 +1,6 @@
         BACKUP ~weidu_external/backup/modulename~
         SUPPORT ~http://somewhere.iflucky.org~
        -VERSION ~1.0~
        +VERSION ~2.0~
         //languages
         LANGUAGE ~English~
                 ~english~
    "#);

    const PATCH_WITH_DELETE: &str = indoc!(r#"
        --- modulename.tp2
        +++ modulename.tp2
        @@ -1,6 +1,6 @@
         BACKUP ~weidu_external/backup/modulename~
         SUPPORT ~http://somewhere.iflucky.org~
        -VERSION ~1.0~
         //languages
         LANGUAGE ~English~
                 ~english~
    "#);

    const PATCH_WITH_ADD: &str = indoc!(r#"
        --- modulename.tp2
        +++ modulename.tp2
        @@ -1,6 +1,6 @@
         BACKUP ~weidu_external/backup/modulename~
         SUPPORT ~http://somewhere.iflucky.org~
        -VERSION ~1.0~
        +VERSION ~2.0~
         //languages
        +//more comment
         LANGUAGE ~English~
                 ~english~
    "#);

    const PATCH_WITH_A_B_PREFIXES: &str = indoc!(r#"
        --- a/modulename.tp2
        +++ b/modulename.tp2
        @@ -1,6 +1,6 @@
         BACKUP ~weidu_external/backup/modulename~
         SUPPORT ~http://somewhere.iflucky.org~
        -VERSION ~1.0~
        +VERSION ~2.0~
         //languages
        +//more comment
         LANGUAGE ~English~
                 ~english~
    "#);


    const PATCH_WITH_UNMODIFIED_EMPTY_LINE: &str = indoc!(r#"
        --- modulename.tp2
        +++ modulename.tp2
        @@ -1,6 +1,6 @@
         BACKUP ~weidu_external/backup/modulename~
         SUPPORT ~http://somewhere.iflucky.org~
        -VERSION ~1.0~
        +VERSION ~2.0~
         //languages
         LANGUAGE ~English~
                 ~english~
    "#);

    const PATCH_WITH_INCORRECT_CONTEXT_LINE: &str = indoc!(r#"
        --- modulename.tp2
        +++ modulename.tp2
        @@ -1,6 +1,6 @@
         BACKUP ~weidu_external/backup/modulename~
         SUPPORT ~http://somewhere.iflucky.com~
        -VERSION ~1.0~
        +VERSION ~2.0~
         //languages
         LANGUAGE ~English~
                 ~english~
    "#);

    const PATCH_WITH_INCORRECT_DELETED_LINE: &str = indoc!(r#"
        --- modulename.tp2
        +++ modulename.tp2
        @@ -1,6 +1,6 @@
         BACKUP ~weidu_external/backup/modulename~
         SUPPORT ~http://somewhere.iflucky.org~
        -VERSION ~1.1~
        +VERSION ~2.0~
         //languages
         LANGUAGE ~English~
                 ~english~
    "#);

    const PATCH_WITH_NEW_FILE: &str = indoc!(r#"
        --- /dev/null
        +++ french.tra
        @@ -0,0 +1 @@
        +@1 = ~héhé~
    "#);

    fn setup_test_game_dir() -> (tempfile::TempDir, crate::canon_path::CanonPath) {
        let tempdir = tempfile::tempdir().unwrap();
        let test_game_dir = tempdir.path().join("game");
        std::fs::create_dir_all(&test_game_dir).unwrap();
        (tempdir, crate::canon_path::CanonPath::new(test_game_dir).unwrap())
    }

    #[test]
    fn apply_simple_patch() {
        let origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/patch/modulename.tp2");
        let old = read_all(&origin).unwrap();
        let patch = Patch::from_single(SIMPLEST_PATCH).unwrap();
        let result = apply_patch(&old, &patch);

        let patched_origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/patch/modulename_patched.tp2");
        let expected = read_all(&patched_origin).unwrap();

        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn apply_delete_patch() {
        let origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/patch/modulename.tp2");
        let old = read_all(&origin).unwrap();
        let patch = Patch::from_single(PATCH_WITH_DELETE).unwrap();
        let result = apply_patch(&old, &patch);

        let patched_origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/patch/modulename_delete.tp2");
        let expected = read_all(&patched_origin).unwrap();

        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn apply_add_patch() {
        let origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/patch/modulename.tp2");
        let old = read_all(&origin).unwrap();
        let patch = Patch::from_single(PATCH_WITH_ADD).unwrap();
        let result = apply_patch(&old, &patch);

        let patched_origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/patch/modulename_add.tp2");
        let expected = read_all(&patched_origin).unwrap();

        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn apply_patch_with_unmodified_empty_line() {
        let origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/patch/modulename.tp2");
        let old = read_all(&origin).unwrap();
        let patch = Patch::from_single(PATCH_WITH_UNMODIFIED_EMPTY_LINE).unwrap();
        let result = apply_patch(&old, &patch);

        let patched_origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/patch/modulename_patched.tp2");
        let expected = read_all(&patched_origin).unwrap();

        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn apply_patch_failure_in_context() {
        let origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/patch/modulename.tp2");
        let old = read_all(&origin).unwrap();
        let patch = Patch::from_single(PATCH_WITH_INCORRECT_CONTEXT_LINE).unwrap();
        let result = apply_patch(&old, &patch);

        println!("{:?}", result);

        assert!(result.is_err());
    }

    #[test]
    fn apply_patch_failure_in_delete() {
        let origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/patch/modulename.tp2");
        let old = read_all(&origin).unwrap();
        let patch = Patch::from_single(PATCH_WITH_INCORRECT_DELETED_LINE).unwrap();
        let result = apply_patch(&old, &patch);

        println!("{:?}", result);

        assert!(result.is_err());
    }

    #[test]
    fn simple_patch_on_files() {
        let (_tempdir, game_dir) = setup_test_game_dir();
        let origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/patch/modulename.tp2");
        std::fs::copy(&origin, &game_dir.join_path("modulename.tp2")).unwrap();

        super::patch_module_with_content(&game_dir, &lwc!("modulename"), SIMPLEST_PATCH,
                                    crate::patch_source::PatchEncoding::UTF8).unwrap();

        // file modulename.tp2.old must exist and contain OLD content
        let dot_old_file = game_dir.join_path("modulename.tp2.old");
        let dot_old_content = read_all(&dot_old_file).unwrap().join("\n");
        let expected = read_all(&origin).unwrap().join("\n");
        assert_eq!(dot_old_content, expected);
        let new_file = game_dir.join_path("modulename.tp2");

        // file modulename.tp2 must exist and contain NEW content
        let new_content = read_all(&new_file).unwrap().join("\n");
        let patched_origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/patch/modulename_patched.tp2");
        let expected = read_all(&patched_origin).unwrap().join("\n");
        assert_eq!(new_content, expected);
    }

    #[test]
    fn apply_add_patch_create_file() {
        let old = vec![];
        let patch = Patch::from_single(PATCH_WITH_NEW_FILE).unwrap();
        let result = apply_patch(&old, &patch);

        assert_eq!(result.unwrap(), vec!["@1 = ~héhé~"]);
    }

    #[test]
    fn apply_patch_with_a_b_prefixes() {
        let origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/patch/modulename.tp2");
        let old = read_all(&origin).unwrap();
        let patch = Patch::from_single(PATCH_WITH_A_B_PREFIXES).unwrap();
        let result = apply_patch(&old, &patch);

        let patched_origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/patch/modulename_add.tp2");
        let expected = read_all(&patched_origin).unwrap();

        assert_eq!(result.unwrap(), expected);
    }
}
