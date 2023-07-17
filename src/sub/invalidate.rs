
use crate::args::Invalidate;
use crate::cache::Cache;
use crate::module::location::{ConcreteLocation, Location};
use crate::lowercase::lwc;
use crate::module::location::source::Source;
use crate::module::manifest::Manifest;
use crate::module::module::Module;
use crate::module::weidu_mod::WeiduMod;

use anyhow::{bail, Result};


pub fn invalidate(params: &Invalidate, cache: &Cache) -> Result<()> {
    let manifest = match Manifest::read_path(&params.manifest_path) {
        Ok(manifest) => manifest,
        Err(error) => bail!("Could not read manifest\n -> {:?}", error),
    };
    let mod_name= lwc!(&params.name);

    for item in manifest.modules {
        if &mod_name == item.get_name() {
            match item {
                Module::Mod { weidu_mod } => match &weidu_mod.location {
                    None => {} // continue to search a mod with same name and a location location
                    Some(Location::Concrete { concrete }) => {
                        clear_mod_archive(concrete, &weidu_mod, cache)?;
                        return Ok(()) // only once per name
                    }
                    Some(Location::Ref { r#ref }) => {
                        match manifest.locations.find(r#ref) {
                            None => {} // continue to search a mod with same name and a location location
                            Some(location) => {
                                clear_mod_archive(location, &weidu_mod, cache)?;
                                return Ok(()) // only once per name
                            }
                        }
                    }
                }
                Module::Generated { .. } => return Ok(()), // generated, not downloaded
            }
        }
    }
    bail!("Module {} not found or location not provided", mod_name);
}

fn clear_mod_archive(location: &ConcreteLocation, module :&WeiduMod, cache: &Cache) -> Result<()> {
    match location.source {
        Source::Local {..} | Source::Absolute{..} => bail!("Can't invalidate mods with absolute or local sources"),
        _ => {}
    }
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
