
use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io::BufWriter;

use anyhow::{Result, bail};

use crate::args::Reverse;
use crate::canon_path::CanonPath;
use crate::module::components::FullComponent;
use crate::global::Global;
use crate::log_parser::{parse_weidu_log, LogRow};
use crate::lowercase::lwc;
use crate::module::global_locations::GlobalLocations;
use crate::module::manifest::Manifest;
use crate::module::module::Module;
use crate::module::weidu_mod::BareMod;
use crate::unique_component::UniqueComponent;
use crate::weidu_conf::read_weidu_conf_lang_dir;

pub fn extract_bare_mods() -> Result<Vec<BareMod>> {
    let log_rows = parse_weidu_log(None)?;
    let init: Vec<BareMod> = vec![];
    let mod_fragments = log_rows.iter().fold(init, |mut accumulator, row| {
        let current_mod = row.module.to_lowercase();
        let last_mod = accumulator.last().map(|module| module.name.clone());
        match last_mod {
            None => accumulator.push(bare_mod_from_log_row(row)),
            Some(mod_name) if mod_name == current_mod => {
                let last_index = accumulator.len() - 1;
                let last = accumulator.get_mut(last_index).unwrap();
                last.components.push(FullComponent {
                    index: row.component_index,
                    component_name: row.component_name.to_string(),
                });
            }
            _ => accumulator.push(bare_mod_from_log_row(row)),
        }
        accumulator
    });
    Ok(mod_fragments)
}

fn format_modules(bare_mods: Vec<BareMod>, export_component_name: Option<bool>, export_language: Option<bool>) -> Vec<Module> {
    bare_mods.into_iter().map(|item| Module::Mod { weidu_mod: item.to_weidu_mod(export_component_name, export_language) }).collect()
}

pub fn extract_manifest(args: &Reverse, game_dir: &CanonPath) -> Result<()> {
    let mods = extract_bare_mods()?;
    let mods = format_modules(mods, args.export_component_name, args.export_language);
    let manifest = generate_manifest(game_dir, mods)?;

    let output_file = OpenOptions::new().create_new(true).write(true).open(&args.output)?;
    let buf_writer = BufWriter::new(&output_file);
    Ok(serde_yaml::to_writer(buf_writer, &manifest)?)
}

pub fn generate_manifest(game_dir: &CanonPath, modules: Vec<Module>) -> Result<Manifest> {
    let lang_dir = match read_weidu_conf_lang_dir(game_dir)? {
        None => "en_US".to_string(),
        Some(lang) => lang.clone(),
    };
    Ok(Manifest {
        version: "1".to_string(),
        locations : GlobalLocations::default(),
        global: Global {
            game_language: lang_dir.clone(),
            lang_preferences: default_lang_pref(&lang_dir),
            ..Default::default()
        },
        modules,
    })
}

fn default_lang_pref(lang_dir: &str) -> Option<Vec<String>> {
    match lang_dir {
        "fr_FR" => Some(vec!["#rx#^fran[cç]ais".to_string(), "french".to_string()]),
        "en_US" => Some(vec!["english".to_string(), "american english".to_string()]),
        "es_ES" => Some(vec!["#rx#^espa[ñn]ol".to_string(), "spanish".to_string()]),
        // some more...
        _ => None,
    }
}

fn bare_mod_from_log_row(row: &LogRow) -> BareMod {
    let components = vec![
        FullComponent {
            index: row.component_index,
            component_name: row.component_name.to_string(),
        }
    ];
    BareMod {
        name: lwc!(&row.module),
        language: row.lang_index,
        components,
    }
}

pub fn extract_unique_components() -> Result<HashSet<UniqueComponent>> {
    let log_rows = parse_weidu_log(None)?;
    log_rows.iter().try_fold(HashSet::new(), |mut set, row| {
        let unique_component = UniqueComponent { mod_key: lwc!(&row.module), index: row.component_index };
        if set.insert(unique_component.clone()) {
            Ok(set)
        } else {
            bail!("Component appears multiple times in weidu.log: {:?}", unique_component);
        }
    })
}
