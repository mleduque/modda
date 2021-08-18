

use std::borrow::Cow;
use std::io::BufRead;
use std::path::{Path};

use anyhow::{bail, Result};
use patch::{Patch, Line};

use crate::patch_source::PatchSource;

pub async fn patch_module(game_dir: &Path, module_name: &str, patch_loc: &Option<PatchSource>) -> Result<()> {
    match patch_loc {
        None => Ok(()),
        Some(patch) => {
            let patch_content = match &patch {
                PatchSource::Http { http: _http } => { bail!("not implemented yet - patch from source {:?}", patch); }
                PatchSource::Inline { inline } => Cow::Borrowed(inline),
            };
            patch_module_with_content(game_dir, module_name, &*patch_content)
        }
    }
}

fn patch_module_with_content(game_dir: &Path, module_name: &str, patch: &str) -> Result<()> {
    let diff = match Patch::from_multiple(&patch) {
        Ok(diff) => diff,
        Err(error) => bail!("Couldn't parse patch for module {}\n -> {:?}", module_name, error),
    };
    for patch in diff {
        let old_path = game_dir.join(&*patch.old.path);
        let old = match old_path.canonicalize() {
            Ok(path) => path,
            Err(error) => bail!("Failed to canonicalize old file path {:?} while patching mod {}\n -> {:?}",
                                        old_path, module_name, error),
        };
        check_path(game_dir, &old)?;

        let new_path = game_dir.join(&*patch.new.path);
        let new = match new_path.canonicalize() {
            Ok(path) => path,
            Err(error) => bail!("Failed to canonicalize new file path {:?} while patching mod {}\n -> {:?}",
                                        new_path, module_name, error),
        };
        check_path(game_dir, &new)?;
        if let Err(error) = patch_files(&old, &new, &patch, module_name) {
            bail!("Failed to patch file {:?} for mod {}\n -> {:?}", old, module_name, error);
        }
    }
    Ok(())
}

fn check_path(base: &Path, path: &Path) -> Result<()> {
    if !path.starts_with(base) {
        bail!("Attempt to patch file not in game directory");
    }
    Ok(())
}

fn read_all(path: &Path) -> Result<Vec<String>> {
    let file = std::fs::File::open(path)?;
    let buf = std::io::BufReader::new(file);
    let mut lines = vec![];
    for line in buf.lines() {
        match line {
            Ok(line) => lines.push(line),
            Err(error) => bail!("Error reading file {:?}\n -> {:?}", path, error),
        }
    }
    Ok(lines)
}

fn patch_files(old: &Path, new: &Path, diff: &Patch, module_name: &str) -> Result<()> {
    let old_lines = match read_all(old) {
        Ok(lines) => lines,
        Err(error) => bail!("Error while patching {}\n -> {:?}", module_name, error),
    };

    let new_lines = apply_patch(&old_lines, diff);

    let save_old_path = crate::pathext::append_extension("old", old.to_path_buf());
    if let Err(error) = std::fs::write(&save_old_path, old_lines.join("\n")) {
        bail!("Error saving old file to {:?} (patching {})\n -> {:?}", save_old_path, module_name, error);
    }
    if old != new {
        std::fs::remove_file(old)?;
    }
    if let Err(error) = std::fs::write(new, new_lines.join("\n")) {
        bail!("Error writing new file to {:?} (patching {})\n -> {:?}", new, module_name, error);
    }
    Ok(())
}

