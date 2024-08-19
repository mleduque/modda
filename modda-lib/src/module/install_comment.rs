
use std::marker::PhantomData;

use anyhow::Result;
use serde::de::{self, MapAccess, Visitor};
use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, PartialEq, Default, Clone)]
pub struct InstallationComments {
    pub general: Option<String>,
    pub before: Option<InstallationHint>,
    pub after: Option<InstallationHint>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum InstallationHint {
    ModHint(String),
    ComponentHint { name:String, component: u32 },
    List(Vec<InstallationHint>),
}


pub fn installation_hint_deser<'de, D>(deserializer: D) -> Result<InstallationHint, D::Error>
        where D: Deserializer<'de> {

    use serde::de::Error as DeError;
    use std::fmt;

    struct Helper(PhantomData<fn() -> InstallationHint>);

    impl<'de> Visitor<'de> for Helper {
        type Value = InstallationHint;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("mod (string), component (object with name and component) or list of those")
        }

        fn visit_str<E>(self, value: &str) -> Result<InstallationHint, E>
                where E: DeError {
            Ok(InstallationHint::ModHint(value.to_string()))
        }

        fn visit_map<M>(self, mut map: M) -> Result<InstallationHint, M::Error>
                where M: MapAccess<'de> {
            let mut name: Option<String> = None;
            let mut component: Option<u32> = None;
            while let Some((key, value)) = map.next_entry::<String, Value>()? {
                match key.as_str() {
                    "name" => match name {
                        Some(_) => return Err(DeError::custom("multiple 'name' fields".to_string())),
                        None => match value.as_str() {
                            None => return Err(DeError::custom("incorrect type for 'name' property")),
                            Some(value) => name = Some(value.to_string())
                        },
                    },
                    "component" => match component {
                        Some(_) => return Err(DeError::custom("mutiple 'component' properties")),
                        None => match value.as_u64() {
                            None => return Err(DeError::custom("incorrect type for 'component' property")),
                            Some(value) => match u32::try_from(value) {
                                Err(_) => return Err(DeError::custom("incorrect value for 'component' property (too big)")),
                                Ok(value) => component = Some(value)
                            }
                        },
                    },
                    _ => {}
                }
            }
            match (name, component) {
                (Some(name), Some(component)) => Ok(InstallationHint::ComponentHint { name: name.to_string(), component }),
                (Some(_), None) => Err(DeError::custom("missing property 'component'")),
                (None, Some(_)) => Err(DeError::custom("missing property 'name'")),
                _ => Err(DeError::custom("missing both properties 'name' and 'component'")),
            }
        }

        fn visit_seq<A>(self, mut seq: A) -> std::result::Result<InstallationHint, A::Error>
        where A: de::SeqAccess<'de>,
        {
            let mut result = match seq.size_hint() {
                Some(size) => Vec::with_capacity(size),
                None => Vec::new(),
            };
            while let Some(item) = seq.next_element::<InstallationHint>()? {
                result.push(item);
            }
            Ok(InstallationHint::List(result))
        }
    }

    deserializer.deserialize_any(Helper(PhantomData))
}

impl <'de> Deserialize<'de> for InstallationHint {

    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        installation_hint_deser(deserializer)
    }
}

impl Serialize for InstallationHint {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        match self {
            InstallationHint::ModHint(mod_name) => serializer.serialize_str(mod_name),
            InstallationHint::ComponentHint { name, component } => {
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("name", name)?;
                map.serialize_entry("component", component)?;
                map.end()
            }
            InstallationHint::List(items) => {
                let mut seq = serializer.serialize_seq(Some(items.len()))?;
                for item in items {
                    seq.serialize_element(item)?;
                }
                seq.end()
            }
        }
    }
}

#[cfg(test)]
mod test_deser {
    use crate::module::install_comment::{InstallationComments, InstallationHint};

