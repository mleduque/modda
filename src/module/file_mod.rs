
use serde::{Deserialize, Serialize};

use crate::lowercase::LwcString;
use crate::post_install::PostInstall;

use super::file_module_origin::FileModuleOrigin;


#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct FileModule {
    pub file_mod: LwcString,
    pub description: Option<String>,
    pub from: FileModuleOrigin,
    /// Path from game directory (location of chitin.key)
    pub to: String,
    pub post_install: Option<PostInstall>,
    #[serde(default)]
    pub allow_overwrite: bool,
}
