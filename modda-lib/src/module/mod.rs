
pub mod components;
pub mod disable_condition;
pub mod file_mod;
pub mod file_module_origin;
pub mod gen_mod;
pub mod global_locations;
pub mod install_comment;
pub mod language;
pub mod location;
pub mod manifest;
pub mod manifest_conditions;
pub mod module;
pub mod module_conf;
pub mod pre_copy_command;
pub mod refresh;
pub mod weidu_mod;


#[cfg(test)]
mod test_deserialize {
    use serde_yaml::Deserializer;

    use crate::lowercase::lwc;
    use crate::module::components::{Components, Component};
    use crate::module::disable_condition::DisableCondition;
    use crate::module::file_mod::FileModule;
    use crate::module::file_module_origin::FileModuleOrigin;
    use crate::module::gen_mod::{GeneratedMod, GenModComponent};
    use crate::module::location::github::{Github, GithubDescriptor};
    use crate::module::location::http::Http;
    use crate::module::location::location::{ConcreteLocation, Location};
    use crate::module::location::source::Source;
    use crate::module::module_conf::{ModuleConf, ModuleContent};
    use crate::module::weidu_mod::WeiduMod;
    use crate::post_install::PostInstall;
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
                location: Some(Location::Concrete {
                    concrete: ConcreteLocation {
                    source: Source::Github(Github {
                        github_user: "Argent77".to_string(),
                        repository: "A7-DlcMerger".to_string(),
                        descriptor: GithubDescriptor::Release {
                            release: Some("v1.3".to_string()),
                            asset: "lin-A7-DlcMerger-v1.3.zip".to_string(),
                        },
                        ..Default::default()
                    }),
                    layout: Layout::single_dir(3),
                    ..ConcreteLocation::default()
                }
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
                location: Some(Location::Concrete {
                    concrete: ConcreteLocation {
                        source: Source::Github(Github {
                            github_user: "Argent77".to_string(),
                            repository: "A7-DlcMerger".to_string(),
                            descriptor: GithubDescriptor::Release {
                                release: Some("v1.3".to_string()),
                                asset: "lin-A7-DlcMerger-v1.3.zip".to_string(),
                            },
                            ..Default::default()
                        }),
                        layout: Layout::multi_dir(vec!["a".to_string(),"b".to_string()]),
                        ..ConcreteLocation::default()
                    }}),
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
                location: Some(Location::Concrete {
                    concrete: ConcreteLocation {
                        source: Source::Http(Http {
                            http: "https://module.location".to_owned(),
                            rename: None,
                            ..Default::default()
                        }),
                        layout: Layout::default(),
                        patch: Some(PatchDesc {
                            patch_source: PatchSource::Http {
                                http: "https://patch.location".to_owned(),
                            },
                            encoding: PatchEncoding::UTF8,
                        }),
                        ..ConcreteLocation::default()
                    }
                }),
                ..WeiduMod::default()
            }
        );
    }

    #[test]
    fn deserialize_mod_with_relative_file_patch() {
        let yaml = r#"
        name: DlcMerger
        location:
            http: https://module.location
            patch:
                relative: patches/my_patch.diff
        components:
            - 1
        "#;
        let module: WeiduMod = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            module,
            WeiduMod {
                name: lwc!("DlcMerger"),
                components: Components::List(vec! [ Component::Simple(1) ]),
                location: Some(Location::Concrete {
                    concrete: ConcreteLocation {
                        source: Source::Http(Http {
                            http: "https://module.location".to_owned(),
                            rename: None,
                            ..Default::default()
                        }),
                        layout: Layout::default(),
                        patch: Some(PatchDesc {
                            patch_source: PatchSource::Relative {
                                relative: "patches/my_patch.diff".to_owned(),
                            },
                            encoding: PatchEncoding::UTF8,
                        }),
                        ..ConcreteLocation::default()
                    }
                }),
                ..WeiduMod::default()
            }
        );
    }

    #[test]
    fn deserialize_mod_with_relative_file_patch_list() {
        let yaml = r#"
        name: DlcMerger
        location:
            http: https://module.location
            patches:
                - relative: patches/my_patch1.diff
                - relative: patches/my_patch2.diff
        components:
            - 1
        "#;
        let module: WeiduMod = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            module,
            WeiduMod {
                name: lwc!("DlcMerger"),
                components: Components::List(vec! [ Component::Simple(1) ]),
                location: Some(Location::Concrete {
                    concrete: ConcreteLocation {
                        source: Source::Http(Http {
                            http: "https://module.location".to_owned(),
                            rename: None,
                            ..Default::default()
                        }),
                        layout: Layout::default(),
                        patches: Some(vec![
                            PatchDesc {
                                patch_source: PatchSource::Relative {
                                    relative: "patches/my_patch1.diff".to_owned(),
                                },
                                encoding: PatchEncoding::UTF8,
                            },
                            PatchDesc {
                                patch_source: PatchSource::Relative {
                                    relative: "patches/my_patch2.diff".to_owned(),
                                },
                                encoding: PatchEncoding::UTF8,
                            },
                        ]),
                        ..ConcreteLocation::default()
                    }
                }),
                ..WeiduMod::default()
            }
        );
    }

    #[test]
    fn deserialize_mod_with_inline_patch() {
        let yaml = include_str!("../../resources/test/read_inline_patch/module_with_inline_patch.yaml");
        let module: WeiduMod = serde_yaml::from_str(yaml).unwrap();
        let expected_content = include_str!("../../resources/test/read_inline_patch/inline_patch.diff");
        assert_eq!(
            module,
            WeiduMod {
                name: lwc!("modulename"),
                components: Components::List(vec! [ Component::Simple(1) ]),
                location: Some(Location::Concrete {
                    concrete: ConcreteLocation {
                        source: Source::Http(Http {
                            http: "https://module.location".to_owned(),
                            rename: None,
                            ..Default::default()
                        }),
                        layout: Layout::default(),
                        patch: Some(PatchDesc {
                            patch_source: PatchSource::Inline {
                                inline: expected_content.to_owned(),
                            },
                            encoding: PatchEncoding::UTF8,
                        }),
                        ..ConcreteLocation::default()
                    }
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
            from: FileModuleOrigin::Local { local: "dir/file.bcs".to_string(), glob: None },
            to: "override/".to_string(),
            description: None,
            post_install: None,
            allow_overwrite: false,
        };
        println!("{}", serde_yaml::to_string(&module).unwrap());
    }

    #[test]
    fn deserialize_file_mod() {
        let yaml = r#"
        file_mod: configure_whatever
        from:
            local: path/file.idk
        to: override/
        "#;
        let module: FileModule = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            module,
            FileModule {
                file_mod: lwc!("configure_whatever"),
                description: None,
                from: FileModuleOrigin::Local { local: "path/file.idk".to_string(), glob: None },
                to: "override/".to_string(),
                post_install: None,
                allow_overwrite: false,
            }
        );
    }

    #[test]
    fn deserialize_gen_mod() {
        let yaml = r#"
        gen_mod: some_name
        description: some description
        post_install: interrupt
        files:
            - local: some_dir
            - local: other_dir
              glob: "*.itm" # must quote because * is a special char
            - absolute: "/location"
        allow_overwrite: true
        ignore_warnings: true
        "#;
        let deserializer = Deserializer::from_str(yaml);
        let module: GeneratedMod = serde_path_to_error::deserialize(deserializer).unwrap();
        assert_eq!(
            module,
            GeneratedMod {
                gen_mod: lwc!("some_name"),
                description: Some("some description".to_string()),
                post_install: Some(PostInstall::Interrupt),
                files: vec![
                    FileModuleOrigin::Local { local: "some_dir".to_string(), glob: None },
                    FileModuleOrigin::Local { local: "other_dir".to_string(), glob: Some("*.itm".to_string()) },
                    FileModuleOrigin::Absolute { absolute: "/location".to_string(), glob: None },
                ],
                component: GenModComponent { index: 0, name: None },
                allow_overwrite: true,
                ignore_warnings: true,
                disabled_if: None,
            }
        );
    }

    #[test]
    fn deserialize_mod_with_unconditional_disabling() {
        let yaml = r#"
        name: DlcMerger
        components:
            - 1
        disabled_if:
            because: This is bg2 not bg1
        "#;
        let module: WeiduMod = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            module,
            WeiduMod {
                name: lwc!("DlcMerger"),
                components: Components::List(vec! [ Component::Simple(1) ]),
                disabled_if: Some(DisableCondition::Because { because: "This is bg2 not bg1".to_string() }),
                ..WeiduMod::default()
            }
        );
    }

    #[test]
    fn deserialize_mod_with_env_var_disabling() {
        let yaml = r#"
        name: DlcMerger
        components:
            - 1
        disabled_if:
            env_is_set: DISABLE_DLCMERGER
        "#;
        let module: WeiduMod = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            module,
            WeiduMod {
                name: lwc!("DlcMerger"),
                components: Components::List(vec! [ Component::Simple(1) ]),
                disabled_if: Some(DisableCondition::EnvVar { env_is_set: "DISABLE_DLCMERGER".to_string() }),
                ..WeiduMod::default()
            }
        );
    }

    #[test]
    fn deserialize_mod_with_external_file() {
        let yaml = r#"
        name: DlcMerger
        components:
            - 1
        disabled_if:
            in_file: disabling.file
            key: dlcmerger
        "#;
        let module: WeiduMod = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            module,
            WeiduMod {
                name: lwc!("DlcMerger"),
                components: Components::List(vec! [ Component::Simple(1) ]),
                disabled_if: Some(DisableCondition::File { in_file: "disabling.file".to_string(), key: "dlcmerger".to_string() }),
                ..WeiduMod::default()
            }
        );
    }

    #[test]
    fn deserialize_mod_with_negation() {
        let yaml = r#"
        name: DlcMerger
        components:
            - 1
        disabled_if:
            not:
                env_is_set: DONT_DISABLE_DLCMERGER
        "#;
        let module: WeiduMod = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            module,
            WeiduMod {
                name: lwc!("DlcMerger"),
                components: Components::List(vec! [ Component::Simple(1) ]),
                disabled_if: Some(DisableCondition::Not {
                    not: Box::new(DisableCondition::EnvVar { env_is_set: "DONT_DISABLE_DLCMERGER".to_string() }),
                }),
                ..WeiduMod::default()
            }
        );
    }

    #[test]
    fn deserialize_mod_with_disabling_condition_with_any() {
        let yaml = r#"
        name: DlcMerger
        components:
            - 1
        disabled_if:
            any:
                - env_is_set: DISABLE_DLCMERGER
                - in_file: disabling.file
                  key: dlcmerger
        "#;
        let module: WeiduMod = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            module,
            WeiduMod {
                name: lwc!("DlcMerger"),
                components: Components::List(vec! [ Component::Simple(1) ]),
                disabled_if: Some(DisableCondition::Any {
                    any: vec![
                        DisableCondition::EnvVar { env_is_set: "DISABLE_DLCMERGER".to_string() },
                        DisableCondition::File { in_file: "disabling.file".to_string(), key: "dlcmerger".to_string() },
                    ],
                }),
                ..WeiduMod::default()
            }
        );
    }

    #[test]
    fn deserialize_mod_with_disabling_condition_with_all() {
        let yaml = r#"
        name: DlcMerger
        components:
            - 1
        disabled_if:
            all:
                - env_is_set: DISABLE_DLCMERGER
                - in_file: disabling.file
                  key: dlcmerger
        "#;
        let module: WeiduMod = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            module,
            WeiduMod {
                name: lwc!("DlcMerger"),
                components: Components::List(vec! [ Component::Simple(1) ]),
                disabled_if: Some(DisableCondition::All {
                    all: vec![
                        DisableCondition::EnvVar { env_is_set: "DISABLE_DLCMERGER".to_string() },
                        DisableCondition::File { in_file: "disabling.file".to_string(), key: "dlcmerger".to_string() },
                    ],
                }),
                ..WeiduMod::default()
            }
        );
    }
}
