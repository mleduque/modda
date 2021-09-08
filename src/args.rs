
use clap::{AppSettings, Clap};


#[derive(Clap, Debug)]
#[clap(version = "1.0")]
#[clap(setting = AppSettings::ColoredHelp)]
pub enum Opts {
    Install(Install),
    Search(Search),
    ListComponents(ListComponents),
    Invalidate(Invalidate),
    Uninstall(Uninstall),
}

#[derive(Clap, Debug)]
pub struct Install {

    #[clap(long, short)]
    pub manifest_path: String,

    #[clap(long)]
    pub no_stop_on_warn: bool,

    /// index in the module list where we start (counting from *one*)
    #[clap(long, short = 'f')]
    pub from_index: Option<usize>,

    /// index in the module list where we stop
    #[clap(long, short = 't')]
    pub to_index: Option<usize>,

    #[clap(long, short = 'o')]
    pub output: Option<String>,


    #[clap(long)]
    pub dry_run: bool,
}

#[derive(Clap, Debug)]
pub struct Search {

    #[clap(long, short)]
    pub manifest_path: String,


    #[clap(long, short)]
    pub name: String,
}

#[derive(Clap, Debug)]
pub struct ListComponents {
    pub module_name: String,

    #[clap(long, short)]
    pub lang: u32,
}

#[derive(Clap, Debug)]
pub struct Invalidate {

    #[clap(long, short)]
    pub manifest_path: String,

    #[clap(long, short)]
    pub name: String,
}

#[derive(Clap, Debug)]
pub struct Uninstall {

    #[clap(long, short)]
    pub manifest_path: String,

    #[clap(long)]
    pub dry_run: bool,

    #[clap(flatten)]
    pub what: UninstallTarget,
}

#[derive(Clap, Debug)]
pub enum UninstallTarget {
    Mod(ModTarget),
    Components(ComponentsTarget),
}

#[derive(Clap, Debug)]
pub struct ModTarget {
    index: u32,
}

#[derive(Clap, Debug)]
pub struct ComponentsTarget {
    #[clap(long, short)]
    index: u32,

    #[clap(long, short)]
    components: Vec<u32>,
}
