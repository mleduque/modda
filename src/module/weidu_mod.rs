
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::components::{Components, Component, FullComponent};
use crate::lowercase::LwcString;
use crate::post_install::PostInstall;

use super::install_comment::InstallationComments;
use super::location::Location;
use super::module_conf::ModuleConf;

/** Definition of a mod. */
#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, PartialEq, Default, Clone)]
pub struct WeiduMod {
    /**
     * Unique identifier of a mod.
     * This is the weidu mod name: name of the tp2 file without `setup-` ot the tp2 extension.
     * This is also the name as used in `weidu.log`.
     * This is case-insensitive.
     */
    pub name: LwcString,
    /// Unused at the moment
    pub version: Option<String>,
    /// Optional description, used to disambiguate multiple occurrences of the same mod
    pub description: Option<String>,
    /// Which language index to use (has precedence over manifest-level lang_prefs)
    pub language: Option<u32>,
    /// List of components to be auto-installed.
    /// Can be `ask`, `none`, a list of components or absent/not set/null (which is the same as `ask`)
    ///   - `ask` (or empty) will use weidu in interactive mode (weidu itself asks how to install components)
    ///   - `none` will just copy the mod files in the game dir without installing anything
    ///   - a list of components will call weidu and provide the list of components on the command line
    #[serde(deserialize_with = "crate::components::component_deser")]
    pub components: Components,
    /// Whether warnings returned by weidu (exit code) will interrupt the whole installation.
    ///
    /// (defaults to _not_ ignoring warnings)..
    /// - If set to true, warning are ignored and the installation proceeds with the following mods
    /// - If set to false (or absent), weidu warnings will stop the installation.
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub ignore_warnings: bool,
    pub add_conf: Option<ModuleConf>,
    /// Where we can obtain the module.
    ///
    /// If absent, it is assumed to be in the game install.
    /// In that case, it checks a `<mod_name.tp2>`,`setup-mod_name>.tp2` in the game dir and in
    /// `<nod_name>` sub-directory. If it is not found, the installation aborts.
    pub location: Option<Location>,
    /// Decides what will be done after the mod installation (in case of success).
    /// - `interrupt` will stop the installation and exist the program
    /// - `wait_seconds: xxx will wait xxx second before continuing to the next mod
    /// - `none` (the default) immediately starts the next mod installation.
    pub post_install: Option<PostInstall>,

    // Below: unused (ATM), sort of inert metadata
    pub comment: Option<String>,
    pub original_thread: Option<String>,
    pub original_dl: Option<String>,
    pub installation: Option<InstallationComments>,
}

fn is_false(value: &bool) -> bool { !value }

pub struct BareMod {
    pub name: LwcString,
    pub components: Vec<FullComponent>,
    pub language: u32,
}

impl BareMod {
    pub fn to_weidu_mod(&self, export_component_name: Option<bool>, export_language: Option<bool>) -> WeiduMod {
        let components = match export_component_name {
            Some(false) => self.components.iter().map(|comp| Component::Simple(comp.index)).collect(),
            _ => self.components.iter().map(|comp| Component::Full(comp.clone())).collect(),
        };
        WeiduMod {
            name: self.name.to_owned(),
            components: Components::List(components),
            language: if let Some(true) = export_language { Some(self.language) } else { None },
            ..Default::default()
        }
    }

    pub fn short(&self) -> String {
        format!("{}: {}", self.name,self.components.iter().map(|comp| comp.index).join(", "))
    }
}
