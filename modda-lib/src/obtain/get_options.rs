use clap_derive::ValueEnum;


pub struct GetOptions {
    pub strict_replace: StrictReplaceAction
}

#[derive(Debug, Copy, Clone, ValueEnum)]
pub enum StrictReplaceAction {
    Ignore,
    Fail,
    Ask,
}

impl Default for StrictReplaceAction {
    fn default() -> Self { Self::Ask }
}
