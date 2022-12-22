use crate::{args::Invalidate, manifest::{Location, Module, read_manifest}, cache::Cache};

use anyhow::{bail, Result};

use crate::lowercase::LwcString;

pub fn invalidate(params: &Invalidate, cache: &Cache) -> Result<()> {
    let manifest = match read_manifest(&params.manifest_path) {
        Ok(manifest) => manifest,
        Err(error) => bail!("Could not read manifest\n -> {:?}", error),
    };
    let modname= lwc!(&params.name);
    for item in manifest.modules {
        if modname == item.name {
            match &item.location {
                None => {}
                Some(location) => {
                    clear_mod_archive(location, &item, cache)?;
                    return Ok(())
                }
            }
        }
    }
    bail!("Module {} not found or location not provided", modname);
}

fn clear_mod_archive(location: &Location, module :&Module, cache: &Cache) -> Result<()> {

    let dest = cache.join(location.source.save_subdir()?);
    let save_name = location.source.save_name(&module.name)?;
    let archive_path = dest.join(save_name);
    if archive_path.exists() {
        match std::fs::remove_file(&archive_path) {
            Ok(_) => Ok(()),
            Err(error) => bail!("Could not remove archive {:?}\n -> {:?}", archive_path, error),
        }
    } else {
        println!("Archive for mod {} not present.", module.name);
        Ok(())
    }
}
