
use std::borrow::Cow;
use std::io::{BufReader, Seek, SeekFrom};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

use anyhow::{bail, Result};

use crate::archive_layout::Layout;
use crate::download::Downloader;
use crate::patch_source::PatchDesc;
use crate::components::{Component, Components};

#[derive(Deserialize, Serialize, Debug)]
pub struct VersionDetect {
    pub version: String,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct Manifest {
    /** Manifest format version */
    pub version: String,
    /** Manifest-wide definitions. */
    pub global: Global,
    #[serde(default)]
    /** List of mods */
    pub modules: Vec<Module>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Default)]
pub struct Global {
    /// The "language code" configured in the game e.g. en_US, es_ES, fr_FR
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
    #[serde(default)]
    pub patch_path: Option<String>,
    /// Path from manifest root (yml file location directory) where "local" mods can be found.
    #[serde(default)]
    pub local_mods: Option<String>,
}

/** Definition of a mod. */
#[derive(Deserialize, Serialize, Debug, PartialEq, Default)]
pub struct Module {
    /**
     * Unique identifier of a mod.
     * This is the weidu mod name: name of the tp2 file without `setup-` ot the tp2 extension.
     * This is also the name as used in `weidu.log`.
     * This is case-insensitive.
     */
    pub name: String,
    /// Unused at the moment
    pub version: Option<String>,
    /// Optional description, used to disambiguate multiple occurrences of the same mod
    pub description: Option<String>,
    /// Which language index to use (has precedence over manifest-level lang_prefs)
    pub language: Option<u32>,
    /// List of components to be auto-installed.
    /// Can be `ask`, `none`, a list of components or absent/not set/null (which is the same as `ask`)
    ///   - `ask` (or empty) will use weidu in interactive mode (weidu itself asks how to install components)
    ///   - `none` will just copy the mod filesin the game dir without installing anything
    ///   - a list of components will call weidu and provide the list of components on the command line
    #[serde(deserialize_with = "crate::components::component_deser")]
    pub components: Components,
    /// Whether warnings returned by weidu (exit code) will interrupt the whole installation.
    ///
    /// (defaults to _not_ ignoring warnings)..
    /// - If set to true, warning are ignored and the installation proceed with the followinf mods
    /// - If set to false (or absent), weidu warnings will stop the installation.
    #[serde(default)]
    pub ignore_warnings: bool,
    pub add_conf: Option<ModuleConf>,
    /// Where we can obtain the module.
    ///
    /// If absent, it is assumed to be in the game install.
    /// In that case, it checks a `<mod_name.tp2>`,`setup-mod_name>.tp2` in the game dir and in
    /// `<nod_name>` sub-directory. If it is not found, the installation aborts.
    pub location: Option<Location>,
    #[serde(default)]
    pub interrupt: bool,
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
            Components::None => vec![],
            Components::Ask => vec![],
            Components::List(components) => components.iter().filter(|comp|
                match comp {
                    Component::Simple(_) => false,
                    Component::Full{ ignore_warn, ..} =>  *ignore_warn,
                }).collect::<Vec<_>>(),
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
#[derive(Deserialize, Serialize, Debug, PartialEq, Default, Clone)]
pub struct Location {
    #[serde(flatten)]
    pub source: Source,
    /// Specifies which files from the archive will be copied to the game directory.
    /// Read as a Unix shell style glob pattern (https://docs.rs/glob/0.3.0/glob/struct.Pattern.html)
    #[serde(default)]
    pub layout: Layout,
    #[serde(default)]
    pub patch: Option<PatchDesc>,
    #[serde(default)]
    pub precopy: Option<PrecopyCommand>,
}


#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
pub enum Source {
    Http { http: String, rename: Option<String> },
    Github(Github),
    Absolute { path: String },
    Local { local: String },
}

impl Default for Source {
    fn default() -> Self {
        Source::Local { local: String::new() }
    }
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
            Absolute { .. } | Local { .. }=> Ok(PathBuf::new()),
            Github(self::Github { github_user, repository, .. }) =>
                Ok(PathBuf::from("github").join(github_user).join(repository)),
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
            Absolute { .. } | Local { .. } => Ok(PathBuf::new()),
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

#[derive(Deserialize, Serialize, Debug, PartialEq, Default, Clone)]
pub struct Github {
    pub github_user: String,
    pub repository: String,
    #[serde(flatten)]
    pub descriptor: GithubDescriptor
}
impl Github {
    pub async fn get_github(&self, downloader: &Downloader, dest: &PathBuf, save_name: PathBuf) -> Result<PathBuf> {
        downloader.download(
            &self.descriptor.get_url(&self.github_user, &self.repository),
            dest,
            save_name,
        ).await
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
pub enum GithubDescriptor {
    Release { release: Option<String>, asset: String },
    Commit { commit: String },
    Branch { branch: String },
    Tag { tag: String },
}

impl Default for GithubDescriptor {
    fn default() -> Self {
        GithubDescriptor::Tag { tag: "".to_string() }
    }
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

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
pub struct PrecopyCommand {
    pub command: String,
    pub args: Option<Vec<String>>,
    pub subdir: Option<String>,
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

    use crate::patch_source::{PatchDesc, PatchEncoding, PatchSource};
    use crate::components::{ Components };
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
                components: Components::List(vec! [ Component::Simple(1) ]),
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
                    precopy: None,
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
                components: Components::List(vec! [ Component::Simple(1) ]),
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
                    precopy: None,
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
                components: Components::List(vec! [ Component::Simple(1) ]),
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
                components: Components::List(vec! [ Component::Simple(1) ]),
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
                components: Components::List(vec! [ Component::Simple(1) ]),
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
                components: Components::List(vec! [ Component::Simple(1) ]),
                location: Some(Location {
                    source: Source::Http {
                        http: "https://module.location".to_owned(),
                        rename: None,
                    },
                    layout: Layout::default(),
                    patch: Some(PatchDesc {
                        patch_source: PatchSource::Http {
                            http: "https://patch.location".to_owned(),
                        },
                        encoding: PatchEncoding::UTF8,
                    }),
                    precopy: None,
                }),
                ..Module::default()
            }
        );
    }

    #[test]
    fn deserialize_module_with_inline_patch() {
        use crate::manifest::{ Module, Component, Location, Layout };

        let yaml = include_str!("../resources/test/read_inline_patch/module_with_inline_patch.yaml");
        let module: Module = serde_yaml::from_str(yaml).unwrap();
        let expected_content = include_str!("../resources/test/read_inline_patch/inline_patch.diff");
        assert_eq!(
            module,
            Module {
                name: "modulename".to_string(),
                components: Components::List(vec! [ Component::Simple(1) ]),
                location: Some(Location {
                    source: Source::Http {
                        http: "https://module.location".to_owned(),
                        rename: None,
                    },
                    layout: Layout::default(),
                    patch: Some(PatchDesc {
                        patch_source: PatchSource::Inline {
                            inline: expected_content.to_owned(),
                        },
                        encoding: PatchEncoding::UTF8,
                    }),
                    precopy: None,
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
                    patch_path: None,
                    local_mods: None,
                },
                modules : vec![],
            }
        )
    }
}
