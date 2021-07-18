
use std::borrow::Cow;
use std::io::{BufReader, Seek, SeekFrom};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

use anyhow::{bail, Result};

use crate::archive_layout::Layout;

#[derive(Deserialize, Serialize, Debug)]
pub struct VersionDetect {
    pub version: String,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct Manifest {
    pub version: String,
    pub global: Global,
    #[serde(default)]
    pub modules: Vec<Module>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
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

#[derive(Deserialize, Serialize, Debug, PartialEq, Default)]
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

#[derive(Deserialize, Serialize, Debug, PartialEq)]
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

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct ModuleConf {
    pub file_name:String,
    pub content: ModuleContent,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum ModuleContent {
    Content(String),
    Prompt(String),
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct Location {
    #[serde(flatten)]
    pub source: Source,
    /// Specifies which files from the archive will be copied to the game directory.
    /// Read as a Unix shell style glob pattern (https://docs.rs/glob/0.3.0/glob/struct.Pattern.html)
    #[serde(default, flatten)]
    pub layout: Layout,
    pub patch: Option<Source>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum Source {
    Http { http: String, rename: Option<String> },
    Local { path: String },
    Github(Github),
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
            Github(self::Github { github_user, repository, .. }) =>
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
            Github(self::Github { descriptor, .. }) => match descriptor {
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

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct Github {
    pub github_user: String,
    pub repository: String,
    #[serde(flatten)]
    pub descriptor: GithubDescriptor
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
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
    let mut file = match std::fs::File::open(path) {
        Err(error) => bail!("Could not open manifest file {} - {:?}", path, error),
        Ok(file) => file,
    };
    {
        let reader = BufReader::new(&file);
        let version: VersionDetect = serde_yaml::from_reader(reader)?;
        if version.version != "1" {
            bail!("Only manifest version 1 is supported for now.");
        }
    }
    let _ = file.seek(SeekFrom::Start(0))?;
    let reader = BufReader::new(&file);
    let manifest: Manifest = match serde_yaml::from_reader(reader) {
        Ok(manifest) => manifest,
        Err(error) => bail!("Failed to parse manifest at {}\n -> {}", path, error),
    };
    Ok(manifest)
}


#[cfg(test)]
mod test_deserialize {
    use super::{Source, GithubDescriptor};

    #[test]
    fn deserialize_source_github_branch() {
        use crate::manifest::Github;
        let yaml = r#"
        github_user: my_user
        repository: my_repo
        branch: main
        "#;
        let source: Source = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            source,
            Source::Github(Github {
                github_user: "my_user".to_string(),
                repository: "my_repo".to_string(),
                descriptor: GithubDescriptor::Branch {
                    branch: "main".to_string(),
                },
            })
        );
    }

    #[test]
    fn deserialize_source_github_tag() {
        use crate::manifest::Github;
        let yaml = r#"
        github_user: my_user
        repository: my_repo
        tag: v1.0
        "#;
        let source: Source = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            source,
            Source::Github(Github {
                github_user: "my_user".to_string(),
                repository: "my_repo".to_string(),
                descriptor: GithubDescriptor::Tag {
                    tag: "v1.0".to_string(),
                },
            })
        );
    }

    #[test]
    fn deserialize_source_github_committag() {
        use crate::manifest::Github;
        let yaml = r#"
        github_user: my_user
        repository: my_repo
        commit: 0123456789abcdef
        "#;
        let source: Source = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            source,
            Source::Github(Github {
                github_user: "my_user".to_string(),
                repository: "my_repo".to_string(),
                descriptor: GithubDescriptor::Commit {
                    commit: "0123456789abcdef".to_string(),
                },
            })
        );
    }

    #[test]
    fn deserialize_source_github_release() {
        use crate::manifest::Github;
        let yaml = r#"
        github_user: my_user
        repository: my_repo
        release: "1.0"
        artifact_name: my_repo-1.0.zip
        "#;
        let source: Source = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            source,
            Source::Github(Github {
                github_user: "my_user".to_string(),
                repository: "my_repo".to_string(),
                descriptor: GithubDescriptor::Release {
                    release: Some("1.0".to_string()),
                    artifact_name: "my_repo-1.0.zip".to_string(),
                },
            })
        );
    }

    #[test]
    fn deserialize_source_github_branch_as_json() {
        use crate::manifest::Github;
        let yaml = r#"{
        "github_user": "my_user",
        "repository": "my_repo",
        "branch": "main"
        }"#;
        let source: Source = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            source,
            Source::Github(Github {
                github_user: "my_user".to_string(),
                repository: "my_repo".to_string(),
                descriptor: GithubDescriptor::Branch {
                    branch: "main".to_string(),
                },
            })
        );
    }

    #[test]
    fn deserialize_module() {
        use crate::manifest::{Module, Location, Source, Layout, Github, Component};
        let yaml = r#"
        name: DlcMerger
        location:
            github_user: Argent77
            repository: A7-DlcMerger
            release: v1.3
            artifact_name: lin-A7-DlcMerger-v1.3.zip
            strip_leading: 3
            layout_type: single_dir
        components:
            - 1
        "#;
        let module: Module = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            module,
            Module {
                name: "DlcMerger".to_string(),
                components: Some(vec! [ Component::Simple(1) ]),
                location: Some(Location {
                    source: Source::Github(Github {
                        github_user: "Argent77".to_string(),
                        repository: "A7-DlcMerger".to_string(),
                        descriptor: GithubDescriptor::Release {
                            release: Some("v1.3".to_string()),
                            artifact_name: "lin-A7-DlcMerger-v1.3.zip".to_string(),
                        },
                    }),
                    layout: Layout::single_dir(3),
                    patch: None,
                }),
                ..Module::default()
            }
        );
    }


    #[test]
    fn check_read_manifest() {
        let manifest_path = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), "resources/test/manifest.yml");
        let manifest = super::read_manifest(&manifest_path).unwrap();
        assert_eq!(
            manifest,
            super::Manifest {
                version : "1".to_string(),
                global : super::Global {
                    game_language: "fr_FR".to_string(),
                    lang_preferences: Some(vec!["french".to_string()]),
                },
                modules : vec![],
            }
        )
    }
}
