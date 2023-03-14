
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
#[skip_serializing_none]
pub struct PrecopyCommand {
    pub command: String,
    pub args: Option<Vec<String>>,
    pub subdir: Option<String>,
}
