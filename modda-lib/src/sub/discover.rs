
use std::fs::OpenOptions;
use std::io::BufWriter;

use anyhow::{bail, Result};
use log::{debug, info};

use crate::args::Discover;
use crate::canon_path::CanonPath;
use crate::module::components::Components;
use crate::module::module::Module;
use crate::module::weidu_mod::WeiduMod;
use crate::settings::Config;
use crate::tp2::find_game_tp2;

use super::extract_manifest::generate_manifest;


pub fn discover(params: &Discover, game_dir: &CanonPath, config: &Config) -> Result<()> {
    info!("Discovering mods in {:?}", game_dir);
    let mods = match find_game_tp2(game_dir) {
        Err(error) => bail!("Failed to detect mods in game dir {:?}\n  {:?}", game_dir, error),
        Ok(mods) => mods,
    };
    debug!("Found {} mods", mods.len());
    let mods = mods.iter().map(|m| Module::Mod { weidu_mod: WeiduMod {
        name: m.clone(),
        components: Components::Ask,
        ..Default::default()
    }}).collect::<Vec<_>>();
    let manifest = generate_manifest(game_dir, mods)?;

    let output_file = OpenOptions::new().create_new(true).write(true).open(&params.output)?;
    let buf_writer = BufWriter::new(&output_file);
    Ok(serde_yaml::to_writer(buf_writer, &manifest)?)
}

