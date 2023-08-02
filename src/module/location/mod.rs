pub mod github;
pub mod http;
pub mod source;

use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;

use anyhow::Result;
use serde::{Deserialize, Serialize, Deserializer, de, Serializer};
use serde::de::{Visitor, MapAccess};
use serde_with::skip_serializing_none;
use void::Void;

use crate::lowercase::{LwcString, lwc};
use crate::module::pre_copy_command::PrecopyCommand;
use crate::{archive_layout::Layout, patch_source::PatchDesc, replace::ReplaceSpec};

use self::source::Source;


#[derive(Debug, PartialEq, Clone)]
pub enum Location {
    Ref { r#ref: LwcString },
    Concrete { concrete: ConcreteLocation },
}

impl FromStr for Location {
    type Err = Void;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(Location::Ref { r#ref: lwc!(s) })
    }
}

impl<'de> Deserialize<'de> for Location {
    fn deserialize<D>(deserializer: D) -> Result<Location, D::Error>
            where D: Deserializer<'de> {
        location_deser(deserializer)
    }
}

impl Serialize for Location {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where S: Serializer {
        match self {
            Location::Ref { r#ref } => r#ref.serialize(serializer),
            Location::Concrete { concrete } => concrete.serialize(serializer),
        }
    }
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, PartialEq, Default, Clone)]
pub struct ConcreteLocation {
    #[serde(flatten)]
    pub source: Source,
    /// Specifies which files from the archive will be copied to the game directory.
    /// Read as a Unix shell style glob pattern (https://docs.rs/glob/0.3.0/glob/struct.Pattern.html)
    #[serde(default)]
    pub layout: Layout,
    pub patch: Option<PatchDesc>,
    /// regex-based search and replace, runs after patch.
    pub replace: Option<Vec<ReplaceSpec>>,
    pub precopy: Option<PrecopyCommand>,
}

pub fn location_deser<'de, D>(deserializer: D) -> Result<Location, D::Error>
        where D: Deserializer<'de> {
    // This is a Visitor that forwards string types to T's `FromStr` impl and
    // forwards seq types to T's `Deserialize` impl. The `PhantomData` is to
    // keep the compiler from complaining about T being an unused generic type
    // parameter. We need T in order to know the Value type for the Visitor
    // impl.
    struct StringOrStruct(PhantomData<fn() -> Location>);

    impl<'de> Visitor<'de> for StringOrStruct {
        type Value = Location;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("location reference (string) or concrete location definition")
        }

        fn visit_str<E>(self, value: &str) -> Result<Location, E>
                where E: de::Error {
            match Location::from_str(value) {
                Err(_) => Err(E::custom("normally unreachable 'void' error")),
                Ok(value) => Ok(value),
            }
        }

        fn visit_map<M>(self, map: M) -> Result<Location, M::Error>
                where M: MapAccess<'de> {
            let concrete = ConcreteLocation::deserialize(de::value::MapAccessDeserializer::new(map))?;
            Ok(Location::Concrete { concrete })
        }
    }

    deserializer.deserialize_any(StringOrStruct(PhantomData))
}

#[cfg(test)]
mod test_deserialize {
    use crate::lowercase::lwc;
    use crate::module::location::Location;
    use crate::module::location::github::{GitBranch, Github, GithubDescriptor};
    use crate::module::location::source::Source;
    use crate::module::weidu_mod::WeiduMod;
    use crate::replace::ReplaceSpec;
    use crate::module::refresh::RefreshCondition::Never;

    use super::ConcreteLocation;

    #[test]
    fn deserialize_source_github_branch() {
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
                descriptor: GithubDescriptor::Branch(GitBranch { branch: "main".to_string(), refresh: Never }),
                ..Default::default()
            })
        );
    }

    #[test]
    fn deserialize_source_github_tag() {
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
                ..Default::default()
            })
        );
    }

    #[test]
    fn deserialize_source_github_committag() {
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
                ..Default::default()
            })
        );
    }

    #[test]
    fn deserialize_source_github_release() {
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
                ..Default::default()
            })
        );
    }

    #[test]
    fn deserialize_source_github_branch_as_json() {
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
                descriptor: GithubDescriptor::Branch(GitBranch { branch: "main".to_string(), refresh: Never }),
                ..Default::default()
            })
        );
    }

    #[test]
    fn deserialize_location_with_replace_property() {
        let yaml = r#"
            github_user: "pseudo"
            repository: my-big-project
            tag: v1
            replace:
                - file_globs: [README.md]
                  replace: typpo
                  with: typo
        "#;
        let location : ConcreteLocation = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            location,
            ConcreteLocation {
                source: Source::Github(Github {
                    github_user: "pseudo".to_string(),
                    repository: "my-big-project".to_string(),
                    descriptor: GithubDescriptor::Tag { tag: "v1".to_string() },
                    ..Default::default()
                }),
                replace: Some(vec![
                    ReplaceSpec {
                        file_globs: vec!["README.md".to_string()],
                        replace: "typpo".to_string(),
                        with: "typo".to_string(),
                        ..Default::default()
                    }
                ]),
                ..Default::default()
            }
        )
    }

    #[test]
    fn deserialize_location_as_reference() {
        let yaml = r#"
            name: my_mod
            components: ask
            location: my_location_ref
        "#;
        let module : WeiduMod = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            module.location,
            Some(Location::Ref { r#ref: lwc!("my_location_ref") })
        )
    }

    #[test]
    fn deserialize_location_as_concrete() {
        let yaml = r#"
            name: my_mod
            components: ask
            location:
                github_user: "pseudo"
                repository: my-big-project
                tag: v1
        "#;
        let module : WeiduMod = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            module.location,
            Some(Location::Concrete { concrete: ConcreteLocation {
                source: Source::Github(Github {
                    github_user: "pseudo".to_string(),
                    repository: "my-big-project".to_string(),
                    descriptor: GithubDescriptor::Tag { tag: "v1".to_string() },
                    ..Default::default()
                }),
                ..Default::default()
            } })
        )
    }
}
