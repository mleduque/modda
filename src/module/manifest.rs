
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Seek, SeekFrom, BufWriter, Write};
use std::path::{PathBuf, Path};

use lazy_static::lazy_static;
use log::{info, debug};
use regex::Regex;
use serde::{Deserialize, Serialize};

use anyhow::{bail, Result, Ok, anyhow};
use serde_yaml::Deserializer;
use shellexpand::full;

use crate::global::Global;
use crate::lowercase::LwcString;
use crate::module::module::Module;
use crate::pathext;

use super::global_locations::{GlobalLocations, LocationRegistry};
use super::location::ConcreteLocation;


#[derive(Deserialize, Serialize, Debug)]
pub struct VersionDetect {
    pub version: String,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
pub struct Manifest {
    /// Manifest format version
    pub version: String,
    /// Manifest-wide definitions
    pub global: Global,
    /// List of global locations
    #[serde(default)]
    #[serde(skip_serializing_if = "GlobalLocations::is_empty")]
    pub locations: GlobalLocations,
    /// List of modules
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub modules: Vec<Module>,
}

impl Manifest {
    pub fn assemble_from_path(path: &str, manifest_root: &PathBuf) -> Result<Self> {
        let mut base = Self::read_path(path)?;
        if !base.locations.external.is_empty() {
            let init: HashMap<LwcString, ConcreteLocation> = HashMap::new();
            let locations = base.locations.external.iter()
                .try_fold(init,|mut acc, curr| {
                    let locations = read_external_registry(curr, manifest_root)?;
                    for location in locations {
                        match acc.entry(location.0) {
                            Entry::Occupied(entry) =>
                                info!("Ignoring location {} from external registry {:?} because the key already exists", entry.key(), curr),
                            Entry::Vacant(entry) => { entry.insert(location.1); }
                        }
                    }
                    Ok(acc)
                })?;
            base.merge_location_registry(locations);
        }
        Ok(base)
    }

    pub fn read_path_convert_comments(path: &Path) -> Result<Self> {
        let file = std::fs::File::open(path)
                .map_err(|error| anyhow!("Could not open manifest file {:?} - {:?}", path, error))?;
        Self::read_file(file, true)
    }

    pub fn read_path(path: &str) -> Result<Self> {
        let file = std::fs::File::open(path)
                .map_err(|error| anyhow!("Could not open manifest file {} - {:?}", path, error))?;
        Self::read_file(file, false)
    }

    pub fn read_file(mut file: File, convert_comments: bool) -> Result<Self> {
        {
            let reader = BufReader::new(&file);
            let version: VersionDetect = serde_yaml::from_reader(reader)?;
            if version.version != "1" {
                bail!("Only manifest version 1 is supported for now.");
            }
        }
        let _ = file.seek(SeekFrom::Start(0))?;
        let reader = BufReader::new(file);

        let content = if convert_comments {
            let content = std::io::read_to_string(reader)?;
            reformat_to_property_comments(&content)?
        } else{
            std::io::read_to_string(reader)?
        };
        debug!("reformatted content:\n{}", content);
        let deserializer = Deserializer::from_str(&content);
        let result: Result<Manifest, _> = serde_path_to_error::deserialize(deserializer);
        let manifest: Manifest = result
                    .map_err(|error| anyhow!("Failed to parse manifest\n -> {}\npath:{}", error, error.path()))?;
        Ok(manifest)
    }

    pub fn merge_location_registry(&mut self, locations: HashMap<LwcString, ConcreteLocation>) {
        for location in locations {
            match self.locations.entries.entry(location.0) {
                Entry::Occupied(entry) =>
                    info!("Ignoring location {} from external registries because the key already exists in `entries`", entry.key()),
                Entry::Vacant(entry) => { entry.insert(location.1); }
            }
        }
    }

