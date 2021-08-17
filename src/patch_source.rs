

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum PatchSource {
    Inline { inline: String },
    Http { http: String }
}
