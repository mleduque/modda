
mod archive_layout;
mod archive_extractor;
mod apply_patch;
mod args;
mod bufread_raw;
mod cache;
mod canon_path;
mod components;
mod download;
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
mod progname;
mod replace;
mod sub;
mod run_result;
mod settings;
mod tp2;
mod weidu;

use std::env::set_current_dir;
use std::io::{BufReader, BufWriter};
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use ansi_term::{Colour, Colour::{Blue, Green, Red, Yellow}};
use anyhow::{anyhow, bail, Result};
use args::{ Opts, Install };
use cache::Cache;
use canon_path::CanonPath;
use clap::Clap;
use download::Downloader;
use env_logger::{Env, Target};
use get_module::ModuleDownload;
use log::{debug, info};
use manifest::Manifest;
use module::{WeiduMod, ModuleContent, Module};
use settings::{read_settings, Config};
use sub::list_components::sub_list_components;
use sub::search::search;
use tp2::find_tp2;
use weidu::{run_weidu, write_run_result};

use crate::file_module_install::FileModuleInstaller;
use crate::post_install::PostInstallOutcome;
use crate::log_parser::check_install_complete;



fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
                            .target(Target::Stdout)
                            .init();

    let current_dir = std::env::current_dir()?;
    let current_dir = CanonPath::new(current_dir)?;

    if ensure_chitin_key().is_err() {
        bail!("Must be run from the game directory (where chitin.key is)");
    } else {
        info!("chitin.key found");
    }
    let settings = read_settings();
    let opts: Opts = Opts::parse();
    let cache = Cache::ensure_from_config(&settings).unwrap();
    match opts {
        Opts::Install(ref install_opts) => install(install_opts, &settings, &current_dir, &cache),
        Opts::Search(ref search_opts) => search(search_opts),
        Opts::ListComponents(ref params) => sub_list_components(params),
        Opts::Invalidate(ref params) => sub::invalidate::invalidate(params, &cache),
    }
}

