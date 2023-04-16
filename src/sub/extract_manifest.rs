
use std::fs::OpenOptions;
use std::io::BufWriter;

use anyhow::Result;

use crate::args::Reverse;
use crate::canon_path::CanonPath;
use crate::components::{Component, Components};
use crate::global::Global;
use crate::log_parser::{parse_weidu_log, LogRow};
use crate::lowercase::lwc;
use crate::manifest::Manifest;
use crate::module::module::Module;
use crate::module::weidu_mod::WeiduMod;
use crate::weidu_conf::read_weidu_conf_lang_dir;


pub fn extract_manifest(args: &Reverse, game_dir: &CanonPath) -> Result<()> {
    let lang_dir = match read_weidu_conf_lang_dir(game_dir)? {
        None => "en_en".to_string(),
        Some(lang) => lang.clone(),
    };
    let log_rows = parse_weidu_log(None)?;
    let init: Vec<WeiduMod> = vec![];
    let mod_fragments = log_rows.iter().fold(init, |mut accumulator, row| {
        let current_mod = row.module.to_lowercase();
        let last_mod = accumulator.last().map(|module| module.name.clone());
        match last_mod {
            None => accumulator.push(weidu_mod_from_log_row(row, args)),
            Some(mod_name) if mod_name == current_mod => {
                let last_index = accumulator.len() - 1;
                let last = accumulator.get_mut(last_index).unwrap();
                last.components.as_mut_list().unwrap().push(Component::Full {
                    index: row.component_index,
                    component_name: row.component_name.to_string(),
                });
            }
            _ => accumulator.push(weidu_mod_from_log_row(row, args)),
        }
        accumulator
    });
    let manifest = Manifest {
        version: "1".to_string(),
        global: Global {
            game_language: lang_dir.clone(),
            lang_preferences: default_lang_pref(&lang_dir),
            ..Default::default()
        },
        modules: mod_fragments.into_iter().map(|item| Module::Mod { weidu_mod: item }).collect(),
    };
    let output_file = OpenOptions::new().create_new(true).write(true).open(&args.output)?;
    let buf_writer = BufWriter::new(&output_file);
    Ok(serde_yaml::to_writer(buf_writer, &manifest)?)
}

fn weidu_mod_from_log_row(row: &LogRow, args: &Reverse) -> WeiduMod {
    let components = match args.export_component_name {
        Some(false) => Components::List(vec![ Component::Simple(row.component_index)]),
        _ => Components::List(vec![
            Component::Full {
                index: row.component_index,
                component_name: row.component_name.to_string(),
            }
        ])
    };
    WeiduMod {
        name: lwc!(&row.module),
        language: if let Some(true) = args.export_language { Some(row.lang_index) } else { None },
        components,
        ..Default::default()
    }
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
