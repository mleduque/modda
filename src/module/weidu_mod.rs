
use serde::{Deserialize, Serialize};

use crate::components::Components;
use crate::location::Location;
use crate::lowercase::LwcString;
use crate::post_install::PostInstall;

use super::install_comment::InstallationComments;
use super::module_conf::ModuleConf;

/** Definition of a mod. */
#[derive(Deserialize, Serialize, Debug, PartialEq, Default)]
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
    ///   - `none` will just copy the mod filesin the game dir without installing anything
    ///   - a list of components will call weidu and provide the list of components on the command line
    #[serde(deserialize_with = "crate::components::component_deser")]
    pub components: Components,
    /// Whether warnings returned by weidu (exit code) will interrupt the whole installation.
    ///
    /// (defaults to _not_ ignoring warnings)..
    /// - If set to true, warning are ignored and the installation proceed with the following mods
    /// - If set to false (or absent), weidu warnings will stop the installation.
    #[serde(default)]
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
    #[serde(default)]
    pub post_install: Option<PostInstall>,

    // Below: unused, sort of inert metadata
    pub comment: Option<String>,
    pub original_thread: Option<String>,
    pub original_dl: Option<String>,
    pub installation: Option<InstallationComments>,
}
