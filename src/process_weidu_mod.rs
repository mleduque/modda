
use std::io::BufWriter;
use std::io::Write;
use std::path::Path;


use ansi_term::{Colour, Colour::{Green, Red, Yellow}};
use anyhow::Result;
use anyhow::anyhow;
use anyhow::bail;
use log::info;

use crate::manifest::Manifest;
use crate::module::gen_mod::GeneratedMod;
use crate::module::module_conf::ModuleContent;
use crate::module::weidu_mod::WeiduMod;
use crate::run_weidu::format_run_result;
use crate::settings::Config;
use crate::tp2::find_tp2;
use crate::tp2_template::create_tp2;
use crate::run_weidu::run_weidu;
use crate::weidu_context::WeiduContext;


pub fn process_weidu_mod(weidu_mod: &WeiduMod, weidu_context: &WeiduContext, manifest: &Manifest,
                            real_index: usize, config: &Config) -> Result<bool, anyhow::Error> {

    let mod_count = manifest.modules.len();
    let WeiduContext { current, opts, module_downloader, ..} = weidu_context;

    let tp2 = match find_tp2(current, &weidu_mod.name) {
        Ok(tp2) => tp2,
        Err(_) => {
            // if tp2 not found, mod must be fetched from location (if any)
            if let Err(error) = module_downloader.get_module(&weidu_mod) {
                let message = format!("module {name} (index={idx}/{len}) download/installation failed, stopping.",
                                                name = weidu_mod.name, idx = real_index + 1, len = mod_count);
                weidu_context.log(&message)?;
                info!("{}", Red.bold().paint(message));
                return Err(error)
            }
            match find_tp2(current, &weidu_mod.name) {
                Ok(tp2) => tp2,
                Err(error) => {
                    let message = format!("module {name} (index={idx}/{len}) mod installed but no tp2 found, stopping.",
                                                    name = weidu_mod.name, idx = real_index + 1, len = mod_count);
                    weidu_context.log(&message)?;
                    info!("{}", Red.bold().paint(message));
                    return Err(error)
                }
            }
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

    let single_result = run_weidu(&tp2_string, weidu_mod, &opts, &manifest.global, config)?;
    let run_result = format_run_result(&single_result, weidu_mod);

    weidu_context.log_bytes(&run_result)?;
    match single_result.status_code() {
        Some(0) => {
            let message = format!("module {name} (index={index}/{len}) finished with success.",
                            name = weidu_mod.name, index = real_index + 1, len = mod_count);
            weidu_context.log(&message)?;
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
            weidu_context.log(&message)?;
            info!("{}", color.bold().paint(message));
            Ok(finished)
        }
        Some(value) => {
            let message = format!("module {name} (index={idx}/{len}) finished with error (status={status}), stopping.",
                                    name = weidu_mod.name, idx = real_index + 1, len = mod_count, status = value);
            weidu_context.log(&message)?;
            info!("{}", Red.bold().paint(message));
            Ok(true)
        }
        None => if !single_result.success() {
            let message = format!("module {name} (index={idx}/{len}) finished with success.",
                                    name = weidu_mod.name, idx = real_index + 1, len = mod_count);
            weidu_context.log(&message)?;
            info!("{}", Green.bold().paint(message));
            Ok(false)
        } else {
            let message = format!("module {name} (index={idx}/{len}) finished with error, stopping.",
                                name = weidu_mod.name, idx = real_index + 1, len = mod_count);
            weidu_context.log(&message)?;
            info!("{}", Red.bold().paint(message));
            Ok(true)
        }
    }
}

pub fn process_generated_mod(gen_mod: &GeneratedMod, weidu_context: &WeiduContext,
                                manifest: &Manifest, real_index: usize, config: &Config) -> Result<bool, anyhow::Error> {
    let WeiduContext { current, file_installer, ..} = weidu_context;

    if let Ok(found) = find_tp2(current, &gen_mod.gen_mod) {
        bail!("Can't generate mod {}, tp2 with same name exists: {:?}", gen_mod.gen_mod, found)
    }
    let mod_dir = current.join(&gen_mod.gen_mod.as_ref());
    if let Err(err) = std::fs::create_dir(&mod_dir) {
        bail!("Could not create mod directory {:?} for generated mod '{}'\n  {}", mod_dir, gen_mod.gen_mod, err);
    }
    let data_dir = mod_dir.join("data");
    if let Err(err) = std::fs::create_dir(&data_dir) {
        bail!("Could not create data directory {:?} for generated mod '{}'\n  {}", data_dir, gen_mod.gen_mod, err);
    }
    if let Err(err) = file_installer.copy_from_origins(&gen_mod.files.iter().collect::<Vec<_>>(), &data_dir, gen_mod.allow_overwrite) {
        bail!("Could not copy files to target for generated mod {}\n  {}", gen_mod.gen_mod, err);
    }
    if let Err(err) = create_tp2(gen_mod, &mod_dir) {
        bail!("Could not generate tp2 file for {}\n  {}", gen_mod.gen_mod, err);
    }
    let weidu_mod = gen_mod.as_weidu();
    process_weidu_mod(&weidu_mod, weidu_context, manifest, real_index, config)
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
