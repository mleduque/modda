
use std::path::PathBuf;

use anyhow::{Result, bail};
use globwalk::GlobWalkerBuilder;
use itertools::Itertools;
use path_clean::PathClean;
use serde::{Serialize, Deserialize};

use crate::args::Install;
use crate::canon_path::CanonPath;
use crate::global::Global;
use crate::module::file_module_origin::FileModuleOrigin;

pub struct FileInstaller<'a> {
    global: &'a Global,
    opts: &'a Install,
    game_dir: &'a CanonPath,
}

impl <'a> FileInstaller<'a> {
    pub fn new(global: &'a Global, opts: &'a Install, game_dir: &'a CanonPath) -> FileInstaller<'a> {
        FileInstaller { global, opts, game_dir }
    }

    pub fn copy_from_origins(&self, origins: &[&FileModuleOrigin], target: &PathBuf, allow_overwrite: bool) -> Result<()> {
        let globs = self.get_file_globs(origins)?;
        self.copy_from_globs(&globs, target, allow_overwrite)
    }

    fn get_file_globs(&self, origins: &[&FileModuleOrigin]) -> Result<Vec<CopyGlob>> {
        let results: Vec<_> = origins.iter().map(|origin| (self.get_origin_base(origin), origin.glob()))
                                    .collect();
        let (success, errors): (Vec<_>, Vec<_>) = results.iter().partition(|entry| entry.0.is_ok());
        let result = if !errors.is_empty() {
            bail!("Could not assemble FileModuleOrigins\n  {}",
                    errors.iter().map(|it| it.0.as_ref().unwrap_err()).join("\n  "))
        } else {
            success.iter().map(|(path_buf, glob)| {
                CopyGlob {
                    base: path_buf.as_ref().map(|it| it.clone()).unwrap(),
                    glob: glob.map(|it| it.to_owned() )
                }
            }).collect()
        };
        Ok(result)
    }

    fn get_origin_base(&self, origin: &FileModuleOrigin) -> Result<PathBuf> {
        match origin {
            FileModuleOrigin::Absolute { absolute, .. } => check_absolute(absolute),
            FileModuleOrigin::Local { local, .. } => self.get_local_base_path(local),
        }
    }

    fn get_local_base_path(&self, file_path: &String) -> Result<PathBuf, anyhow::Error> {
        let manifest_path = self.opts.get_manifest_root(self.game_dir);
        let local_files = match &self.global.local_files {
            None => PathBuf::new(),
            Some(path) => PathBuf::from(path).clean(),
        };
        if local_files.is_absolute() || local_files.starts_with("..") {
            bail!("Invalid local_files value");
        }
        let file_path = PathBuf::from(file_path).clean();
        if file_path.is_absolute() || local_files.starts_with("..") {
            bail!("Invalid local value");
        }
        let relative_path = local_files.join(file_path);
        Ok(manifest_path.join(relative_path)?.to_path_buf())
    }

    fn copy_from_globs(&self, globs: &[CopyGlob], target: &PathBuf, allow_overwrite: bool) -> Result<()> {
        // ensure the destination path exists
        ensure_path(target)?;

        for glob in globs {
            self.copy_from_glob(glob, target, allow_overwrite)?;
        }
        Ok(())
    }

    fn copy_from_glob(&self, copy_glob: &CopyGlob, target: &PathBuf, allow_overwrite: bool) -> Result<()> {
        match &copy_glob.glob {
            None => {
                if copy_glob.base.is_dir() {
                    copy_single_dir(&copy_glob.base, target, allow_overwrite)
                } else {
                    copy_single_file(&copy_glob.base, target, allow_overwrite)
                }
            },
            Some(glob) =>  {
                let glob_builder = GlobWalkerBuilder::from_patterns(&copy_glob.base, &vec![glob])
                        .case_insensitive(true);
                let glob = match glob_builder.build() {
                    Err(error) => bail!("Could not evaluate pattern {:?}\n -> {:?}", glob, error),
                    Ok(glob) => glob,
                };
                for item in glob.into_iter().filter_map(Result::ok) {
                    let copy_options = fs_extra::dir::CopyOptions {
                        overwrite: allow_overwrite,
                        ..Default::default()
                    };
                    let _bytes = fs_extra::copy_items(&vec![&item.into_path()], target, &copy_options)?;
                }
                Ok(())
            }
        }
    }

}

fn copy_single_file(path: &PathBuf, target: &PathBuf, allow_overwrite: bool) -> Result<()> {
    let copy_options = fs_extra::dir::CopyOptions {
        overwrite: allow_overwrite,
        copy_inside: true,
        ..Default::default()
    };
    if let Err(error) = fs_extra::copy_items(&[path], target, &copy_options) {
        bail!("Could not copy single file {:?} to {:?}\n  {}", path, target, error);
    }
    Ok(())
}

fn copy_single_dir(path: &PathBuf, target: &PathBuf, allow_overwrite: bool) -> Result<()> {
    let copy_options = fs_extra::dir::CopyOptions {
        overwrite: allow_overwrite,
        copy_inside: true,
        content_only: true,
        ..Default::default()
    };
    if let Err(error) = fs_extra::dir::copy(path, target, &copy_options) {
        bail!("Could not copy single dir {:?} to {:?}\n  {}", path, target, error);
    }
    Ok(())
}

fn ensure_path(target: &PathBuf)-> Result<()> {
    if let Err(error) = std::fs::create_dir_all(target) {
        bail!("ensure_dirs: error creating destination {:?}\n -> {:?}", target, error);
    } else {
        Ok(())
    }
}

fn check_absolute(path: &str) -> Result<PathBuf> {
    let path_buf = PathBuf::from(path).canonicalize()?;
    if !path_buf.is_absolute() {
        bail!("path {} is not absolute", path)
    } else if !path_buf.exists() {
        bail!("path {} doesn't exist", path)
    } else if path_buf.parent().is_none() {
        // arbitrarily disallow root as a base location
        bail!("path {} is not allowed as 'absolute' origin base; use a subdirectory", path)
    } else {
        Ok(path_buf)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(untagged)]
pub enum AllowOverwrite {
    Allow,
    #[default]
    Disallow,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Default)]
pub enum CopyMode {
    Glob,
    #[default]
    File,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct CopyOptions {
    pub allow_overwrite: AllowOverwrite,
    pub copy_mode: CopyMode,
}

struct CopyGlob {
    pub base: PathBuf,
    pub glob: Option<String>,
}
