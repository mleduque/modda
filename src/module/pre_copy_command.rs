
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
pub struct PrecopyCommand {
    pub command: String,
    pub args: Option<Vec<String>>,
    pub subdir: Option<String>,
}
