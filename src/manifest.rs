
use std::borrow::Cow;
use std::io::BufReader;
use std::path::PathBuf;
use serde::Deserialize;

use anyhow::{bail, Result};

use crate::archive_layout::Layout;

#[derive(Deserialize, Debug)]
pub struct Manifest {
    pub global: Global,
    pub modules: Vec<Module>,
}

#[derive(Deserialize, Debug)]
pub struct Global {
    #[serde(rename = "lang_dir")]
    pub game_language: String,

    /// List of language _names_ that should be selected if available, in decreasing order of priority
    /// items in the list are used as regexp (case insensitive by default)
    /// - the simplest case is just putting the expected language names 
    ///   ex. [français, french, english]
    /// - items in the list that start with `#rx#`are interpreted as regexes
    ///   syntax here https://docs.rs/regex/1.5.4/regex/#syntax
    ///   ex. ["#rx#^fran[cç]ais", french, english]
    pub lang_preferences: Option<Vec<String>>,
}

#[derive(Deserialize, Debug)]
pub struct Module {
    pub name: String,
    /// Unused at the moment
    pub version: Option<String>,
    /// Optional description, used to disambiguate multiple occurrences of the same mod
    pub description: Option<String>,
    /// Which language index to use (has precedence over manifest-level lang_prefs)
    pub language: Option<u32>,
    /// List of components to be auto-installed. In None or empty list, run interactively
    pub components: Option<Vec<Component>>,
    #[serde(default)]
    pub ignore_warnings: bool,
    pub add_conf: Option<ModuleConf>,
    /// Where we can obtain the module. If absent, it is assumed to be in the game install
    pub location: Option<Location>,
}
impl Module {
    pub fn describe(&self) -> Cow<String> {
        match &self.description {
            None => Cow::Borrowed(&self.name),
            Some(desc) => Cow::Owned(format!("{} ({})", self.name, desc)),
        }
    }

    pub fn components_with_warning(&self) -> Vec<&Component> {
        match &self.components {
            None => vec![],
            Some(components) => components.iter().filter(|comp| match comp {
                Component::Simple(_) => false,
                Component::Full{ ignore_warn, ..} =>  *ignore_warn,
            }).collect::<Vec<_>>(),
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum Component {
    Simple(u32),
    Full { index: u32, ignore_warn: bool },
}
impl Component {
    pub fn index(&self) -> u32 {
        match &self {
            Component::Simple(index) => *index,
            Component::Full { index, ..} => *index,
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct ModuleConf {
    pub file_name:String,
    pub content: ModuleContent,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum ModuleContent {
    Content(String),
    Prompt(String),
}

#[derive(Deserialize, Debug)]
pub struct Location {
    pub source: Source,
    pub cache_name: Option<String>,
    /// Specifies which files from the archive will be copied to the game directory.
    /// Read as a Unix shell style glob pattern (https://docs.rs/glob/0.3.0/glob/struct.Pattern.html)
    #[serde(default)]
    pub layout: Layout,
    pub patch: Option<Source>,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum Source {
    Http { http: String, rename: Option<String> },
    Local { path: String },
    Github { github_user: String, repository: String, descriptor: GithubDescriptor },
}

impl Source {
    pub fn save_subdir(&self) -> Result<PathBuf> {
        use Source::*;
        use url::{Url, Host};
        match self {
            Http { ref http, .. } => {
                let url = match Url::parse(http) {
                    Ok(url) => url,
                    Err(error) => bail!("Couldn't parse location url {}\n -> {:?}", http, error),
                };
                let host = match url.host() {
                    None => bail!("Invalid http source {}", http),
                    Some(Host::Domain(ref domain)) => Cow::Borrowed(*domain),
                    Some(Host::Ipv6(ref ipv6)) => Cow::Owned(ipv6.to_string()),
                    Some(Host::Ipv4(ref ipv4)) => Cow::Owned(ipv4.to_string()),
                };
                Ok(PathBuf::from("http").join(&*host))
            }
            Local { .. } => Ok(PathBuf::new()),
            Github { github_user, repository, .. } => 
                Ok(PathBuf::from("github").join(github_user).join(repository))
        }
    }

    pub fn save_name(&self, module_name:&str) -> Result<PathBuf> {
        use Source::*;
        match self {
            Http { ref http, ref rename } => {
                match rename {
                    Some(rename) => Ok(PathBuf::from(rename)),
                    None => {
                        let url = match url::Url::parse(http) {
                            Err(error) => bail!("Couldn't parse url {}\n -> {:?}", http, error),
                            Ok(url) => url,
                        };
                        match url.path_segments() {
                            None => bail!("Couldn't decide archive name for url {} - provide one with 'rename' field", http),
                            Some(segments) => match segments.last() {
                                Some(seg) => Ok(PathBuf::from(
                                    percent_encoding::percent_decode_str(seg).decode_utf8_lossy().into_owned()
                                )),
                                None => bail!("Couldn't decide archive name for url {} - provide one with 'rename' field", http),
                            }
                        }
                    }
                }
            }
            Local { .. } => Ok(PathBuf::new()),
            Github { descriptor, .. } => match descriptor {
                GithubDescriptor::Release { artifact_name , ..} => 
                                                    Ok(PathBuf::from(artifact_name.to_owned())),
                GithubDescriptor::Commit { commit } => 
                                                    Ok(PathBuf::from(format!("{}-{}.zip",module_name, commit))),
                GithubDescriptor::Branch { branch } => 
                                                    Ok(PathBuf::from(format!("{}-{}.zip",module_name, branch))),
                GithubDescriptor::Tag { tag } => Ok(PathBuf::from(format!("{}-{}.zip",module_name, tag))),
            }
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum GithubDescriptor {
    Release { release: Option<String>, artifact_name: String },
    Commit { commit: String },
    Branch { branch: String },
    Tag { tag: String },
}

impl GithubDescriptor {
    pub fn get_url(&self, user: &str, repository: &str) -> String {
        use GithubDescriptor::*;
        match self {
            Release { release, artifact_name } => {
                let release = match &release {
                    None => String::from("latest"),
                    Some(release) => release.to_owned(),
                };
                format!("https://github.com/{user}/{repo}/releases/download/{release}/{artifact}",
                    user = user,
                    repo = repository,
                    release = release,
                    artifact = artifact_name.replace("{{release}}", &release),
                )
            }
            Tag { tag } =>
                format!("https://github.com/{user}/{repo}/archive/refs/tags/{tag}.zip",
                    user = user,
                    repo = repository,
                    tag = tag,
                ),
            Branch { branch } =>
                format!("https://github.com/{user}/{repo}/archive/refs/heads/{branch}.zip",
                    user = user,
                    repo = repository,
                    branch = branch,
                ),
            Commit { commit } =>
                format!("https://github.com/{user}/{repo}/archive/refs/{commit}.zip",
                    user = user,
                    repo = repository,
                    commit = commit,
                ),
        }
    }
}

pub fn read_manifest(path: &str) -> Result<Manifest> {
    let file = match std::fs::File::open(path) {
        Err(error) => bail!("Could not open manifest file {} - {:?}", path, error),
        Ok(file) => file,
    };
    let reader = BufReader::new(file);
    let manifest: Manifest = serde_yaml::from_reader(reader)?;
    Ok(manifest)
}
