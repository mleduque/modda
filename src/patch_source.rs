

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum PatchSource {
    None,
    Inline { inline: String },
    Http { http: String }
}

impl Default for PatchSource {
    fn default() -> Self { PatchSource::None }
}
