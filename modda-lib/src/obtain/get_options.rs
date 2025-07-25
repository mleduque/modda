use clap_derive::ValueEnum;
use serde::{Deserialize, Serialize};


pub struct GetOptions {
    pub strict_replace: StrictReplaceAction
}

#[derive(Debug, Copy, Clone, ValueEnum, Deserialize, Serialize)]
pub enum StrictReplaceAction {
    Ignore,
    Fail,
    Ask,
}

impl Default for StrictReplaceAction {
    fn default() -> Self { Self::Ask }
}
