

use anyhow::{bail, Result};
use clap::Parser;
use env_logger::{Env, Target};
use log::debug;

use modda_lib::args::{ Cli, Commands };
use modda_lib::cache::Cache;
use modda_lib::canon_path::CanonPath;
use modda_lib::chitin::ensure_chitin_key;
use modda_lib::run_weidu::check_weidu_exe;
use modda_lib::settings::read_settings;
use modda_lib::sub::append_mod::append_mod;
use modda_lib::sub::discover::discover;
use modda_lib::sub::extract_manifest::extract_manifest;
use modda_lib::sub::install::install;
use modda_lib::sub::invalidate::invalidate;
use modda_lib::sub::list_components::sub_list_components;
use modda_lib::sub::reset::reset;
use modda_lib::sub::search::search;



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
    check_weidu_exe(&settings, &current_dir)?;
    let cache = Cache::ensure_from_config(&settings).unwrap();

    match cli.command {
        Commands::Install(ref install_opts) => install(install_opts, &settings, &current_dir, &cache),
        Commands::Search(ref search_opts) => search(search_opts),
        Commands::ListComponents(ref params) => sub_list_components(params, &current_dir, &settings),
        Commands::Invalidate(ref params) => invalidate(params, &cache),
        Commands::Reverse(ref params) => extract_manifest(params, &current_dir),
        Commands::AppendMod(ref params) => append_mod(params, &current_dir, &settings),
        Commands::Reset(ref reset_args) => reset(reset_args, &current_dir, &settings),
        Commands::Discover(ref params) => discover(params, &current_dir, &settings),
    }
}

