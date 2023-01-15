

use std::borrow::Cow;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_yaml::Value;

use crate::components::Components;
use crate::location::Location;
use crate::lowercase::LwcString;
use crate::post_install::{PostInstall, PostInstallExec, PostInstallOutcome};

#[derive(Serialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum Module {
    Mod { weidu_mod: WeiduMod },
    File { file: FileModule },
}

impl Module {
    pub fn get_name(&self) -> &LwcString {
        match self {
            Module::Mod { weidu_mod } => &weidu_mod.name,
            Module::File { file } => &file.file_mod,
        }
    }

    pub fn get_description(&self) -> &Option<String> {
        match self {
            Module::Mod { weidu_mod } => &weidu_mod.description,
            Module::File { file } => &file.description,
        }
    }

    pub fn describe(&self) -> Cow<String> {
        match &self.get_description() {
            None => Cow::Borrowed(self.get_name().as_ref()),
            Some(desc) => Cow::Owned(format!("{} ({})", self.get_name().as_ref(), desc)),
        }
    }

    pub fn exec_post_install(&self, mod_name: &LwcString) -> PostInstallOutcome {
        match self {
            Module::Mod { weidu_mod } => weidu_mod.post_install.exec(mod_name),
            Module::File { file } => file.post_install.exec(mod_name),
        }
    }
}

/** Definition of a mod. */
#[derive(Deserialize, Serialize, Debug, PartialEq, Default)]
pub struct WeiduMod {
    /**
     * Unique identifier of a mod.
     * This is the weidu mod name: name of the tp2 file without `setup-` ot the tp2 extension.
     * This is also the name as used in `weidu.log`.
     * This is case-insensitive.
     */
    pub name: LwcString,
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
    /// - If set to true, warning are ignored and the installation proceed with the following mods
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
    /// Decides what will be done after the mod installation (in case of success).
    /// - `interrupt` will stop the installation and exist the program
    /// - `wait_seconds: xxx will wait xxx second before continuing to the next mod
    /// - `none` (the default) immediately starts the next mod installation.
    #[serde(default)]
    pub post_install: Option<PostInstall>,

    // Below: unused, sort of inert metadata
    pub comment: Option<String>,
    pub original_thread: Option<String>,
    pub original_dl: Option<String>,
    pub installation: Option<InstallationComments>,
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

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
pub struct PrecopyCommand {
    pub command: String,
    pub args: Option<Vec<String>>,
    pub subdir: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Default)]
pub struct InstallationComments {
    pub general: Option<String>,
    pub before: Option<String>,
    pub after: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct FileModule {
    pub file_mod: LwcString,
    pub description: Option<String>,
    pub origin: FileModuleOrigin,
    pub post_install: Option<PostInstall>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum FileModuleOrigin {
    Local { local: String },
}



#[derive(Deserialize, Debug)]
struct ModuleHelper {
    #[serde(flatten)]
    weidu: Option<WeiduMod>,
    #[serde(flatten)]
    file: Option<FileModule>,
    #[serde(flatten)]
    unknown: HashMap<String, Value>,
}
impl <'de> Deserialize<'de> for Module {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where D: serde::Deserializer<'de> {
        let helper = ModuleHelper::deserialize(deserializer)?;
        match helper {
            ModuleHelper { weidu: None, file: None, unknown } => Err(serde::de::Error::custom(
                format!("Incorrect module definition found ; could not recognize weidu mod or file module definition in {:?}", unknown)
            )),
            ModuleHelper { weidu: Some(weidu), file: Some(file), unknown } => Err(serde::de::Error::custom(
                format!("Incorrect module definition found ; could not decide module kind, either {:?} or {:?} with additional data {:?}",
                            weidu, file, unknown)
            )),
            ModuleHelper { file: Some(file), .. } => Ok(Module::File { file }),
            ModuleHelper { weidu: Some(weidu_mod), .. } => Ok(Module::Mod { weidu_mod }),
        }
    }
}

#[cfg(test)]
mod test_deserialize {
    use crate::lowercase::lwc;
    use crate::module::{WeiduMod, ModuleConf, ModuleContent, FileModule, FileModuleOrigin, Module};
    use crate::components::{Components, Component};
    use crate::post_install::PostInstall;
    use crate::location::{Location, Source, GithubDescriptor, Github};
    use crate::patch_source::{PatchEncoding, PatchSource, PatchDesc};
    use crate::archive_layout::Layout;

