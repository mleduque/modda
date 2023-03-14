
use serde::{Deserialize, Serialize};


#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct ModuleConf {
    pub file_name:String,
    #[serde(flatten)]
    pub content: ModuleContent,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum ModuleContent {
    /// The actual content of the file is provided
    Content { content: String },
    /// Interrupt and ask the user to input the content (value of `prompt` is shown)
    Prompt { prompt: String },
}
