use serde::{Deserialize, Serialize};



#[derive(Deserialize, Serialize, Debug, PartialEq, Default)]
pub struct Global {
    /// The "language code" configured in the game e.g. en_US, es_ES, fr_FR
    #[serde(rename = "lang_dir")]
    pub game_language: String,

    /// List of language _names_ that should be selected if available, in decreasing order of priority
    /// items in the list are used as regexp (case insensitive by default)
    /// - the simplest case is just putting the expected language names
    ///   ex. `[français, french, english]`
    /// - items in the list that start with `#rx#`are interpreted as regexes
    ///   syntax here https://docs.rs/regex/1.5.4/regex/#syntax
    ///   ex. `["#rx#^fran[cç]ais", french, english]`
    pub lang_preferences: Option<Vec<String>>,
    #[serde(default)]
    pub patch_path: Option<String>,
    /// Path from manifest root (yml file location directory) where "local" mods can be found.
    #[serde(default)]
    pub local_mods: Option<String>,
    #[serde(default)]
    pub local_files: Option<String>,
}
