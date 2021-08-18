

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum PatchSource {
    /// Inline content inside the yaml file
    Inline { inline: String },
    /// Downloaded from an HTTP resource
    Http { http: String },
    /// From a local file ; path is relative and searched in the order:
    /// - in the same location asthemanifest file (or subdirs)
    /// - in thegame dir (or subdirs)
    Relative { relative: String },
    // /// From any file on the filesystem
    //Path { path: String },
}
