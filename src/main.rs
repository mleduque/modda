
mod archive_layout;
mod archive_extractor;
mod apply_patch;
mod args;
mod bufread_raw;
mod cache;
mod canon_path;
mod components;
mod download;
mod file_installer;
mod file_module_install;
mod get_module;
mod global;
mod language;
mod list_components;
mod location;
mod log_parser;
mod lowercase;
mod manifest;
mod module;
#[macro_use]
mod named_unit_variant;
mod patch_source;
mod pathext;
mod post_install;
mod process_weidu_mod;
mod progname;
mod replace;
mod run_result;
mod run_weidu;
mod settings;
mod sub;
mod timeline;
mod tp2;
mod tp2_template;
mod unique_component;
mod weidu_conf;
mod weidu_context;

use std::env::set_current_dir;
use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::Parser;
use env_logger::{Env, Target};
use log::{info, debug};

use args::{ Cli, Commands };
use cache::Cache;
use canon_path::CanonPath;
use run_weidu::check_weidu_exe;
use settings::read_settings;
use sub::install::install;
use sub::list_components::sub_list_components;
use sub::search::search;



fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
                            .target(Target::Stdout)
                            .init();

    let cli = Cli::parse();

    let current_dir = std::env::current_dir()?;
    let current_dir = CanonPath::new(current_dir)?;

    if ensure_chitin_key().is_err() {
        bail!("Must be run from the game directory (where chitin.key is)");
    } else {
        debug!("chitin.key found");
    }
    let settings = read_settings()?;
    check_weidu_exe(&settings)?;
    let cache = Cache::ensure_from_config(&settings).unwrap();

    match cli.command {
        Commands::Install(ref install_opts) => install(install_opts, &settings, &current_dir, &cache),
        Commands::Search(ref search_opts) => search(search_opts),
        Commands::ListComponents(ref params) => sub_list_components(params, &current_dir, &settings),
        Commands::Invalidate(ref params) => sub::invalidate::invalidate(params, &cache),
        Commands::Reverse(ref params) => sub::extract_manifest::extract_manifest(params, &current_dir),
        Commands::AppendMod(ref params) => sub::append_mod::append_mod(params, &current_dir, &settings),
        Commands::Reset(ref reset_args) => sub::reset::reset(reset_args, &current_dir, &settings),
    }
}

fn ensure_chitin_key() -> Result<()> {
    if !PathBuf::from("chitin.key").exists() {
        if PathBuf::from("game/chitin.key").exists() {
            if let Err(err) = set_current_dir("game") {
                bail!("Could not enter game directory 'game' {:?}", err)
            } else {
                info!("./game/chitin.key found, entered game subdir");
            }
        } else {
            bail!("no chitin.key of game/chitin.key file");
        }
    } else {
        info!("./chitin.key found");
    }
    Ok(())
}