    pub fn write(&self, path: &Path, reformat_comments: bool) -> Result<()> {
        let temp_path = pathext::append_extension("new", path);
        let dest = match OpenOptions::new().create(true).truncate(true).write(true).open(&temp_path) {
            Err(err) => bail!("Could not create temp output file\n  {}", err),
            std::io::Result::Ok(file) => file,
        };
        let mut buf_writer = BufWriter::new(&dest);
        if reformat_comments {
            let input = serde_yaml::to_string(self)?;
            let reformatted = reformat_to_yaml_comments(&input)?;
            write!(buf_writer, "{}", reformatted)?;
        } else {
            serde_yaml::to_writer(buf_writer, self)?;
        }

        // rename temp file to output file
        if let Err(error) = std::fs::rename(&temp_path, path) {
            bail!("Failed to rename temp output file {:?} to {:?}\n -> {:?}", temp_path, path, error);
        } else {
            debug!("renamed temp output file to {:?}", path);
        }
        Ok(())
    }
}

lazy_static! {
    static ref COMPONENT_REGEX: Regex = Regex::new(concat!(
        r#"(?m)^"#,
        r#"(?P<indent>\s*)-\s*(?P<index>\d+)\s*#\s*(?P<yaml_cmt>.*)"#,
        r#"$"#,
    )).unwrap();
}

pub fn reformat_to_property_comments(input: &str) -> Result<String> {
    debug!("reformat_to_property_comments - input=\n{}", input);
    try_replace_component_comment(&*COMPONENT_REGEX, input)
}

fn try_replace_component_comment<'a>(re: &Regex, haystack: &'a str) -> Result<String> {
    let mut new = String::with_capacity(haystack.len());
    let mut last_match = 0;
    for caps in re.captures_iter(haystack) {
        let everything = &caps.get(0).unwrap(); // apparently we can always unwrap the 0 match
        let indent = &caps["indent"];
        let index = &caps["index"];
        let yaml_cmt = &caps["yaml_cmt"];
        let yaml_cmt = serde_yaml::to_string(&yaml_cmt)?;

        new.push_str(&haystack[last_match..everything.start()]);
        new.push_str(indent);
        new.push_str("- index: ");
        new.push_str(index);
        new.push_str("\n");
        new.push_str(indent);
        new.push_str("  component_name: ");
        new.push_str(&yaml_cmt);
        last_match = everything.end();
    }
    new.push_str(&haystack[last_match..]);
    debug!("try_replace_component_comment - result{}", new);
    Ok(new)
}

fn read_external_registry(registry: &LocationRegistry, manifest_root: &PathBuf) -> Result<HashMap<LwcString, ConcreteLocation>> {
    let path = match registry {
        LocationRegistry::Absolute { path } => PathBuf::from(path),
        LocationRegistry::Local { local } => manifest_root.join(local),
    };
    let file = std::fs::File::open(&path)
            .map_err(|error| anyhow!("Could not open location registry file {:?} - {:?}", path, error))?;
    let reader = BufReader::new(file);
    let deserializer = Deserializer::from_reader(reader);
    let result: Result<HashMap<LwcString, ConcreteLocation>, _> = serde_path_to_error::deserialize(deserializer);

    result.map_err(|error| anyhow!("Failed to parse location registry {:?}\n -> {}\npath:{}", registry, error, error.path()))
}


lazy_static! {
    static ref COMMENT_PROP_REGEX: Regex = Regex::new(concat!(
        r#"(?m)^"#, // multi-line mode: ^ and $ match begin/end of line
        r#"(?P<indent1>\s*)-\s+index:\s+(?P<index>\d+)(?:\s+\#\s*(?P<yaml_cmt>.*))?"#,
        r#"\n"#,
        r#"(?P<indent2>\s*)  component_name:\s+(?P<comp_name>.*)$"#,
    )).unwrap();
}

pub fn reformat_to_yaml_comments(input: &str) -> Result<String> {
    debug!("reformat_to_yaml_comments - input=\n{}", input);
    try_replace_component(&*COMMENT_PROP_REGEX, input)
}

// Can't do fallible replace_all with regex so have to write ad-hoc replacement
fn try_replace_component(re: &Regex, haystack: &str) -> Result<String> {
    let mut new = String::with_capacity(haystack.len());
    let mut last_match = 0;
    for caps in re.captures_iter(haystack) {
        let everything = &caps.get(0).unwrap(); // apparently we can always unwrap the 0 match
        let indent1 = &caps["indent1"];
        let indent2 = &caps["indent2"];

        let index = &caps["index"];
        if indent1.len() != indent2.len() {
            bail!("indentation error for component (index={}", index);
        }

        let comp_name = &caps["comp_name"];
        let comp_name: String = serde_yaml::from_str(comp_name)?;
        let yaml_cmt = &caps.name("yaml_cmt");
        let full_comment: String = match yaml_cmt {
            None => comp_name.to_owned(),
            Some(m) => format!("{} [{}]", comp_name, m.as_str()),
        };

        new.push_str(&haystack[last_match..everything.start()]);
        new.push_str(indent1);
        new.push_str("- ");
        new.push_str(index);
        new.push_str(" # ");
        new.push_str(&full_comment);
        last_match = everything.end();
    }
    new.push_str(&haystack[last_match..]);
    Ok(new)
}

#[cfg(test)]
mod test_deserialize {

    use std::io::BufReader;
    use std::path::PathBuf;

