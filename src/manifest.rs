
use std::borrow::Cow;
use std::io::{BufReader, Seek, SeekFrom};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

use anyhow::{bail, Result};

use crate::archive_layout::Layout;
use crate::patch_source::PatchSource;

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
    #[serde(flatten)]
    pub content: ModuleContent,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum ModuleContent {
    /// The actual content of thefile is provided
    Content { content: String },
    /// Interrupt and ask the user to input the content (value of `prompt` is shown)
    Prompt { prompt: String },
}
#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct Location {
    #[serde(flatten)]
    pub source: Source,
    /// Specifies which files from the archive will be copied to the game directory.
    /// Read as a Unix shell style glob pattern (https://docs.rs/glob/0.3.0/glob/struct.Pattern.html)
    #[serde(default)]
    pub layout: Layout,
    #[serde(default)]
    pub patch: Option<PatchSource>,
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
                GithubDescriptor::Release { asset , ..} =>
                                                    Ok(PathBuf::from(asset.to_owned())),
                GithubDescriptor::Commit { commit } =>
                                                    Ok(PathBuf::from(format!("{}-{}.zip",module_name, commit))),
                GithubDescriptor::Branch { branch } =>
                                                    Ok(PathBuf::from(format!("{}-{}.zip",module_name, branch))),
                GithubDescriptor::Tag { tag } => Ok(PathBuf::from(format!("{}-{}.zip",module_name, tag))),
            }
        }
    }

    pub fn default_strip_leading(&self) -> usize {
        use GithubDescriptor::*;
        match self {
            Source::Github(Github { descriptor: Commit{..}, .. })
            | Source::Github(Github { descriptor: Tag{..}, .. })
            | Source::Github(Github { descriptor: Branch{..}, .. }) => 1,
            _ => 0,
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
    Release { release: Option<String>, asset: String },
    Commit { commit: String },
    Branch { branch: String },
    Tag { tag: String },
}

impl GithubDescriptor {
    pub fn get_url(&self, user: &str, repository: &str) -> String {
        use GithubDescriptor::*;
        match self {
            Release { release, asset } => {
                let release = match &release {
                    None => String::from("latest"),
                    Some(release) => release.to_owned(),
                };
                format!("https://github.com/{user}/{repo}/releases/download/{release}/{asset}",
                    user = user,
                    repo = repository,
                    release = release,
                    asset = asset.replace("{{release}}", &release),
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
                format!("https://github.com/{user}/{repo}/archive/{commit}.zip",
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
impl Source {
    pub fn http_source() -> Source {
        Source::Http { http: "https://dummy.example".to_string(), rename: None }
    }
    pub fn gh_release_source() -> Source {
        Source::Github(
            Github {
                github_user: "".to_string(),
                repository: "".to_string(),
                descriptor: GithubDescriptor::Release {
                    release: Some("".to_string()),
                    asset: "".to_string(),
                },
            }
        )
    }
    pub fn gh_branch_source() -> Source {
        Source::Github(
            Github {
                github_user: "".to_string(),
                repository: "".to_string(),
                descriptor: GithubDescriptor::Branch {
                    branch: "".to_string(),
                },
            }
        )
    }
}

#[cfg(test)]
mod test_deserialize {

    use indoc::indoc;

    use crate::patch_source::PatchSource;
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
        asset: my_repo-1.0.zip
        "#;
        let source: Source = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            source,
            Source::Github(Github {
                github_user: "my_user".to_string(),
                repository: "my_repo".to_string(),
                descriptor: GithubDescriptor::Release {
                    release: Some("1.0".to_string()),
                    asset: "my_repo-1.0.zip".to_string(),
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
    fn deserialize_module_usual() {
        use crate::manifest::{Module, Location, Source, Layout, Github, Component};
        let yaml = r#"
        name: DlcMerger
        location:
            github_user: Argent77
            repository: A7-DlcMerger
            release: v1.3
            asset: lin-A7-DlcMerger-v1.3.zip
            layout:
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
                            asset: "lin-A7-DlcMerger-v1.3.zip".to_string(),
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
    fn deserialize_multi_module() {
        use crate::manifest::{Module, Location, Source, Layout, Github, Component};
        let yaml = r#"
        name: DlcMerger
        location:
            github_user: Argent77
            repository: A7-DlcMerger
            release: v1.3
            asset: lin-A7-DlcMerger-v1.3.zip
            strip_leading: 37
            layout:
                layout_type: multi_dir
                dirs: ["a", "b"]
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
                            asset: "lin-A7-DlcMerger-v1.3.zip".to_string(),
                        },
                    }),
                    layout: Layout::multi_dir(vec!["a".to_string(),"b".to_string()]),
                    patch: None,
                }),
                ..Module::default()
            }
        );
    }

    #[test]
    fn deserialize_module_with_add_conf_and_content() {

        use crate::manifest::{Module, Component, ModuleConf, ModuleContent};
        let yaml = r#"
        name: DlcMerger
        add_conf:
            file_name: toto
            content: whatever
        components:
            - 1
        "#;
        let module: Module = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            module,
            Module {
                name: "DlcMerger".to_string(),
                components: Some(vec! [ Component::Simple(1) ]),
                add_conf: Some(ModuleConf {
                    file_name: "toto".to_string(),
                    content: ModuleContent::Content { content: "whatever".to_string() },
                }),
                ..Module::default()
            }
        );
    }

    #[test]
    fn deserialize_module_with_add_conf_and_multiline_content() {

        use crate::manifest::{Module, Component, ModuleConf, ModuleContent};
        let yaml = r#"
        name: DlcMerger
        add_conf:
            file_name: toto
            content: |
                line 1
                line 2
        components:
            - 1
        "#;
        let module: Module = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            module,
            Module {
                name: "DlcMerger".to_string(),
                components: Some(vec! [ Component::Simple(1) ]),
                add_conf: Some(ModuleConf {
                    file_name: "toto".to_string(),
                    content: ModuleContent::Content { content: "line 1\nline 2\n".to_string() },
                }),
                ..Module::default()
            }
        );
    }

    #[test]
    fn deserialize_module_with_add_conf_with_prompt() {

        use crate::manifest::{Module, Component, ModuleConf, ModuleContent};
        let yaml = r#"
        name: DlcMerger
        add_conf:
            file_name: toto
            prompt: prompt
        components:
            - 1
        "#;
        let module: Module = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            module,
            Module {
                name: "DlcMerger".to_string(),
                components: Some(vec! [ Component::Simple(1) ]),
                add_conf: Some(ModuleConf {
                    file_name: "toto".to_string(),
                    content: ModuleContent::Prompt { prompt: "prompt".to_string() },
                }),
                ..Module::default()
            }
        );
    }

    #[test]
    fn deserialize_module_with_http_patch() {
        use crate::manifest::{ Module, Component, Location, Layout };

        let yaml = r#"
        name: DlcMerger
        location:
            http: https://module.location
            patch:
                http: https://patch.location
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
                    source: Source::Http {
                        http: "https://module.location".to_owned(),
                        rename: None,
                    },
                    layout: Layout::default(),
                    patch: Some(PatchSource::Http {
                        http: "https://patch.location".to_owned(),
                    }),
                }),
                ..Module::default()
            }
        );
    }

    #[test]
    fn deserialize_module_with_inline_patch() {
        use crate::manifest::{ Module, Component, Location, Layout };

        let yaml = indoc!(r#"
            name: modulename
            location:
                http: https://module.location
                patch:
                    inline: |
                        diff --git a/resources/test/patch/modulename.tp2 b/resources/test/patch/modulename.tp2
                        index a27f249..12c4323 100644
                        --- a/resources/test/patch/modulename.tp2
                        +++ b/resources/test/patch/modulename.tp2
                        @@ -1,6 +1,6 @@
                        BACKUP ~weidu_external/backup/modulename~
                        SUPPORT ~http://somewhere.iflucky.org~
                        -VERSION ~1.0~
                        +VERSION ~2.0~

                        LANGUAGE ~English~
                                ~english~
            components:
                - 1
        "#);
        let module: Module = serde_yaml::from_str(yaml).unwrap();
        let expected_content = indoc!(r#"
            diff --git a/resources/test/patch/modulename.tp2 b/resources/test/patch/modulename.tp2
            index a27f249..12c4323 100644
            --- a/resources/test/patch/modulename.tp2
            +++ b/resources/test/patch/modulename.tp2
            @@ -1,6 +1,6 @@
            BACKUP ~weidu_external/backup/modulename~
            SUPPORT ~http://somewhere.iflucky.org~
            -VERSION ~1.0~
            +VERSION ~2.0~

            LANGUAGE ~English~
                    ~english~
        "#).to_owned();
        assert_eq!(
            module,
            Module {
                name: "modulename".to_string(),
                components: Some(vec! [ Component::Simple(1) ]),
                location: Some(Location {
                    source: Source::Http {
                        http: "https://module.location".to_owned(),
                        rename: None,
                    },
                    layout: Layout::default(),
                    patch: Some(PatchSource::Inline {
                        inline: expected_content,
                    }),
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
