
use anyhow::{bail, Result};

use crate::{args::Uninstall, manifest::read_manifest, settings::Config};


pub fn uninstall(opts: &Uninstall, config: &Config) -> Result<()> {

    let manifest = read_manifest(&opts.manifest_path)?;
    let modules = &manifest.modules;

    let current = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(error) => bail!("Failed to obtain current directory\n -> {:?}", error),
    };
    //match find_tp2_str(current, module_name) {
    //    Err(error) => bail!("No module with name {} found - {:?}", module_name, error),
    //    Ok(tp2) => {
    //        match run_weidu_list_components(&tp2, lang_index) {
    //            Err(error) => bail!("Couldn't obtain component list for module {} - {:?}", module_name, error),
    //            Ok(list) => Ok(list),
    //        }
    //    }
    //}
    Ok(())
}
