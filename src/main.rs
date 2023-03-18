
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
mod sub;
mod run_result;
mod settings;
mod tp2;
mod tp2_template;
mod run_weidu;
mod weidu_conf;
mod weidu_context;

use std::cell::RefCell;
use std::env::set_current_dir;
use std::io::BufWriter;
use std::path::PathBuf;

use ansi_term::Colour::Blue;
use anyhow::{anyhow, bail, Result};
use args::{ Cli, Install, Commands };
use cache::Cache;
use canon_path::CanonPath;
use clap::Parser;
use download::Downloader;
use env_logger::{Env, Target};
use file_installer::FileInstaller;
use get_module::ModuleDownload;
use log::{debug, info};
use manifest::Manifest;
use run_weidu::check_weidu_exe;
use settings::{read_settings, Config};
use sub::list_components::sub_list_components;
use sub::search::search;
use weidu_conf::check_weidu_conf_lang;
use weidu_context::WeiduContext;

use crate::file_module_install::FileModuleInstaller;
use crate::module::module::Module;
use crate::post_install::PostInstallOutcome;
use crate::log_parser::check_install_complete;
use crate::process_weidu_mod::{process_generated_mod, process_weidu_mod};



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
        info!("chitin.key found");
    }
    let settings = read_settings();
    check_weidu_exe(&settings)?;
    let cache = Cache::ensure_from_config(&settings).unwrap();
    match cli.command {
        Commands::Install(ref install_opts) => install(install_opts, &settings, &current_dir, &cache),
        Commands::Search(ref search_opts) => search(search_opts),
        Commands::ListComponents(ref params) => sub_list_components(params, &settings),
        Commands::Invalidate(ref params) => sub::invalidate::invalidate(params, &cache),
        Commands::Reverse(ref params) => sub::extract_manifest::extract_manifest(params, &current_dir),
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

fn install(opts: &Install, settings: &Config, game_dir: &CanonPath, cache: &Cache) -> Result<()> {

    let manifest = Manifest::read_path(&opts.manifest_path)?;
    check_weidu_conf_lang(game_dir, &manifest.global.game_language)?;
    let modules = &manifest.modules;

    let log = if let Some(output) = &opts.output {
        let file = match std::fs::OpenOptions::new().create(true).write(true).truncate(true).open(output) {
            Err(error) => return Err(
                anyhow!(format!("Could not create log file {} - {:?}", output, error)
            )),
            Ok(file) => file,
        };
        let buffered = BufWriter::new(file);
        Some(buffered)
    } else {
        None
    };

    let modules = match (opts.from_index, opts.to_index) {
        (Some(from_index), Some(to_index)) => {
            if from_index > modules.len() || from_index > to_index {
                return Ok(());
            } else if to_index > modules.len() {
                &modules[(from_index - 1)..]
            } else {
                &modules[(from_index - 1)..(to_index - 1)]
            }
        }
        (Some(from_index), None) => {
            if from_index > modules.len() {
                return Ok(());
            } else {
                &modules[(from_index - 1)..]
            }
        }
        (None, Some(to_index)) => {
            if to_index > modules.len() {
                &modules
            } else {
                &modules[..(to_index - 1)]
            }
        }
        (None, None) => &modules,
    };

    let current = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(error) => bail!("Failed to obtain current directory\n -> {:?}", error),
    };
    let downloader = Downloader::new();
    let module_downloader = ModuleDownload::new(&settings, &manifest.global, &opts,
                                                                        &downloader, &game_dir, cache);
    let file_installer = FileInstaller::new(&manifest.global, &opts, &game_dir);
    let file_module_installer = FileModuleInstaller::new(&file_installer);

    let weidu_context = WeiduContext { current: &current, settings: &settings, opts: &opts,
                                                    module_downloader: &module_downloader, file_installer: &file_installer,
                                                    log: RefCell::from(log) };

    for (index, module) in modules.iter().enumerate() {
        let real_index = index + opts.from_index.unwrap_or(0);
        info!("module {} - {}", real_index, module.describe());
        debug!("{:?}", module);
        let finished = match module {
            Module::Mod { weidu_mod } => process_weidu_mod(weidu_mod, &weidu_context, &manifest, real_index, settings)?,
            Module::File { file } => file_module_installer.file_module_install(file)?,
            Module::Generated { gen } => process_generated_mod(gen, &weidu_context, &manifest, real_index, settings)?,
        }
        ;
        if finished {
            bail!("Program interrupted on error on non-whitelisted warning");
        } else {
            match module.exec_post_install(&module.get_name()) {
                PostInstallOutcome::Stop => {
                    info!("{}",  Blue.bold().paint(format!("Interruption requested for module {} - {}",
                                                            real_index + 1, module.describe())));
                    return Ok(());
                }
                PostInstallOutcome::Continue => {}
            }
        }
        // Now check we actually installed all requested components
        check_install_complete(&module)?
    }
    Ok(())
}
