

use std::cell::RefCell;
use std::io::BufWriter;

use ansi_term::Colour::{Blue, Green};
use anyhow::{Result, anyhow, bail};
use log::{debug, info};

use crate::args::Install;
use crate::cache::Cache;
use crate::canon_path::CanonPath;
use crate::download::Downloader;
use crate::file_installer::FileInstaller;
use crate::get_module::ModuleDownload;
use crate::file_module_install::FileModuleInstaller;
use crate::module::module::Module;
use crate::post_install::PostInstallOutcome;
use crate::log_parser::check_install_complete;
use crate::manifest::Manifest;
use crate::process_weidu_mod::{process_generated_mod, process_weidu_mod};
use crate::settings::{Config};
use crate::weidu_conf::check_weidu_conf_lang;
use crate::weidu_context::WeiduContext;

pub fn install(opts: &Install, settings: &Config, game_dir: &CanonPath, cache: &Cache) -> Result<()> {

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

    let modules = get_modules_range(&modules, opts)?;
    if modules.is_empty() {
        info!("{}", Green.paint("Nothing to install with given range"));
        return Ok(())
    }

    let downloader = Downloader::new();
    let module_downloader = ModuleDownload::new(&settings, &manifest.global, &opts,
                                                                        &downloader, &game_dir, cache);
    let file_installer = FileInstaller::new(&manifest.global, &opts, &game_dir);
    let file_module_installer = FileModuleInstaller::new(&file_installer);

    let weidu_context = WeiduContext { current: game_dir, settings: &settings, opts: &opts,
                                                    module_downloader: &module_downloader, file_installer: &file_installer,
                                                    log: RefCell::from(log) };

    for (index, module) in modules.iter().enumerate() {
        let real_index = index + opts.from_index.unwrap_or(0);
        info!("module {} - {}", real_index, module.describe());
        debug!("{:?}", module);
        let finished = match module {
            Module::Mod { weidu_mod } => process_weidu_mod(weidu_mod, &weidu_context, &manifest, real_index, settings)?,
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
        if !opts.dry_run {
            check_install_complete(&module)?
        }
    }
    info!("Installation done with no error");
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
        (None, false, None) => &modules,
        _ => bail!("incompatible arguments given"),
    };
    Ok(result)
}
