
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum FileModuleOrigin {
    /// A path in manifest `global.local_files`
    /// Interpreted as glob from this location.
    Local {
        local: String,
        glob: Option<String>,
    },
    /// Any path on the computer.
    Absolute {
        absolute: String,
        glob: Option<String>,
    },
}

impl FileModuleOrigin {
    pub fn glob(&self) -> Option<&str> {
        match self {
            Self::Local { glob, .. } => glob.as_ref().map(|glob| glob.as_str()),
            Self::Absolute { glob, .. } => glob.as_ref().map(|glob| glob.as_str()),
        }
    }
}
