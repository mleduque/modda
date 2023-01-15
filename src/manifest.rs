
use std::io::{BufReader, Seek, SeekFrom};
use serde::{Deserialize, Serialize};

use anyhow::{bail, Result};

use crate::global::Global;
use crate::module::Module;


#[derive(Deserialize, Serialize, Debug)]
pub struct VersionDetect {
    pub version: String,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct Manifest {
    /// Manifest format version
    pub version: String,
    /// Manifest-wide definitions
    pub global: Global,
    #[serde(default)]
    /// List of modules
    pub modules: Vec<Module>,
}

impl Manifest {
    pub fn read_path(path: &str) -> Result<Self> {
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
}

#[cfg(test)]
mod test_deserialize {

    use crate::components::{Component, Components};
    use crate::location::Location;
    use crate::lowercase::lwc;
    use crate::module::{WeiduMod, Module, FileModule, FileModuleOrigin};

    use super::Manifest;

    #[test]
    fn check_read_manifest() {
        let manifest_path = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), "resources/test/manifest.yml");
        let manifest = Manifest::read_path(&manifest_path).unwrap();
        assert_eq!(
            manifest,
            super::Manifest {
                version : "1".to_string(),
                global : super::Global {
                    game_language: "fr_FR".to_string(),
                    lang_preferences: Some(vec!["french".to_string()]),
                    patch_path: None,
                    local_mods: None,
                    local_files: None,
                },
                modules : vec![],
            }
        )
    }

    #[test]
    fn check_read_manifest_with_module() {
        let manifest_path = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), "resources/test/manifest_with_modules.yml");
        let manifest = Manifest::read_path(&manifest_path).unwrap();
        assert_eq!(
            manifest,
            super::Manifest {
                version : "1".to_string(),
                global : super::Global {
                    game_language: "fr_FR".to_string(),
                    lang_preferences: Some(vec!["french".to_string()]),
                    patch_path: None,
                    local_mods: Some("mods".to_string()),
                    local_files: None,
                },
                modules : vec![
                    Module::Mod {
                        weidu_mod: WeiduMod {
                            name: lwc!("aaa"),
                            components: Components::List(vec! [ Component::Simple(1) ]),
                            location: Some(Location {
                                source: crate::location::Source::Http { http: "http://example.com/my-mod".to_string(), rename: None },
                                ..Default::default()
                            }),
                            ..Default::default()
                        },
                    },
                    Module::File {
                        file: FileModule {
                            file_mod: lwc!("bbb"),
                            from: FileModuleOrigin::Local { local:"files/my-file.itm".to_string() },
                            to: "override".to_string(),
                            description: None,
                            post_install: None,
                        }
                    }
                ],
            }
        )
    }
}
