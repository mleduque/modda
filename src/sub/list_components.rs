

use anyhow::{bail, Result};
use itertools::Itertools;

use crate::args::ListComponents;
use crate::canon_path::CanonPath;
use crate::list_components::list_components;
use crate::settings::Config;

pub fn sub_list_components(param: &ListComponents, game_dir: &CanonPath, config: &Config) -> Result<()> {
    match list_components(game_dir, &param.module_name, param.lang, config) {
        Err(error) => bail!("Couldn't obtain component list for module {}\n-> {:?}", param.module_name, error),
        Ok(list) => {
            println!("{}", list.iter().map(|comp| format!("{} - {}", comp.number, comp.name)).join("\n"));
            Ok(())
        }
    }
}