    use crate::module::components::{Component, Components, FullComponent};
    use crate::lowercase::lwc;
    use crate::module::file_module_origin::FileModuleOrigin;
    use crate::module::gen_mod::{GeneratedMod, GenModComponent};
    use crate::module::global_locations::{GlobalLocations, LocationRegistry};
    use crate::module::location::github::GithubDescriptor;
    use crate::module::location::http::Http;
    use crate::module::location::{ConcreteLocation, Location};
    use crate::module::location::source::Source;
    use crate::module::module::Module;
    use crate::module::weidu_mod::WeiduMod;
    use crate::post_install::PostInstall;

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
                locations : GlobalLocations::default(),
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
                locations : GlobalLocations::default(),
                modules : vec![
                    Module::Mod {
                        weidu_mod: WeiduMod {
                            name: lwc!("aaa"),
                            components: Components::List(vec! [ Component::Simple(1) ]),
                            location: Some(Location::Concrete {
                                concrete: ConcreteLocation {
                                    source: Source::Http(Http { http: "http://example.com/my-mod".to_string(), rename: None, ..Default::default() }),
                                    ..Default::default()
                                }
                            }),
                            ..Default::default()
                        },
                    },
                    Module::Mod {
                        weidu_mod: WeiduMod {
                            name: lwc!("aaaa"),
                            components: Components::List(vec! [ Component::Simple(1) ]),
                            location: Some(Location::Concrete {
                                concrete: ConcreteLocation {
                                    source: Source::Http(Http { http: "http://example.com/my-mod".to_string(), rename: None, ..Default::default() }),
                                    ..Default::default()
                                }
                            }),
                            description: Some("some description".to_string()),
                            post_install: Some(PostInstall::Interrupt),
                            ignore_warnings: true,
                            ..Default::default()
                        },
                    },
                    Module::Generated {
                        gen:  GeneratedMod {
                            gen_mod: lwc!("ccc"),
                            files: vec![
                                FileModuleOrigin::Local { local: "my_subdir".to_string(), glob: None },
                            ],
                            description: None,
                            component: GenModComponent { index: 0, name: None },
                            post_install: Some(PostInstall::WaitSeconds { wait_seconds:10 }),
                            ignore_warnings: true,
                            allow_overwrite: true,
                        },
                    },
                    Module::Generated {
                        gen:  GeneratedMod {
                            gen_mod: lwc!("ddd"),
                            files: vec![
                                FileModuleOrigin::Local { local: "my_other_subdir".to_string(), glob: Some("*.itm".to_string()) },
                            ],
                            description: None,
                            post_install: None,
                            component: GenModComponent { index: 10, name: Some("Do whatever".to_string()) },
                            ignore_warnings: true,
                            allow_overwrite: true,
                        },
                    },
                ],
            }
        )
    }

    #[test]
    fn serialize_manifest_with_modules() {

        let manifest = super::Manifest {
            version : "1".to_string(),
            global : super::Global {
                game_language: "fr_FR".to_string(),
                lang_preferences: Some(vec!["french".to_string()]),
                patch_path: None,
                local_mods: Some("mods".to_string()),
                local_files: None,
            },
            locations : GlobalLocations::default(),
            modules : vec![
                Module::Mod {
                    weidu_mod: WeiduMod {
                        name: lwc!("aaa"),
                        components: Components::List(vec! [ Component::Simple(1) ]),
                        location: Some(Location::Concrete {
                            concrete: ConcreteLocation {
                                source: Source::Http(Http { http: "http://example.com/my-mod".to_string(), rename: None, ..Default::default() }),
                                ..Default::default()
                            }
                        }),
                        ignore_warnings: true,
                        ..Default::default()
                    },
                },
                Module::Generated {
                    gen:  GeneratedMod {
                        gen_mod: lwc!("ccc"),
                        files: vec![
                            FileModuleOrigin::Local { local: "my_subdir".to_string(), glob: None },
                        ],
                        description: None,
                        post_install: None,
                        component: GenModComponent { index: 0, name: None },
                        ignore_warnings: false,
                        allow_overwrite: false,
                    },
                },
                Module::Generated {
                    gen:  GeneratedMod {
                        gen_mod: lwc!("ddd"),
                        files: vec![
                            FileModuleOrigin::Local { local: "my_other_subdir".to_string(), glob: Some("*.itm".to_string()) },
                        ],
                        description: None,
                        post_install: None,
                        component: GenModComponent { index: 10, name: Some("Do whatever".to_string()) },
                        ignore_warnings: true,
                        allow_overwrite: true,
                    },
                },
            ],
        };

        let serialized = serde_yaml::to_string(&manifest).unwrap();
        println!("{}", serialized);
        assert_eq!(
            manifest,
            serde_yaml::from_str(&serialized).unwrap()
        )
    }

    #[test]
    fn check_read_manifest_with_locations() {
        use crate::module::location::github::Github;
        let manifest_path = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), "resources/test/manifest_with_locations.yml");
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
                locations : GlobalLocations::from([
                    (lwc!("aaa"), ConcreteLocation { source: Source::Http(Http::from("http://example.com/my-mod")), ..Default::default() }),
                    (lwc!("aaaa"), ConcreteLocation { source: Source::Local { local: "directory/my-other-mod.zip".to_owned() }, ..Default::default() }),
                    (lwc!("bbb"),ConcreteLocation { source: Source::Github(Github {
                        github_user: "some_user".to_owned(), repository: "mod-repo".to_owned(),
                        descriptor: GithubDescriptor::Tag { tag: "v324".to_owned() },
                        ..Default::default()
                    }), ..Default::default() })
                ]).with_external(LocationRegistry::Absolute { path: "/directory/locations.yml".to_owned() }),
                modules : vec![],
            }
        )
    }

    #[test]
    fn check_read_manifest_with_empty_locations() {
        let manifest_path = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), "resources/test/manifest_with_empty_locations.yml");
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
                locations : GlobalLocations::from([]),
                modules : vec![],
            }
        )
    }

    #[test]
    fn assemble_manifest_with_locations() {
        use crate::module::location::github::Github;
        let manifest_root = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), "resources/test");
        let manifest_path = format!("{}/{}", manifest_root, "manifest_with_real_ext_locations.yml");
        let manifest = Manifest::assemble_from_path(&manifest_path, &PathBuf::from(&manifest_root)).unwrap();
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
                locations : GlobalLocations::from([
                    (lwc!("aaa"), ConcreteLocation { source: Source::Http(Http::from("http://example.com/my-mod")), ..Default::default() }),
                    (lwc!("aaaa"), ConcreteLocation { source: Source::Local { local: "directory/my-other-mod.zip".to_owned() }, ..Default::default() }),
                    (lwc!("bbb"),ConcreteLocation { source: Source::Github(Github {
                        github_user: "some_user".to_owned(), repository: "mod-repo".to_owned(),
                        descriptor: GithubDescriptor::Tag { tag: "v324".to_owned() },
                        ..Default::default()
                    }), ..Default::default() })
                ]).with_external(LocationRegistry::Local { local: "registries/external-locations.yml".to_owned() }),
                modules : vec![],
            }
        )
    }

    #[test]
    fn read_manifest_convert_comments() {
        let manifest_root = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), "resources/test");
        let manifest_path = format!("{}/{}", manifest_root, "manifest_with_component_comments.yml");
        let manifest = Manifest::read_path_convert_comments(&PathBuf::from(manifest_path)).unwrap();
        assert_eq!(
            manifest,
            super::Manifest {
                version : "1".to_string(),
                global : super::Global {
                    game_language: "fr_FR".to_string(),
                    lang_preferences: Some(vec!["french".to_string()]),
                    ..Default::default()
                },
                modules : vec![
                    Module::Mod {
                        weidu_mod: WeiduMod {
                            name: lwc!("aaa"),
                            components: Components::List(vec! [
                                Component::Full(FullComponent { index: 1, component_name: "yaml comment".to_string()}),
                                Component::Full(FullComponent { index: 2, component_name: "comment as field".to_string()}),
                            ]),
                            ..Default::default()
                        },
                    },
                    Module::Mod {
                        weidu_mod: WeiduMod {
                            name: lwc!("bbb"),
                            components: Components::List(vec! [
                                Component::Simple(1)
                            ]),
                            ..Default::default()
                        },
                    },
                ],
                locations: GlobalLocations::default(),
            }
        );
    }

    #[test]
    fn write_manifest_convert_comments() {
        let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let test_path = project.join("target").join("test_data");
        let temp_dir = tempfile::tempdir_in(&test_path).unwrap();
        let output_path = temp_dir.as_ref().join("test.yml");

        let manifest = super::Manifest {
            version : "1".to_string(),
            global : super::Global {
                game_language: "fr_FR".to_string(),
                lang_preferences: Some(vec!["french".to_string()]),
                ..Default::default()
            },
            modules : vec![
                Module::Mod {
                    weidu_mod: WeiduMod {
                        name: lwc!("aaa"),
                        components: Components::List(vec! [
                            Component::Full(FullComponent { index: 1, component_name: "yaml comment 123".to_string()}),
                            Component::Full(FullComponent { index: 2, component_name: "comment as field 789".to_string()}),
                        ]),
                        ..Default::default()
                    },
                },
                Module::Mod {
                    weidu_mod: WeiduMod {
                        name: lwc!("bbb"),
                        components: Components::List(vec! [
                            Component::Simple(1)
                        ]),
                        ..Default::default()
                    },
                },
            ],
            locations: GlobalLocations::default(),
        };
        manifest.write(&output_path, true).unwrap();

        let file = std::fs::File::open(output_path).unwrap();
        let reader = BufReader::new(&file);
        let content = std::io::read_to_string(reader).unwrap();
        println!("comparing result:\n---\n{}\n---", content);

        assert!(content.contains("- 1 # yaml comment 123"));
        assert!(content.contains("- 2 # comment as field 789"));
    }
}
