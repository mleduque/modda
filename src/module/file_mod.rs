
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::lowercase::LwcString;
use crate::post_install::PostInstall;

use super::file_module_origin::FileModuleOrigin;


#[skip_serializing_none]
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
