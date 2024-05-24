
use std::fs::OpenOptions;
use std::io::BufWriter;

use anyhow::{bail, Result};
use log::{debug, info};

use modda_lib::args::Discover;
use modda_lib::modda_context::WeiduContext;
use modda_lib::module::components::Components;
use modda_lib::module::module::Module;
use modda_lib::module::weidu_mod::WeiduMod;
use modda_lib::sub::extract_manifest::generate_manifest;
use modda_lib::tp2::find_game_tp2;

pub fn discover(params: &Discover, weidu_context: &WeiduContext) -> Result<()> {
    let game_dir = weidu_context.current_dir;
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

