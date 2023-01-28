use std::path::PathBuf;

use anyhow::{Result, bail};
use log::{debug, info};
use path_clean::PathClean;

use crate::args::Install;
use crate::canon_path::CanonPath;
use crate::global::Global;
use crate::module::{FileModule, FileModuleOrigin};

pub struct FileModuleInstaller<'a> {
    global: &'a Global,
    opts: &'a Install,
    game_dir: &'a CanonPath,
}

// TODO instead, generate a weidu <file_mod_name>/<file_mod_name>.tp2,
// a <file_mod_name>/data/<file_name> and install the weidu way
impl <'a> FileModuleInstaller<'a> {
    pub fn new(global: &'a Global, opts: &'a Install, game_dir: &'a CanonPath) -> FileModuleInstaller<'a> {
        FileModuleInstaller { global, opts, game_dir }
    }

    pub fn file_module_install(&self, file: &FileModule) -> Result<bool>  {
        info!("Install file module {}{}.", file.file_mod, file.description.as_ref().map_or_else(|| "".to_string(), |desc| format!(" ({})", desc)));
        debug!("{:?}", file);
        let origin = self.get_file_location(&file.from)?;
        let target_path = PathBuf::from(&file.to).clean();
        if target_path.is_absolute() || target_path.starts_with("..") {
            bail!("Invalid file module destination (`to` property)");
        }
        let target =PathBuf::from(self.game_dir).join(target_path);
        info!("FileModuleInstaller: copy file from {:?} to {:?}", origin, target);
        self.copy_file(&origin, &target)?;
        bail!("not implemented")
    }

    fn get_file_location(&self, origin: &FileModuleOrigin) -> Result<PathBuf> {
        match origin {
            FileModuleOrigin::Absolute { absolute } => Ok(PathBuf::from(absolute)),
            FileModuleOrigin::Local { local } => self.get_local_file_path(local),
        }
    }

    fn get_local_file_path(&self, file_path: &String) -> Result<PathBuf, anyhow::Error> {
        let manifest_path = self.get_manifest_root().clean();
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
        Ok(manifest_path.join(local_files).join(file_path))
    }

    fn get_manifest_root(&self) -> PathBuf {
        let manifest = PathBuf::from(&self.opts.manifest_path);
        match manifest.parent() {
            None => PathBuf::from(&self.game_dir),
            Some(path) => PathBuf::from(path),
        }
    }

    fn copy_file(&self, origin: &PathBuf, target: &PathBuf) -> Result<()> {
        bail!("not ready")
    }
}
