

use std::cell::RefCell;
use std::fs::OpenOptions;
use std::io::BufWriter;
use std::path::PathBuf;

use ansi_term::Colour::{Blue, Green, Red};
use anyhow::{Result, anyhow, bail};
use chrono::Local;
use itertools::Itertools;
use log::{debug, info, error, warn};

use crate::args::Install;
use crate::cache::Cache;
use crate::canon_path::CanonPath;
use crate::module::components::{Components, Component, FullComponent};
use crate::download::Downloader;
use crate::file_installer::FileInstaller;
use crate::get_module::ModuleDownload;
use crate::lowercase::{lwc, LwcString};
use crate::module::module::Module;
use crate::module::weidu_mod::WeiduMod;
use crate::post_install::PostInstallOutcome;
use crate::log_parser::{check_install_complete, parse_weidu_log, LogRow};
use crate::module::manifest::Manifest;
use crate::process_weidu_mod::{process_generated_mod, process_weidu_mod, ProcessResult};
use crate::settings::Config;
use crate::timeline::InstallTimeline;
use crate::unique_component::UniqueComponent;
use crate::weidu_conf::check_weidu_conf_lang;
use crate::weidu_context::WeiduContext;

use super::extract_manifest::extract_unique_components;

pub fn install(opts: &Install, settings: &Config, game_dir: &CanonPath, cache: &Cache) -> Result<()> {

    let manifest = Manifest::assemble_from_path(&opts.manifest_path, &opts.get_manifest_root(game_dir))?;
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

    let modules = get_modules_range(&modules, opts)?;
    if modules.is_empty() {
        info!("{}", Green.paint("Nothing to install with given range"));
        return Ok(())
    }

    let downloader = Downloader::new();
    let module_downloader = ModuleDownload::new(&settings, &manifest.global, &manifest.locations, &opts,
                                                                        &downloader, &game_dir, cache);
    let file_installer = FileInstaller::new(&manifest.global, &opts, &game_dir);

    let weidu_context = WeiduContext { current: game_dir, settings: &settings, opts: &opts,
                                                    module_downloader: &module_downloader, file_installer: &file_installer,
                                                    log: RefCell::from(log) };

    let mut timelines = vec![];
    for (index, module) in modules.iter().enumerate() {
        let real_index = index + opts.from_index.unwrap_or(0) + 1;
        info!("module {} - {}", real_index, module.describe());
        debug!("{:?}", module);

        match check_safely_installable(module)? {
            SafetyResult::Abort => bail!("Aborted"),
            SafetyResult::Safe => {}
            SafetyResult::Conflicts(matches) if matches.is_empty() => {}
            SafetyResult::Conflicts(matches) => {
                let list = format!("\n  - {}", matches.iter().map(|item| item.short_desc()).join("\n  - "));
                error!("{}", Red.bold().paint(format!("Module fragment\n  {:?}\ncontains components that were already installed:{}", module, list)));
                show_reset_help();
                bail!("Aborting - proceeding with `install` is unsafe (could uninstall then install modules repeatedly)");
            }
        }

        let process_result = match module {
            Module::Mod { weidu_mod } => {
                let result = process_weidu_mod(weidu_mod, &weidu_context, &manifest, real_index, settings)?;
                if let (true, Some(output_path)) = (module.get_components().is_ask(), &opts.record ){
                    record_selection(index, weidu_mod, &output_path, &manifest, opts.record_no_confirm)?;
                }
                result
            }
            Module::Generated { gen } => process_generated_mod(gen, &weidu_context, &manifest, real_index, settings)?,
        };

        let ProcessResult { stop: finished, timeline } = process_result;
        timelines.push(timeline);

        if finished {
            warn!("interrupted");
            timelines.push(InstallTimeline::new(lwc!("aborted"), Local::now()));
            handle_timeline(opts.timeline, &timelines);
            bail!("Program interrupted on error on non-whitelisted warning");
        } else {
            match module.exec_post_install(&module.get_name()) {
                PostInstallOutcome::Stop => {
                    info!("{}",  Blue.bold().paint(format!("Interruption requested for module {} - {}",
                                                            real_index, module.describe())));
                    return Ok(());
                }
                PostInstallOutcome::Continue => {}
            }
        }
        // Now check we actually installed all requested components
        if !opts.dry_run {
            check_install_complete(&module)?
        }
    }
    info!("Installation done with no error");
    timelines.push(InstallTimeline::new(lwc!("finished"), Local::now()));
    handle_timeline(opts.timeline, &timelines);
    Ok(())
}

fn  get_modules_range<'a>(modules: &'a[Module], opts: &Install) -> Result<&'a [Module]> {
    let from_index = match opts.from_index {
        Some(from_index) => if from_index > modules.len() {
            return Ok(&modules[0..0]);
        } else {
            from_index - 1
        }
        None => 0,
    };
    let result = match (opts.to_index, opts.just_one, opts.count) {
        (Some(to_index), false, None) => if from_index > to_index {
            return Ok(&modules[0..0]);
        } else if to_index > modules.len() {
            &modules[(from_index)..]
        } else {
            &modules[(from_index)..(to_index - 1)]
        }
        (None, true, None) => &modules[(from_index)..(from_index + 1)],
        (None, false, Some(count)) => if from_index + count > modules.len() {
            &modules[(from_index)..]
        } else {
            &modules[(from_index)..(from_index + count)]
        }
        (None, false, None) => &modules[(from_index)..],
        _ => bail!("incompatible arguments given"),
    };
    debug!("range: {:?}", result);
    Ok(result)
}

