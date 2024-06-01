
mod log_settings;
mod subcommands;

use anyhow::{bail, Result};
use clap::Parser;
use env_logger::{Env, Target};
use log::debug;

use log_settings::LogSettings;
use modda_lib::args::{ Cli, Commands, ConfigArgs };
use modda_lib::cache::Cache;
use modda_lib::canon_path::CanonPath;
use modda_lib::chitin::ensure_chitin_key;
use modda_lib::modda_context::WeiduContext;
use modda_lib::run_weidu::check_weidu_exe;
use modda_lib::config::{global_conf_dir, Settings};
use modda_lib::sub::append_mod::append_mod;
use modda_lib::sub::extract_manifest::extract_manifest;
use modda_lib::sub::install::install;
use modda_lib::sub::invalidate::invalidate;
use subcommands::config_show::open_global_config_dir;
use subcommands::config_edit::edit_global_config_dir;
use subcommands::discover::discover;
use subcommands::introspect::introspect;
use subcommands::list_components::sub_list_components;
use subcommands::reset::reset;
use subcommands::search::search;

fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
                            .target(Target::Stdout)
                            .init();

    let log_settings = LogSettings {
        max_level: log::max_level(),
        log_var_name: "RUST_LOG".to_string(),
        log_var_value: std::env::var("RUST_LOG").unwrap_or("<not present>".to_string()),
        log_style_name: "RUST_LOG_STYLE".to_string(),
        log_style_value: std::env::var("RUST_LOG_STYLE").unwrap_or("<not present>".to_string()),
    };

    let cli = Cli::parse();

    let current_dir = std::env::current_dir()?;
    let current_dir = CanonPath::new(current_dir)?;

    if ensure_chitin_key().is_err() {
        bail!("Must be run from the game directory (where chitin.key is)");
    } else {
        debug!("chitin.key found");
    }
    let settings = Settings::read_settings(&current_dir)?;
    let config = &settings.combined;
    let weidu_context = WeiduContext{ config: &config, current_dir: &current_dir };
    check_weidu_exe(&weidu_context)?;
    let cache = Cache::ensure_from_config(config).unwrap();

    match cli.command {
        Commands::Install(ref install_opts) => install(install_opts, &config, &current_dir, &cache),
        Commands::Search(ref search_opts) => search(search_opts),
        Commands::ListComponents(ref params) => sub_list_components(params, &weidu_context),
        Commands::Invalidate(ref params) => invalidate(params, &cache),
        Commands::Reverse(ref params) => extract_manifest(params, &current_dir),
        Commands::AppendMod(ref params) => append_mod(params, &weidu_context),
        Commands::Reset(ref reset_args) => reset(reset_args, &weidu_context),
        Commands::Discover(ref params) => discover(params, &weidu_context),
        Commands::Introspect(ref params) => introspect(params, &settings, &current_dir,
                                                                    &global_conf_dir(),
                                                                    &log_settings),
        Commands::GlobalConfig(sub) => match sub {
            ConfigArgs::Show(_) => open_global_config_dir(),
            ConfigArgs::Edit(_) => edit_global_config_dir(&config),
        }
    }
}
