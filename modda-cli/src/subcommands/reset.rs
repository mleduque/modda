
use anyhow::{Result, bail};
use itertools::Itertools;
use log::info;

use modda_lib::args::Reset;
use modda_lib::modda_context::WeiduContext;
use modda_lib::module::components::Components;
use modda_lib::module::manifest::Manifest;
use modda_lib::module::module::Module;
use modda_lib::run_weidu::run_weidu_uninstall;
use modda_lib::sub::extract_manifest::extract_bare_mods;
use modda_lib::tp2::find_tp2_str;


pub fn reset(args: &Reset, weidu_context: &WeiduContext) -> Result<()> {
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
        Module::Generated { generated } => generated.as_weidu(),
    };
    let components = match weidu_mod.components {
        Components::None => bail!("Can't reset to a module fragment which doesn't install components (`components: none`)"),
        Components::Ask => bail!("Can't reset to a module fragment which doesn't specify components explicitly (`components: ask`)"),
        Components::All => bail!("Can't reset to a module fragment which doesn't specify components explicitly (`components: all`)"),
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
            let tp2 = find_tp2_str(weidu_context.current_dir, &fragment.name)?;
            run_weidu_uninstall(&tp2, fragment, args, weidu_context)?;
        }
        Ok(())
    } else {
        info!("Aborted");
        Ok(())
    }
}