    #[test]
    fn deser_installation_comment_single_string() {
        let yaml = r#"
            general: near the middle
            before: foo
            after: bar
        "#;

        let i_comments: InstallationComments = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            i_comments,
            InstallationComments {
                general: Some("near the middle".to_string()),
                before: Some(InstallationHint::ModHint("foo".to_string())),
                after: Some(InstallationHint::ModHint("bar".to_string())),
            }
        );
    }

    #[test]
    fn deser_installation_comment_single_component() {
        let yaml = r#"
            after: { name: bar, component: 12 }
        "#;

        let i_comments: InstallationComments = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            i_comments,
            InstallationComments {
                general: None,
                before: None,
                after: Some(InstallationHint::ComponentHint { name: "bar".to_string(), component: 12u32 }),
            }
        );
    }

    #[test]
    fn deser_installation_comment_list_of_strings() {
        let yaml = r#"
            before:
                - foo
                - bar
        "#;

        let i_comments: InstallationComments = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            i_comments,
            InstallationComments {
                general: None,
                before: Some(InstallationHint::List(vec![
                    InstallationHint::ModHint("foo".to_string()),
                    InstallationHint::ModHint("bar".to_string()),
                ])),
                after: None,
            }
        );
    }

    #[test]
    fn deser_installation_comment_list_of_components() {
        let yaml = r#"
            before:
                - { name: foo, component: 12 }
                - { name: bar, component: 13 }
        "#;

        let i_comments: InstallationComments = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            i_comments,
            InstallationComments {
                general: None,
                before: Some(InstallationHint::List(vec![
                    InstallationHint::ComponentHint { name: "foo".to_string(), component: 12u32 },
                    InstallationHint::ComponentHint { name: "bar".to_string(), component: 13u32 },
                ])),
                after: None,
            }
        );
    }

    #[test]
    fn deser_installation_comment_mixed_list() {
        let yaml = r#"
            after:
                - foo
                - { name: bar, component: 13 }
        "#;

        let i_comments: InstallationComments = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            i_comments,
            InstallationComments {
                general: None,
                before: None,
                after: Some(InstallationHint::List(vec![
                    InstallationHint::ModHint("foo".to_string()),
                    InstallationHint::ComponentHint { name: "bar".to_string(), component: 13u32 },
                ])),
            }
        );
    }

    #[test]
    fn deser_installation_single_component_missing_name() {
        let yaml = r#"
            after: { component: 13 }
        "#;

        serde_yaml::from_str::<InstallationComments>(yaml).unwrap_err();
    }

    #[test]
    fn deser_installation_single_component_missing_component() {
        let yaml = r#"
            after: { name: foo }
        "#;

        serde_yaml::from_str::<InstallationComments>(yaml).unwrap_err();
    }

    #[test]
    fn deser_installation_component_list_missing_name() {
        let yaml = r#"
            after:
                - { component: 13 }
        "#;

        serde_yaml::from_str::<InstallationComments>(yaml).unwrap_err();
    }

    #[test]
    fn deser_installation_component_list_missing_component() {
        let yaml = r#"
            after:
                - { name: foo }
        "#;

        serde_yaml::from_str::<InstallationComments>(yaml).unwrap_err();
    }
}

#[cfg(test)]
mod test_ser {
    use crate::module::install_comment::{InstallationComments, InstallationHint};


    #[test]
    fn ser_installation_comment_single_string() {
        let comment = InstallationComments {
            general: Some("near the middle".to_string()),
            before: Some(InstallationHint::ModHint("foo".to_string())),
            after: Some(InstallationHint::ModHint("bar".to_string())),
        };

        let as_yaml = serde_yaml::to_string(&comment).unwrap();
        println!("{as_yaml}");
        let reverted: InstallationComments = serde_yaml::from_str(&as_yaml).unwrap();

        assert_eq!(comment, reverted)
    }

    #[test]
    fn ser_installation_comment_single_component() {
        let comment = InstallationComments {
            general: None,
            before: None,
            after: Some(InstallationHint::ComponentHint { name: "bar".to_string(), component: 12u32 }),
        };

        let as_yaml = serde_yaml::to_string(&comment).unwrap();
        println!("{as_yaml}");
        let reverted: InstallationComments = serde_yaml::from_str(&as_yaml).unwrap();

        assert_eq!(comment, reverted)
    }

    #[test]
    fn ser_installation_comment_mixed_list() {
        let comment = InstallationComments {
            general: None,
            before: None,
            after: Some(InstallationHint::List(vec![
                InstallationHint::ModHint("foo".to_string()),
                InstallationHint::ComponentHint { name: "bar".to_string(), component: 13u32 },
            ])),
        };

        let as_yaml = serde_yaml::to_string(&comment).unwrap();
        println!("{as_yaml}");
        let reverted: InstallationComments = serde_yaml::from_str(&as_yaml).unwrap();

        assert_eq!(comment, reverted)
    }
}
