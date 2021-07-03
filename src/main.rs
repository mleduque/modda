mod args;
mod bufread_raw;
mod get_module;
mod download;
mod language;
mod archive_layout;
mod list_components;
mod log_parser;
mod lowercase;
mod manifest;
mod sub;
mod run_result;
mod settings;
mod tp2;
mod weidu;

use std::io::{BufReader, BufWriter};
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use ansi_term::{Colour, Colour::{Green, Red, Yellow}};
use anyhow::{anyhow, bail, Result};
use clap::Clap;

use args::{ Opts, Install };
use get_module::get_module;
use log_parser::{find_components_without_warning, parse_weidu_log};
use lowercase::LwcString;
use list_components::list_components;
use manifest::{ Module, ModuleContent, read_manifest };
use settings::{read_settings, Config};
use sub::list_components::sub_list_components;
use sub::search::search;
use tp2::find_tp2;
use weidu::{run_weidu, write_run_result};



fn main() -> Result<()> {
    if !PathBuf::from("chitin.key").exists() {
        bail!("Must be run from the game directory (where chitin.key is)");
    }
    let settings = read_settings();
    let opts: Opts = Opts::parse();
    match opts {
        Opts::Install(ref install_opts) => { install(install_opts, &settings)?; Ok(()) },
        Opts::Search(ref search_opts) => search(search_opts),
        Opts::ListComponents(ref params) => sub_list_components(params),
    }
}

fn install(opts: &Install, settings: &Config) -> Result<()> {

    let manifest = read_manifest(&opts.manifest_path)?;
    check_weidu_conf_lang(&manifest.global.game_language)?;
    let modules = &manifest.modules;

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
        (Some(from_index), Some(to_index)) => &modules[from_index..to_index],
        (Some(from_index), None) => &modules[from_index..],
        (None, Some(to_index)) => &modules[..to_index],
        (None, None) => &modules,
    };
    let mut finished = false;
    for (index, module) in modules.iter().enumerate() {
        println!("module {} - {}", index, module.name);
        let tp2 = match find_tp2(&module.name) {
            Ok(tp2) => tp2,
            Err(_) => {
                get_module(&module, &settings)?;
                find_tp2(&module.name)?
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
        configure_module(module)?;
        let single_result = run_weidu(&tp2_string, module, &opts, &manifest.global)?;
        if let Some(ref mut file) = log {
            let _ = write_run_result(&single_result, file, module);
        }
        match single_result.status_code() {
            Some(0) => {
                let message = format!("module {} (index={}) finished with success.", 
                                module.name, index);
                if let Some(ref mut file) = log {
                    let _ = writeln!(file, "{}", message);
                }
                println!("{}", Green.bold().paint(message));
            }
            Some(3) => {
                let (message, color) = if opts.no_stop_on_warn || module.ignore_warnings {
                    ignore_warnings(module, index)
                } else {
                    // need to check if component with warning was flagged with ignore_warnings
                    if component_failure_allowed(module) {
                        ignore_warnings(module, index)
                    } else {
                        finished = true;
                        fail_warnings(module, index)
                    }
                }; 
                if let Some(ref mut file) = log {
                    let _ = writeln!(file, "{}", message);
                }
                println!("{}", color.bold().paint(message));
            }
            Some(value) => {
                finished = true;
                let message = format!("module {} (index={}) finished with error (status={}), stopping.", 
                                        module.name, index, value);
                if let Some(ref mut file) = log {
                    let _ = writeln!(file, "{}", message);
                }
                println!("{}", Red.bold().paint(message));
            }
            None => if !single_result.success() {
                let message = format!("module {} (index={}) finished with success.", 
                                module.name, index);
                if let Some(ref mut file) = log {
                    let _ = writeln!(file, "{}", message);
                }
                println!("{}", Green.bold().paint(message));
            } else {
                finished = true;
                let message = format!("module {} (index={}) finished with error, stopping.", 
                                        module.name, index);
                if let Some(ref mut file) = log {
                    let _ = writeln!(file, "{}", message);
                }
                println!("{}", Red.bold().paint(message));
            }
        }
        if finished {
            bail!("Program interrupted on error on non-whitelisted warning");
        }
    }
    Ok(())
}

fn component_failure_allowed(module: &Module) -> bool {
    let warning_allowed = module.components_with_warning();
    if warning_allowed.is_empty() {
        return false;
    }
    let components_that_didnt_warn = match find_components_without_warning(module) {
        Err(error) => {
            eprintln!("Could not retrieve per-components success state from weidu.log for module {} - {:?}", module.name, error);
            return false;
        }
        Ok(report) => report,
    };

    // read module installation language index from weidu.log
    let module_lang_idx = match parse_weidu_log(Some(LwcString::new(&module.name))) {
        Err(error) => {
            eprintln!("Couldn't read module installation language from weidu.log\n->{:?}", error);
            return false;
        }
        Ok(report) => match report.first() {
            None => {
                eprintln!("Couldn't read module installation language from weidu.log\n-> no row in weidu.log for module {}", module.name);
                return false;
            }
            Some(row) => row.lang_index,
        }
    };

    // Ask weidu the list of components in the module in the (module) install language
    // to match component numbers with their "name"
    let components = match list_components(&module.name, module_lang_idx) {
        Err(error) => {
            eprintln!("Couldn't obtain component list for module {} - {:?}", module.name, error);
            return false;
        }
        Ok(list) => list,
    };

    // get list of names of components that are allowed to have warnings 
    // (we only have indexes until now)
    let allowed_names = warning_allowed.iter().filter_map(|comp| {
        match components.iter().find(|weidu_comp| weidu_comp.number == comp.index()) {
            None => None,
            Some(weidu_comp) => Some(weidu_comp.name.to_owned())
        }
    }).collect::<Vec<_>>();

    for component_name in allowed_names {
        if !components_that_didnt_warn.contains(&component_name) {
            return false;
        }
    }
    true
}

fn ignore_warnings(module: &Module, index: usize) -> (String, Colour) {
    let message = format!("module {} (index={}) finished with warning (status=3), ignoring as requested", 
                            module.name, index);
    (message, Yellow)
}

fn fail_warnings(module: &Module, index: usize) -> (String, Colour) {
    let message = format!("module {} (index={}) finished with warning (status=3), stopping as requested", 
                            module.name, index);
    (message, Red)
}

fn configure_module(module: &Module) -> Result<()> {
    if let Some(conf) = &module.add_conf {
        let conf_path = Path::new(&module.name).join(&conf.file_name);
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
            ModuleContent::Content(content) => content,
            ModuleContent::Prompt(_prompt) => {
                // print the prompt and read the content line
                bail!("not implemented yet")
            }
        };
        writeln!(buffered, "{}", content)?;
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
