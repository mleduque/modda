
use anyhow::{Result, bail};
use itertools::Itertools;
use log::info;

use crate::args::Reset;
use crate::canon_path::CanonPath;
use crate::module::components::Components;
use crate::module::manifest::Manifest;
use crate::module::module::Module;
use crate::run_weidu::run_weidu_uninstall;
use crate::settings::Config;
use crate::sub::extract_manifest::extract_bare_mods;
use crate::tp2::find_tp2_str;


pub fn reset(args: &Reset, game_dir: &CanonPath, config: &Config) -> Result<()> {
    let installed = extract_bare_mods()?;
    let manifest = Manifest::read_path(&args.manifest_path,)?;

    let reset_index = args.to_index;

    // ensure the index actually exists in the manifest
    let target_module = match manifest.modules.get(reset_index) { // to_index is the first fragment that should be removed
        None => bail!("No module at index {} in manifest (last index is {})", reset_index, manifest.modules.len()),
        Some(module) => module,
    };
    let weidu_mod = match target_module {
        Module::Mod { weidu_mod } => weidu_mod.clone(),
        Module::Generated { gen } => gen.as_weidu(),
    };
    let components = match weidu_mod.components {
        Components::None => bail!("Can't reset to a module fragment which doesn't install components (`components: none`)"),
        Components::Ask => bail!("Can't reset to a module fragment which doesn't specify components explicitly (`components: ask`)"),
        Components::List(list) if list.is_empty() => bail!("Can't reset to a module fragment which doesn't install components (`components list is empty`)"),
        Components::List(ref list) => list,
    };
    let name_matches = installed.iter().enumerate()
        .filter(|(_, module)| module.name == weidu_mod.name)
        .collect::<Vec<_>>();
    if name_matches.is_empty() {
        info!("Nothing to remove, next mod at position {} ({}) was not installed", reset_index + 1, target_module.get_name());
        return Ok(());
    }
    let component_matches = name_matches.iter().filter(|(_, module)|
        components.iter().all(|comp| module.components.iter().any(|item| item.index == comp.index()))
    ).collect::<Vec<_>>();

    let (index, _) = match component_matches.as_slice() {
        &[] => bail!("Mod fragment was not installed (or was not found)"),
        &[single_match] => single_match,
        _=> bail!("Found multiple occurrences of mod/component in weidu.log - aborting reset"),
    };
    let removed = &installed[*index..];
    let prompt = format!("Will uninstall these (in reverse order)\n  {}\nProceed? ", removed.iter().map(|item| item.short()).join("\n  "));
    if dialoguer::Confirm::new().with_prompt(prompt).interact()? {
        for fragment in removed.iter().rev() {
            let tp2 = find_tp2_str(game_dir, &fragment.name)?;
            run_weidu_uninstall(&tp2, fragment, config, args)?;
        }
        Ok(())
    } else {
        info!("Aborted");
        Ok(())
    }
}
