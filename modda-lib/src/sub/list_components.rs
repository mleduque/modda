
use anyhow::{bail, Result};
use itertools::Itertools;

use crate::args::ListComponents;
use crate::list_components::list_components;
use crate::modda_context::WeiduContext;

pub fn sub_list_components(param: &ListComponents, weidu_context: &WeiduContext) -> Result<()> {
    match list_components(&param.module_name, param.lang, weidu_context) {
        Err(error) => bail!("Couldn't obtain component list for module {}\n-> {:?}",
                                    param.module_name, error),
        Ok(list) => {
            println!("{}", list.iter().map(|comp| format!("{} - {}", comp.number, comp.name)).join("\n"));
            Ok(())
        }
    }
}