fn ensure_chitin_key() -> Result<()> {
    if !PathBuf::from("chitin.key").exists() {
        if PathBuf::from("game/chitin.key").exists() {
            if let Err(err) = set_current_dir("game") {
                bail!("Could not enter game directory 'game' {:?}", err)
            } else {
                info!("./game//chitin.key found, entered game subdir");
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
    check_weidu_conf_lang(&manifest.global.game_language)?;
    let modules = &manifest.modules;
    let mod_count = modules.len();

    let mut log = if let Some(output) = &opts.output {
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
        (Some(from_index), Some(to_index)) => &modules[(from_index - 1)..(to_index - 1)],
        (Some(from_index), None) => &modules[(from_index - 1)..],
        (None, Some(to_index)) => &modules[..(to_index - 1)],
        (None, None) => &modules,
    };

    let current = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(error) => bail!("Failed to obtain current directory\n -> {:?}", error),
    };
    let downloader = Downloader::new();
    let module_downloader = ModuleDownload::new(&settings, &manifest.global, &opts,
                                                                        &downloader, &game_dir, cache);
    let file_module_installer = FileModuleInstaller::new(&manifest.global, &opts, &game_dir);

    for (index, module) in modules.iter().enumerate() {
        let real_index = index + opts.from_index.unwrap_or(0);
        info!("module {} - {}", real_index, module.describe());
        debug!("{:?}", module);
        let finished = match module {
            Module::Mod { weidu_mod } => process_weidu_mod(weidu_mod, &module_downloader, &current, opts, &manifest,
                                                                        &mut log, real_index, mod_count)?,
            Module::File { file } => file_module_installer.file_module_install(file)?,
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

fn process_weidu_mod(weidu_mod: &WeiduMod, module_downloader: &ModuleDownload, current: &PathBuf, opts: &Install, manifest: &Manifest,
                    log: &mut Option<BufWriter<std::fs::File>>, real_index: usize, mod_count: usize) -> Result<bool, anyhow::Error> {
    let tp2 = match find_tp2(current, &weidu_mod.name) {
        Ok(tp2) => tp2,
        Err(_) => {
            // if tp2 not found, mod must be fetched from location (if any)
            module_downloader.get_module(&weidu_mod)?;
            find_tp2(current, &weidu_mod.name)?
        }
    };
    let tp2_string = match tp2.into_os_string().into_string() {
        Ok(string) => string,
        Err(os_string) => {
            let os_str = os_string.as_os_str();
            let msg = os_str.to_string_lossy().to_owned();
            return Err(anyhow!(format!("{}", msg)));
        }
    };
    configure_module(weidu_mod)?;
    let single_result = run_weidu(&tp2_string, weidu_mod, &opts, &manifest.global)?;
    if let Some(ref mut file) = *log {
        let _ = write_run_result(&single_result, file, weidu_mod);
    }
    match single_result.status_code() {
        Some(0) => {
            let message = format!("module {name} (index={index}/{len}) finished with success.",
                            name = weidu_mod.name, index = real_index + 1, len = mod_count);
            if let Some(ref mut file) = *log {
                let _ = writeln!(file, "{}", message);
            }
            info!("{}", Green.bold().paint(message));
            Ok(false)
        }
        Some(3) => {
            let mut finished = false;
            let (message, color) = if opts.no_stop_on_warn || weidu_mod.ignore_warnings {
                ignore_warnings(weidu_mod, real_index, mod_count)
            } else {
                finished = true;
                fail_warnings(weidu_mod, real_index, mod_count)
            };
            if let Some(ref mut file) = *log {
                let _ = writeln!(file, "{}", message);
            }
            info!("{}", color.bold().paint(message));
            Ok(finished)
        }
        Some(value) => {
            let message = format!("module {name} (index={idx}/{len}) finished with error (status={status}), stopping.",
                                    name = weidu_mod.name, idx = real_index + 1, len = mod_count, status = value);
            if let Some(ref mut file) = *log {
                let _ = writeln!(file, "{}", message);
            }
            info!("{}", Red.bold().paint(message));
            Ok(true)
        }
        None => if !single_result.success() {
            let message = format!("module {name} (index={idx}/{len}) finished with success.",
                                    name = weidu_mod.name, idx = real_index + 1, len = mod_count);
            if let Some(ref mut file) = *log {
                let _ = writeln!(file, "{}", message);
            }
            info!("{}", Green.bold().paint(message));
            Ok(false)
        } else {
            let message = format!("module {name} (index={idx}/{len}) finished with error, stopping.",
                                name = weidu_mod.name, idx = real_index + 1, len = mod_count);
            if let Some(ref mut file) = *log {
                let _ = writeln!(file, "{}", message);
            }
            info!("{}", Red.bold().paint(message));
            Ok(true)
        }
    }
}

fn ignore_warnings(module: &WeiduMod, index: usize, total: usize) -> (String, Colour) {
    let message = format!("module {modname} (index={idx}/{total}) finished with warning (status=3), ignoring as requested",
                                modname =  module.name, idx = index, total = total);
    (message, Yellow)
}

fn fail_warnings(module: &WeiduMod, index: usize, total: usize) -> (String, Colour) {
    let message = format!("module {modname} (index={idx}/{total}) finished with warning (status=3), stopping as requested",
                                modname =  module.name, idx = index, total = total);
    (message, Red)
}

fn configure_module(module: &WeiduMod) -> Result<()> {
    if let Some(conf) = &module.add_conf {
        let conf_path = Path::new(module.name.as_ref()).join(&conf.file_name);
        let file = match std::fs::OpenOptions::new()
                        .create(true).write(true).truncate(true)
                        .open(&conf_path) {
            Err(error) => return Err(
                anyhow!(format!("Could not create conf file {:?} - {:?}", conf_path, error)
            )),
            Ok(file) => file,
        };
        let mut buffered = BufWriter::new(file);
        let content = match &conf.content {
            ModuleContent::Content { content } => content,
            ModuleContent::Prompt { .. } => {
                // print the prompt and read the content line
                bail!("not implemented yet")
            }
        };
        write!(buffered, "{}", content)?;
        buffered.flush()?;
        Ok(())
    } else { Ok(()) }
}

fn check_weidu_conf_lang(lang: &str) -> Result<()> {
    if !Path::new("weidu.conf").exists() {
        return Ok(())
    }
    let file = match std::fs::File::open("weidu.conf") {
        Err(error) => return Err(
            anyhow!(format!("Could not open weidu.conf - {:?}", error)
        )),
        Ok(file) => file,
    };
    let regex = regex::Regex::new(r##"(?i)lang_dir(\s)+=(\s)+([a-z_]+)"##)?;
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line?;
        if let Some(caps) = regex.captures_iter(&line).next() {
            if caps[3].to_lowercase() == lang.to_lowercase() {
                return Ok(())
            } else {
                bail!(
                    "lang_dir (in manifest) {} doesn't match value in weidu.conf {}",
                    lang, &caps[3]
                );
            }
        }
    }
    Ok(())
}
