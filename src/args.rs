
use clap_derive::{Parser, Subcommand, Args};

use crate::progname::PROGNAME;


#[derive(Parser, Debug)]
#[command(name = PROGNAME)]
#[command(author, version)]
#[command(about = "Weidu install automation", long_about = None)]
pub struct Cli {

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// install mods.
    Install(Install),
    /// Search all module declarations in the manifest with the given name.
    Search(Search),
    /// List the available components of a weidu mod (by index).
    ListComponents(ListComponents),
    /// Remove a downloaded mod from the cache
    Invalidate(Invalidate),
    /// generate a skeleton manifest YAML file from a `weidu.log` file.
    Reverse(Reverse),
}

#[derive(Args, Debug, Default)]
pub struct Install {

    /// Path of the YAML manifest file.
    #[arg(long, short)]
    pub manifest_path: String,

    /// If set to true, will not stop when weidu returns a warning.
    #[arg(long)]
    pub no_stop_on_warn: bool,

    /// Index in the module list where we start (counting from *one*).
    #[arg(long, short = 'f')]
    pub from_index: Option<usize>,

    /// Index in the module list where we stop (excluded, counting from one).
    #[arg(long, short = 't')]
    pub to_index: Option<usize>,

    /// name of a file where the output will be written.
    #[arg(long, short = 'o')]
    pub output: Option<String>,

    /// If set to true, the mods will be downloaded and copied in the game directory, but not actually installed.
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Args, Debug)]
pub struct Search {

    /// Path of the YAML manifest file.
    #[arg(long, short)]
    pub manifest_path: String,

    /// Name of the module we want to find.
    #[arg(long, short)]
    pub name: String,
}

#[derive(Args, Debug)]
pub struct ListComponents {
    /// Name of the module we want to find.
    pub module_name: String,

    /// Language we want the component names to appear in.
    #[arg(long, short)]
    pub lang: u32,
}

#[derive(Args, Debug)]
pub struct Invalidate {

    /// Path of the YAML manifest file.
    #[arg(long, short)]
    pub manifest_path: String,

    /// Name of the mod that must be removed from cache.
    #[arg(long, short)]
    pub name: String,
}

#[derive(Args, Debug)]
pub struct Reverse {
    /// Name of the file that will be generated.
    #[arg(long, short)]
    pub output: String,

    /// If set, the `language` field in mod definitions will be generated (default: `false`).
    #[arg(long, short = 'l')]
    pub export_language: Option<bool>,

    /// If set, the component names will be generated (default: `true`).
    #[arg(long, short = 'c')]
    pub export_component_name: Option<bool>,
}
