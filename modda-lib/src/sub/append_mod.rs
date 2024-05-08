
use std::fs::OpenOptions;
use std::io::{ErrorKind, BufWriter};
use std::path::PathBuf;

use anyhow::{bail, Result};
use log::debug;

use crate::args::AppendMod;
use crate::modda_context::WeiduContext;
use crate::module::components::{Components, Component, FullComponent};
use crate::module::language::{select_language_pref, LanguageSelection};
use crate::list_components::list_components;
use crate::lowercase::LwcString;
use crate::module::manifest::Manifest;
use crate::module::module::Module;
use crate::module::weidu_mod::WeiduMod;
use crate::utils::pathext::append_extension;
use crate::run_weidu::WeiduComponent;
use crate::sub::extract_manifest::generate_manifest;
use crate::tp2::find_tp2_str;


pub fn append_mod(params: &AppendMod, weidu_context: &WeiduContext) -> Result<()> {
    let mod_name = &params.r#mod;
    let existing: Result<Option<Manifest>> = match OpenOptions::new().read(true).open(&params.output) {
        Err(err)if err.kind() == ErrorKind::NotFound => Ok(None),
        Err(err) => bail!("Error reading output file {}\n  {}", &params.output, err),
        Ok(file) => Ok(Some(Manifest::read_file(file, true)?)),
    };
    let existing = existing?;

    let tp2 = match find_tp2_str(weidu_context.current_dir, mod_name) {
        Ok(tp2) => tp2,
        Err(_) => bail!(""),
    };
    let lang_preferences = match existing {
        None => None,
        Some(ref manifest) => manifest.global.lang_preferences.clone(),
    };
    let selected_lang = match select_language_pref(&tp2, mod_name, &lang_preferences, weidu_context) {
        Ok(LanguageSelection::Selected(selected)) => selected,
        _ => 0,
    };
    let components = match list_components(mod_name, selected_lang,weidu_context) {
        Err(error) => bail!("Could not obtain component list for mod {}\n  {}",params.r#mod, error),
        Ok(value) => value,
    };
    let generate_comment = match params.export_component_name {
        Some(true) => true,
        _ => false,
    };
    let modified = match existing {
        None => generate_manifest(weidu_context.current_dir, vec![generate_mod(mod_name, components, generate_comment)]),
        Some(ref manifest) => Ok(append_to_manifest(&manifest, mod_name, components, generate_comment)),
    };
    let modified = modified?;

    // write back to file (create + replace)
    let output_path = PathBuf::from(&params.output);
    let temp_path = append_extension("new", &output_path);
    let dest = match OpenOptions::new().create(true).truncate(true).write(true).open(&temp_path) {
        Err(err) => bail!("Could not create temp output file\n  {}", err),
        Ok(file) => file,
    };
    let buf_writer = BufWriter::new(&dest);
    serde_yaml::to_writer(buf_writer, &modified)?;
    if let Err(error) = std::fs::rename(&temp_path, output_path) {
        bail!("Failed to rename temp output file {:?} to {:?}\n -> {:?}", temp_path, params.output, error);
    } else {
        debug!("renamed temp output file to {:?}", params.output);
    }
    Ok(())
}

fn generate_mod(mod_name: &LwcString, components: Vec<WeiduComponent>, generate_comment: bool) -> Module {
    Module::Mod {
        weidu_mod: WeiduMod {
            name: mod_name.clone(),
            components: Components::List(
                components.iter()
                    .map(|comp| {
                        match generate_comment {
                            true => Component::Full(FullComponent { index: comp.index, component_name: comp.name.to_owned() }),
                            false => Component::Simple(comp.index),
                        }
                    })
                    .collect()
            ),
            ..Default::default()
        }
    }
}

fn append_to_manifest(original: &Manifest, mod_name: &LwcString, components: Vec<WeiduComponent>, generate_comment: bool) -> Manifest {
    let mut mods = original.modules.clone();
    mods.push( generate_mod(mod_name, components, generate_comment));

    Manifest {
        modules: mods,
        ..original.to_owned()
    }
}
