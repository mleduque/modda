
use anyhow::Result;
use log::{debug, info};
use crate::file_installer::FileInstaller;
use crate::module::file_mod::FileModule;

pub struct FileModuleInstaller<'a> {
    file_installer: &'a FileInstaller<'a>,
}

// TODO instead, generate a weidu <file_mod_name>/<file_mod_name>.tp2,
// a <file_mod_name>/data/<file_name> and install the weidu way
impl <'a> FileModuleInstaller<'a> {
    pub fn new(file_installer: &'a FileInstaller<'a>) -> FileModuleInstaller<'a> {
        FileModuleInstaller { file_installer }
    }

    pub fn file_module_install(&self, file: &FileModule) -> Result<bool>  {
        let dest = self.file_installer.resolve_from_game_dir(&file.to);
        info!("Install file module {}({}) to {:?}.", file.file_mod, file.description.as_ref().map_or_else(|| "".to_string(), |desc| format!(" ({})", desc)), dest);
        debug!("{:?}", file);
        self.file_installer.copy_from_origin(&file.from, &dest, file.allow_overwrite)?;
        Ok(false)
    }
}
