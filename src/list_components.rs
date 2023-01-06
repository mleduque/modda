
use std::path::PathBuf;

use anyhow::{bail, Result};

use crate::lowercase::LwcString;
use crate::tp2::find_tp2_str;
use crate::weidu::{run_weidu_list_components, WeiduComponent};

pub fn list_components(from_base: &PathBuf, module_name: LwcString, lang_index: u32) -> Result<Vec<WeiduComponent>> {
    match find_tp2_str(from_base, &module_name) {
        Err(error) => bail!("No module with name {} found - {:?}", module_name, error),
        Ok(tp2) => {
            match run_weidu_list_components(&tp2, lang_index) {
                Err(error) => bail!("Couldn't obtain component list for module {} - {:?}", module_name, error),
                Ok(list) => Ok(list),
            }
        }
    }
}