fn apply_patch<'a>(old_lines: &'a[String], diff: &'a Patch) -> Vec<&'a str> {
    let mut new_lines = vec![];
    let mut old_line = 0;
    for hunk in &diff.hunks {
        //println!("hunk {:?}", hunk);
        while old_line < hunk.old_range.start - 1 {
            //println!("copy line {}", old_lines[old_line as usize]);
            new_lines.push(old_lines[old_line as usize].as_str());
            old_line += 1;
        }
        for line in &hunk.lines {
            match line {
                Line::Add(s)  => new_lines.push(s),
                Line::Context(s) => {
                    new_lines.push(s);
                    old_line += 1;
                }
                Line::Remove(_) => { old_line += 1; }
            }
        }
        //println!("at the end of the hunk, old_line is {}", old_line);
    }
    for line in old_lines.get((old_line as usize)..).unwrap_or(&[]) {
        new_lines.push(line);
    }
    match old_lines.last() {
        Some(s) if s == "\n" => new_lines.push(""),
        _ => {}
    }
    new_lines
}

#[cfg(test)]
mod apply_patch_tests {
    use std::path::Path;
    use indoc::indoc;
    use patch::Patch;

    use crate::apply_patch::apply_patch;

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

    fn setup_test_game_dir() -> (tempfile::TempDir, std::path::PathBuf) {
        let tempdir = tempfile::tempdir().unwrap();
        let test_game_dir = tempdir.path().join("game");
        std::fs::create_dir_all(&test_game_dir).unwrap();
        (tempdir, test_game_dir.to_owned())
    }

    #[test]
    fn apply_simple_patch() {
        let origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/patch/modulename.tp2");
        let old = super::read_all(&origin).unwrap();
        let patch = Patch::from_single(SIMPLEST_PATCH).unwrap();
        let result = apply_patch(&old, &patch);

        let patched_origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/patch/modulename_patched.tp2");
        let expected = super::read_all(&patched_origin).unwrap();

        assert_eq!(result, expected);
    }

    #[test]
    fn apply_delete_patch() {
        let origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/patch/modulename.tp2");
        let old = super::read_all(&origin).unwrap();
        let patch = Patch::from_single(PATCH_WITH_DELETE).unwrap();
        let result = apply_patch(&old, &patch);

        let patched_origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/patch/modulename_delete.tp2");
        let expected = super::read_all(&patched_origin).unwrap();

        assert_eq!(result, expected);
    }

    #[test]
    fn apply_add_patch() {
        let origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/patch/modulename.tp2");
        let old = super::read_all(&origin).unwrap();
        let patch = Patch::from_single(PATCH_WITH_ADD).unwrap();
        let result = apply_patch(&old, &patch);

        let patched_origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/patch/modulename_add.tp2");
        let expected = super::read_all(&patched_origin).unwrap();

        assert_eq!(result, expected);
    }

    #[test]
    fn apply_patch_with_unmodified_empty_line() {
        let origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/patch/modulename.tp2");
        let old = super::read_all(&origin).unwrap();
        let patch = Patch::from_single(PATCH_WITH_UNMODIFIED_EMPTY_LINE).unwrap();
        let result = apply_patch(&old, &patch);

        let patched_origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/patch/modulename_patched.tp2");
        let expected = super::read_all(&patched_origin).unwrap();

        assert_eq!(result, expected);
    }

    #[test]
    fn simple_patch_on_files() {
        let (_tempdir, game_dir) = setup_test_game_dir();
        let origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/patch/modulename.tp2");
        std::fs::copy(&origin, &game_dir.join("modulename.tp2")).unwrap();

        super::patch_module_with_content(&game_dir, "modulename", SIMPLEST_PATCH).unwrap();

        // file modulename.tp2.old must exist and contain OLD content
        let dot_old_file = game_dir.join("modulename.tp2.old");
        let dot_old_content = super::read_all(&dot_old_file).unwrap().join("\n");
        let expected = super::read_all(&origin).unwrap().join("\n");
        assert_eq!(dot_old_content, expected);
        let new_file = game_dir.join("modulename.tp2");

        // file modulename.tp2 must exist and contain NEW content
        let new_content = super::read_all(&new_file).unwrap().join("\n");
        let patched_origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/patch/modulename_patched.tp2");
        let expected = super::read_all(&patched_origin).unwrap().join("\n");
        assert_eq!(new_content, expected);
    }
}
