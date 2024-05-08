
use anyhow::{bail, Result};

use crate::lowercase::LwcString;
use crate::modda_context::WeiduContext;
use crate::tp2::find_tp2_str;
use crate::run_weidu::{run_weidu_list_components, WeiduComponent};

pub fn list_components(module_name: &LwcString, lang_index: u32, weidu_context: &WeiduContext) -> Result<Vec<WeiduComponent>> {
    match find_tp2_str(weidu_context.current_dir, &module_name) {
        Err(error) => bail!("No module with name {} found - {:?}", module_name, error),
        Ok(tp2) => {
            match run_weidu_list_components(&tp2, lang_index, weidu_context) {
                Err(error) => bail!("Couldn't obtain component list for module {} - {:?}", module_name, error),
                Ok(list) => Ok(list),
            }
        }
    }
}
