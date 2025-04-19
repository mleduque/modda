
use std::io::BufWriter;
use std::io::Write;
use std::path::Path;


use nu_ansi_term::Color;
use nu_ansi_term::Color::{Green, Red, Yellow};
use anyhow::Result;
use anyhow::anyhow;
use anyhow::bail;
use chrono::Local;
use log::info;

use crate::module::manifest::Manifest;
use crate::obtain::get_options::GetOptions;
use crate::timeline::InstallTimeline;
use crate::timeline::SetupTimeline;
use crate::module::gen_mod::GeneratedMod;
use crate::module::module_conf::ModuleContent;
use crate::module::weidu_mod::WeiduMod;
use crate::run_weidu::format_install_result;
use crate::tp2::find_tp2;
use crate::tp2_template::create_tp2;
use crate::run_weidu::run_weidu_install;
use crate::modda_context::ModdaContext;

pub struct ProcessResult {
    pub was_disabled: bool,
    pub stop: bool,
    pub timeline: InstallTimeline,
}

pub fn process_weidu_mod(weidu_mod: &WeiduMod, modda_context: &ModdaContext, manifest: &Manifest,
                            real_index: usize) -> Result<ProcessResult, anyhow::Error> {

    let mod_count = manifest.modules.len();
    let ModdaContext { current_dir: current, opts, module_downloader, ..} = modda_context;

    let mut install_timeline = InstallTimeline::new(weidu_mod.name.clone(), Local::now());

    let tp2 = match find_tp2(current, &weidu_mod.name) {
        Ok(tp2) => tp2,
        Err(_) => {
            // if tp2 not found, mod must be fetched from location (if any)
            let get_options = GetOptions { strict_replace: opts.check_replace };
            let setup_log = match module_downloader.get_module(&weidu_mod, &get_options) {
                Err(error) => {
                    let message = format!("module {name} (index={idx}/{len}) download/installation failed, stopping.",
                                                    name = weidu_mod.name, idx = real_index, len = mod_count);
                    modda_context.log(&message)?;
                    info!("{}", Red.bold().paint(message));
                    return Err(error)
                }
                Ok(setup_log) => {
                    configure_module(weidu_mod)?;
                    SetupTimeline {
                        configured: Some(Local::now()),
                        ..setup_log
                    }
                }
            };

            install_timeline.complete(setup_log);

            match find_tp2(current, &weidu_mod.name) {
                Ok(tp2) => tp2,
                Err(error) => {
                    let message = format!("module {name} (index={idx}/{len}) mod installed but no tp2 found, stopping.",
                                                    name = weidu_mod.name, idx = real_index, len = mod_count);
                    modda_context.log(&message)?;
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

    install_timeline.start_install = Some(Local::now());
    let single_result = run_weidu_install(&tp2_string, weidu_mod, &opts, &manifest.global, &modda_context.as_weidu_context())?;
    install_timeline.installed = Some(Local::now());

    let run_result = format_install_result(&single_result, weidu_mod);

    modda_context.log_bytes(&run_result)?;
    let must_stop = match single_result.status_code() {
        Some(0) => {
            let message = format!("module {name} (index={index}/{len}) finished with success.",
                            name = weidu_mod.name, index = real_index, len = mod_count);
            modda_context.log(&message)?;
            info!("{}", Green.bold().paint(message));
            false
        }
        Some(3) => {
            let mut finished = false;
            let (message, color) = if opts.no_stop_on_warn || weidu_mod.ignore_warnings {
                ignore_warnings(weidu_mod, real_index, mod_count)
            } else {
                finished = true;
                fail_warnings(weidu_mod, real_index, mod_count)
            };
            modda_context.log(&message)?;
            info!("{}", color.bold().paint(message));
            finished
        }
        Some(value) => {
            let message = format!("module {name} (index={idx}/{len}) finished with error (status={status}), stopping.",
                                    name = weidu_mod.name, idx = real_index, len = mod_count, status = value);
            modda_context.log(&message)?;
            info!("{}", Red.bold().paint(message));
            true
        }
        None => if !single_result.success() {
            let message = format!("module {name} (index={idx}/{len}) finished with success.",
                                    name = weidu_mod.name, idx = real_index, len = mod_count);
            modda_context.log(&message)?;
            info!("{}", Green.bold().paint(message));
            false
        } else {
            let message = format!("module {name} (index={idx}/{len}) finished with error, stopping.",
                                name = weidu_mod.name, idx = real_index, len = mod_count);
            modda_context.log(&message)?;
            info!("{}", Red.bold().paint(message));
            true
        }
    };
    Ok(ProcessResult { was_disabled: false, stop: must_stop, timeline: install_timeline })
}

pub fn process_generated_mod(gen_mod: &GeneratedMod, modda_context: &ModdaContext,
                                manifest: &Manifest, real_index: usize) -> Result<ProcessResult, anyhow::Error> {
    let ModdaContext { current_dir: current, file_installer, ..} = modda_context;

    if  find_tp2(current, &gen_mod.gen_mod).is_err() {
        let mod_dir = current.join(&gen_mod.gen_mod.as_ref())?;
        if let Err(err) = std::fs::create_dir(&mod_dir) {
            bail!("Could not create mod directory {:?} for generated mod '{}'\n  {}", mod_dir, gen_mod.gen_mod, err);
        }
        let data_dir = mod_dir.join("data")?;
        if let Err(err) = std::fs::create_dir(&data_dir) {
            bail!("Could not create data directory {:?} for generated mod '{}'\n  {}", data_dir, gen_mod.gen_mod, err);
        }
        if let Err(err) = file_installer.copy_from_origins(&gen_mod.files.iter().collect::<Vec<_>>(),
                                                                        &data_dir.path().to_path_buf(), gen_mod.allow_overwrite) {
            bail!("Could not copy files to target for generated mod {}\n  {}", gen_mod.gen_mod, err);
        }
        if let Err(err) = create_tp2(gen_mod, &mod_dir) {
            bail!("Could not generate tp2 file for {}\n  {}", gen_mod.gen_mod, err);
        }
    } else {
        info!("Skip generated mod creation (already present)");
    }
    let weidu_mod = gen_mod.as_weidu();
    process_weidu_mod(&weidu_mod, modda_context, manifest, real_index)
}


fn ignore_warnings(module: &WeiduMod, index: usize, total: usize) -> (String, Color) {
    let message = format!("module {modname} (index={idx}/{total}) finished with warning (status=3), ignoring as requested",
                                modname =  module.name, idx = index, total = total);
    (message, Yellow)
}

fn fail_warnings(module: &WeiduMod, index: usize, total: usize) -> (String, Color) {
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