    #[test]
    fn deserialize_mod_usual() {
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
        let module: WeiduMod = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            module,
            WeiduMod {
                name: lwc!("DlcMerger"),
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
                    ..Location::default()
                }),
                ..WeiduMod::default()
            }
        );
    }

    #[test]
    fn deserialize_multi_mod() {
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
        let module: WeiduMod = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            module,
            WeiduMod {
                name: lwc!("DlcMerger"),
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
                    ..Location::default()
                }),
                ..WeiduMod::default()
            }
        );
    }

    #[test]
    fn deserialize_mod_with_add_conf_and_content() {
        let yaml = r#"
        name: DlcMerger
        add_conf:
            file_name: toto
            content: whatever
        components:
            - 1
        "#;
        let module: WeiduMod = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            module,
            WeiduMod {
                name: lwc!("DlcMerger"),
                components: Components::List(vec! [ Component::Simple(1) ]),
                add_conf: Some(ModuleConf {
                    file_name: "toto".to_string(),
                    content: ModuleContent::Content { content: "whatever".to_string() },
                }),
                ..WeiduMod::default()
            }
        );
    }

    #[test]
    fn deserialize_mod_with_add_conf_and_multiline_content() {
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
        let module: WeiduMod = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            module,
            WeiduMod {
                name: lwc!("DlcMerger"),
                components: Components::List(vec! [ Component::Simple(1) ]),
                add_conf: Some(ModuleConf {
                    file_name: "toto".to_string(),
                    content: ModuleContent::Content { content: "line 1\nline 2\n".to_string() },
                }),
                ..WeiduMod::default()
            }
        );
    }

    #[test]
    fn deserialize_mod_with_add_conf_with_prompt() {
        let yaml = r#"
        name: DlcMerger
        add_conf:
            file_name: toto
            prompt: prompt
        components:
            - 1
        "#;
        let module: WeiduMod = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            module,
            WeiduMod {
                name: lwc!("DlcMerger"),
                components: Components::List(vec! [ Component::Simple(1) ]),
                add_conf: Some(ModuleConf {
                    file_name: "toto".to_string(),
                    content: ModuleContent::Prompt { prompt: "prompt".to_string() },
                }),
                ..WeiduMod::default()
            }
        );
    }

    #[test]
    fn deserialize_mod_with_http_patch() {
        let yaml = r#"
        name: DlcMerger
        location:
            http: https://module.location
            patch:
                http: https://patch.location
        components:
            - 1
        "#;
        let module: WeiduMod = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            module,
            WeiduMod {
                name: lwc!("DlcMerger"),
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
                    ..Location::default()
                }),
                ..WeiduMod::default()
            }
        );
    }

    #[test]
    fn deserialize_mod_with_inline_patch() {
        let yaml = include_str!("../resources/test/read_inline_patch/module_with_inline_patch.yaml");
        let module: WeiduMod = serde_yaml::from_str(yaml).unwrap();
        let expected_content = include_str!("../resources/test/read_inline_patch/inline_patch.diff");
        assert_eq!(
            module,
            WeiduMod {
                name: lwc!("modulename"),
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
                    ..Location::default()
                }),
                ..WeiduMod::default()
            }
        );
    }

    #[test]
    fn serialize_mod_with_continue() {
        let module = WeiduMod {
            name: lwc!("DlcMerger"),
            components: Components::List(vec! [ Component::Simple(1) ]),
            post_install: Some(PostInstall::None),
            ..WeiduMod::default()
        };
        println!("{}", serde_yaml::to_string(&module).unwrap());
    }

    #[test]
    fn deserialize_mod_with_post_install_interrupt() {
        let yaml = r#"
        name: DlcMerger
        components:
            - 1
        post_install: interrupt
        "#;
        let module: WeiduMod = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            module,
            WeiduMod {
                name: lwc!("DlcMerger"),
                components: Components::List(vec! [ Component::Simple(1) ]),
                post_install: Some(PostInstall::Interrupt),
                ..WeiduMod::default()
            }
        );
    }

    #[test]
    fn deserialize_mod_with_post_install_none() {
        let yaml = r#"
        name: DlcMerger
        components:
            - 1
        post_install: none
        "#;
        let module: WeiduMod = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            module,
            WeiduMod {
                name: lwc!("DlcMerger"),
                components: Components::List(vec! [ Component::Simple(1) ]),
                post_install: Some(PostInstall::None),
                ..WeiduMod::default()
            }
        );
    }

    #[test]
    fn serialize_mod_with_post_install_wait() {
        let module = WeiduMod {
            name: lwc!("DlcMerger"),
            components: Components::List(vec! [ Component::Simple(1) ]),
            post_install: Some(PostInstall::WaitSeconds { wait_seconds: 10 }),
            ..WeiduMod::default()
        };
        println!("{}", serde_yaml::to_string(&module).unwrap());
    }

    #[test]
    fn deserialize_mod_with_post_install_wait() {
        let yaml = r#"
        name: DlcMerger
        components:
            - 1
        post_install:
            wait_seconds: 10
        "#;
        let module: WeiduMod = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            module,
            WeiduMod {
                name: lwc!("DlcMerger"),
                components: Components::List(vec! [ Component::Simple(1) ]),
                post_install: Some(PostInstall::WaitSeconds { wait_seconds: 10 }),
                ..WeiduMod::default()
            }
        );
    }

    #[test]
    fn serialize_filemodule() {
        let module = FileModule {
            file_mod: lwc!("DlcMerger"),
            origin: FileModuleOrigin::Local { local: "dir/file.bcs".to_string() },
            description: None,
            post_install: None,
        };
        println!("{}", serde_yaml::to_string(&module).unwrap());
    }

    #[test]
    fn deserialize_file_mod() {
        let yaml = r#"
        file_mod: configure_whatever
        origin:
            local: path/file.idk
        "#;
        let module: FileModule = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            module,
            FileModule {
                file_mod: lwc!("configure_whatever"),
                description: None,
                origin: FileModuleOrigin::Local { local: "path/file.idk".to_string() },
                post_install: None,
            }
        );
    }

    #[test]
    fn deserialize_modules_with_weidu_mod_and_file_mod() {
        let yaml = r#"
            - name: DlcMerger
              components: ask
            - file_mod: configure_whatever
              origin:
                local: path/file.idk
        "#;
        let modules: Vec<Module> = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            modules,
            vec![
                Module::Mod { weidu_mod: WeiduMod {
                    name: lwc!("DlcMerger"),
                    components: Components::Ask,
                    ..Default::default()
                }},
                Module::File { file: FileModule {
                    file_mod: lwc!("configure_whatever"),
                    description: None,
                    origin: FileModuleOrigin::Local { local: "path/file.idk".to_string() },
                    post_install: None,
                }},
            ],
        );
    }

    #[test]
    fn deserialize_mixed_module() {
        let yaml = r#"
            name: DlcMerger
            components: ask
            file_mod: some_name
            origin:
              local: path/file.idk
        "#;
        let error: Result<Module, serde_yaml::Error> = serde_yaml::from_str(yaml);
        let err = error.unwrap_err();
        println!("deserialize_mixed_module error is {:?}", err)
    }
}