fn check_safely_installable(module: &Module) -> Result<SafetyResult> {
    let installed = extract_unique_components()?;
    match module.get_components() {
        Components::None => Ok(SafetyResult::Safe),
        Components::Ask => {
            let existing = installed.iter().filter(|comp| comp.mod_key == *module.get_name()).collect_vec();
            if !existing.is_empty() {
                let prompt = format!(r#"
                    For the next module fragment ({}), weidu will ask which components must be installed.
                    Be aware that selecting a component that was already installed will uninstall all
                    components that were installed in the meantime, reinstall this component and all the
                    following ones which can take a long time (and, possibly, break things) and is better avoided.

                    The following components for the same mod were installed:
                    {}

                    Continue?
                "#, module.get_name(), existing.iter().map(|comp| comp.index.to_string()).join(", "));
                if dialoguer::Confirm::new().with_prompt(prompt).interact()? {
                    Ok(SafetyResult::Safe)
                } else{
                    Ok(SafetyResult::Abort)
                }
            } else {
                Ok(SafetyResult::Safe)
            }
        }
        Components::List(list) => {
            let matches = list.iter().fold(vec![], |mut matches, current| {
                let current = UniqueComponent { mod_key: module.get_name().to_owned(), index: current.index() };
                if installed.contains(&current) {
                    matches.push(current);
                    matches
                } else {
                    matches
                }
            });
            if matches.is_empty() {
                Ok(SafetyResult::Conflicts(matches))
            } else {
                Ok(SafetyResult::Safe)
            }
        }
    }
}

// should show the actual reset command, with the correct index, TBD
fn show_reset_help() {
    info!("You may use the `reset` subcommand")
}

fn handle_timeline(flag: bool, timelines: &[InstallTimeline]) {
    if flag {
        info!("timelines:\n  - {}", timelines.iter().map(|it| it.short()).join("\n  - "));
    } else{
        debug!("timelines:\n  - {}", timelines.iter().map(|it| it.short()).join("\n  - "));
    }
}

pub enum SafetyResult {
    Conflicts(Vec<UniqueComponent>),
    Safe,
    Abort,
}

fn record_selection(index: usize, module: &WeiduMod, output_file: &str, original_manifest: &Manifest, no_confirm_flag: bool) -> Result<()> {
    let log_rows = parse_weidu_log(None)?;
    let output_path = PathBuf::from(output_file);
    let mut record_manifest = if output_path.exists() {
        Manifest::read_path(output_file)?
    } else {
        original_manifest.clone()
    };

    let previous_mod = record_manifest.modules[..index].iter().rev().find(|item| match item.get_components() {
        Components::List(_) => true,
        Components::Ask => true,
        Components::None => false,
    });
    debug!("record_selection- previous_mod={:?}", previous_mod);

    let selection_rows = match previous_mod {
        None => log_rows.iter().filter(|row| module.name == row.module).collect::<Vec<_>>(),
        Some(previous) => {
            let previous_components = previous.get_components();
            let previous_components = match previous_components {
                Components::List(ref list) => list,
                Components::Ask => bail!("components for previous mod fragment were not recorded"),
                Components::None => bail!("search incorrectly returned a 'none' component list"),
            };
            let previous_name = previous.get_name();
            debug!("record_selection- previous_components={:?}, previous_name={}", previous_components, previous_name);
            let previous_match = log_rows.iter().enumerate().rev().find(|(_, row)| {
                let result = previous_name == &row.module && previous_components.iter().any(|comp| comp.index() == row.component_index);
                debug!("{:?} ? {}", row, result);
                result
            });
            let last_index = match previous_match {
                None => bail!("Couldn't find components for the previous mod"),
                Some((index, _)) => index,
            };
            log_rows[(last_index + 1)..].iter().filter(|row| module.name == row.module).collect::<Vec<_>>()
        }
    };
    let selection = selection_rows.iter().map(|row|
        Component::Full(FullComponent { index: row.component_index, component_name: row.component_name.to_owned() })
    ).collect_vec();

    if confirm_record(no_confirm_flag, &selection_rows, &module.name)? {
        // update manifest with new component selection
        let components = if selection.is_empty() {
            Components::None
        } else{
            Components::List(selection)
        };
        debug!("replace {:?} at position {}", components, index);
        record_manifest.modules[index] = Module::Mod { weidu_mod: WeiduMod {
            components,
            ..module.to_owned()
        } };

        // write updated manifest to new file
        let temp_path = PathBuf::from(format!("{}.new", output_file));
        let dest = match OpenOptions::new().create(true).truncate(true).write(true).open(&temp_path) {
            Err(err) => bail!("Could not create temp output file\n  {}", err),
            Ok(file) => file,
        };
        let buf_writer = BufWriter::new(&dest);
        serde_yaml::to_writer(buf_writer, &record_manifest)?;

        // rename temp file to output file
        if let Err(error) = std::fs::rename(&temp_path, output_file) {
            bail!("Failed to rename temp output file {:?} to {:?}\n -> {:?}", temp_path, output_file, error);
        } else {
            debug!("renamed temp output file to {:?}", output_file);
        }
    }

    Ok(())
}

fn confirm_record(no_confirm_flag: bool, selection: &[&LogRow], module_name: &LwcString) -> Result<bool> {
    if no_confirm_flag {
        Ok(true)
    } else {
        let prompt = format!("Record component selection for mod {}?\n  selection:\n- {}\n",
                                    module_name, selection.iter().map(|row| format!("{} - {}", row.component_index, row.component_name)).join("\n- "));
        if dialoguer::Confirm::new().with_prompt(prompt).interact()? {
            Ok(true)
        } else{
            Ok(false)
        }
    }
}
