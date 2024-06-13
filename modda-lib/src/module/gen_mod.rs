
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::module::components::{Component, Components};
use crate::lowercase::LwcString;
use crate::post_install::PostInstall;

use super::disable_condition::DisableCondition;
use super::file_module_origin::FileModuleOrigin;
use super::weidu_mod::WeiduMod;

/// Generates a skeleton weidu mod that just copies a bunch of files into `games/override`
#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, PartialEq, Default, Clone)]
pub struct GeneratedMod {
    pub gen_mod: LwcString,
    pub description: Option<String>,
    /// files that will be copied by the gene rated mod
    pub files: Vec<FileModuleOrigin>,
    #[serde(default)]
    pub post_install: Option<PostInstall>,
    #[serde(default)]
    pub component: GenModComponent,
    #[serde(default)]
    pub ignore_warnings: bool,
    #[serde(default)]
    pub allow_overwrite: bool,
    /// Condition that disables the mod installation (if absent, not disabled)
    pub disabled_if: Option<DisableCondition>,
}

impl GeneratedMod {
    pub fn as_weidu(&self) -> WeiduMod {
        WeiduMod {
            name: self.gen_mod.clone(),
            description: self.description.clone(),
            components: Components::List(vec![
                Component::Simple(self.component.index),
            ]),
            ignore_warnings: self.ignore_warnings,
            post_install: self.post_install.clone(),
            ..Default::default()
        }
    }
}
#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, PartialEq, Default, Clone)]
pub struct GenModComponent {
    #[serde(default)]
    pub index: u32,
    pub name: Option<String>,
}
