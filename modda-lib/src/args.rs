
use std::path::PathBuf;

use clap_derive::{Parser, Subcommand, Args};

use crate::canon_path::CanonPath;
use crate::lowercase::LwcString;
use crate::obtain::get_options::StrictReplaceAction;
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
    /// Append all components of a mod to a manifest. This can result in an uninstallable mod (incompatible components, GROUPs etc.)
    /// so it should probably manually edited.
    AppendMod(AppendMod),
    /// Tries to uninstall modules that are after a given index in the manifest (EXPERIMENTAL).
    Reset(Reset),
    /// Discovers mods in the game directory and builds a manifest skeleton.
    Discover(Discover),
    /// Show configuration/settings information.
    Introspect(Introspect),

    /// Works with the global configuration
    #[clap(subcommand)]
    GlobalConfig(ConfigArgs)
}

impl Commands {
    pub fn wants_chitin_key(&self) -> bool {
        match self {
            Commands::Install(..) => true,
            Commands:: Search(..) => false,
            Commands::ListComponents(..) => true,
            Commands::Invalidate(..) => false,
            Commands::Reverse(..) => true,
            Commands::AppendMod(..) => true,
            Commands::Reset(..) => true,
            Commands::Discover(..) => true,
            Commands::Introspect(..) => true,
            Commands::GlobalConfig(variant) => match variant {
                ConfigArgs::Edit(..) => false,
                ConfigArgs::Show(..) => false,
            }
        }
    }
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
    #[arg(long, short = 't', group="limit")]
    pub to_index: Option<usize>,

    /// Tells to only install one mod fragment.
    #[arg(long, short = 'j', group="limit")]
    pub just_one: bool,

    /// Tells to only install `count` mod fragments.
    #[arg(long, short = 'c', group="limit")]
    pub count: Option<usize>,

    /// name of a file where the output will be written.
    #[arg(long, short = 'o')]
    pub output: Option<String>,

    /// If set to true, the mods will be downloaded and copied in the game directory, but not actually installed.
    #[arg(long, short = 'd')]
    pub dry_run: bool,

    /// If set, prints duration information.
    #[arg(long)]
    pub timeline: bool,

    /// If set, records the weidu component selection for mod fragments with `components:ask`.
    #[arg(long, short)]
    pub record: Option<String>,

    /// If set along with `--record`, will not ask for confirmation before recording.
    #[arg(long, requires = "record")]
    pub record_no_confirm: bool,

    /// If set along with `--record`, will not ask for confirmation before recording.
    #[arg(long, requires = "record")]
    pub record_with_comment_as_field: bool,

    /// Decides what to do if a replace action has a `strict` property that is not obeyed.<br>
    #[arg(long, default_value = "ask")]
    pub check_replace: StrictReplaceAction,
}

impl Install {
    pub fn get_manifest_root(&self, game_dir: &CanonPath) -> CanonPath {
        let manifest = PathBuf::from(&self.manifest_path);
        match manifest.parent() {
            None => game_dir.to_owned(),
            Some(path) => CanonPath::new(path).unwrap_or_else(|_| game_dir.to_owned()),
        }
    }
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
    pub module_name: LwcString,

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

#[derive(Args, Debug)]
pub struct AppendMod {
    /// Name of the file that will be appended to.
    /// The file is created if it doesn't exist.
    #[arg(long, short)]
    pub output: String,

    #[arg(long, short)]
    pub r#mod: LwcString,

    /// If set, the component names will be generated (default: `true`).
    #[arg(long, short = 'l')]
    pub language: Option<String>,

    /// If set, the component names will be generated (default: `true`).
    #[arg(long, short = 'c')]
    pub export_component_name: Option<bool>,
}


#[derive(Args, Debug)]
pub struct Reset {

    /// Path of the YAML manifest file.
    #[arg(long, short)]
    pub manifest_path: String,

    /// Index to reset to.
    /// zero means before the first module.
    #[arg(long, short)]
    pub to_index: usize,

    /// If set, doesn't actually run the weidu command, only prints what would be executed
    #[arg(long, short)]
    pub dry_run: bool,
}

#[derive(Args, Debug)]
pub struct Discover {
    /// Name of the file that will be generated.
    #[arg(long, short)]
    pub output: String,
}

#[derive(Args, Debug)]
pub struct Introspect {
    /// Name of the file that will be generated.
    #[arg(long, short)]
    pub show_config: bool,
}

#[derive(Debug, Subcommand)]
pub enum ConfigArgs {
    /// Show the global configuration (opens the directory that contains the global configuration file)
    Show(ConfigShow),
    /// Open the global configuration file.<br>
    /// If it doesn't exist yet, it will create an default configuration file.
    Edit(ConfigEdit),
}

#[derive(Args, Debug)]
pub struct ConfigShow {}

#[derive(Args, Debug)]
pub struct ConfigEdit {}
