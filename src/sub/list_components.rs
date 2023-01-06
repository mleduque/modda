

use anyhow::{bail, Result};
use itertools::Itertools;

use crate::args::ListComponents;
use crate::list_components::list_components;
use crate::lowercase::lwc;

pub fn sub_list_components(param: &ListComponents) -> Result<()> {
    let current = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(error) => bail!("Failed to obtain current directory\n -> {:?}", error),
    };
    match list_components(&current, lwc!(&param.module_name), param.lang) {
        Err(error) => bail!("Couldn't obtain component list for module {}\n-> {:?}", param.module_name, error),
        Ok(list) => {
            println!("{}", list.iter().map(|comp| format!("{} - {}", comp.number, comp.name)).join("\n"));
            Ok(())
        }
    }
}
