
use std::fmt::{self, Display};
use std::str::FromStr;
use std::marker::PhantomData;

use serde::{Deserialize, Deserializer, Serialize};
use serde::de::{self, Visitor, SeqAccess};

#[derive(Deserialize, Serialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum Components {
    Ask,
    None,
    List(Vec<Component>),
}
impl Default for Components {
    fn default() -> Self {
        Components::Ask
    }
}

#[derive(Debug)]
pub struct ParseComponentError(String);

impl de::Error for ParseComponentError {
    fn custom<T: Display>(msg: T) -> Self {
        ParseComponentError(msg.to_string())
    }
}

impl Display for ParseComponentError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParseComponentError(msg) => formatter.write_str(msg),
        }
    }
}

impl std::error::Error for ParseComponentError {}


impl FromStr for Components {
    type Err = ParseComponentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        return match s {
            "ask" => Ok(Components::Ask),
            "none" => Ok(Components::None),
            _ => Err(ParseComponentError(s.to_string())),
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

pub fn component_deser<'de, D>(deserializer: D) -> Result<Components, D::Error>
where
    D: Deserializer<'de>,
{
    // This is a Visitor that forwards string types to T's `FromStr` impl and
    // forwards seq types to T's `Deserialize` impl. The `PhantomData` is to
    // keep the compiler from complaining about T being an unused generic type
    // parameter. We need T in order to know the Value type for the Visitor
    // impl.
    struct StringOrComponents(PhantomData<fn() -> Components>);

    impl<'de> Visitor<'de> for StringOrComponents {
        type Value = Components;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("'ask', 'none' or list of components")
        }

        fn visit_str<E>(self, value: &str) -> Result<Components, E>
        where E: de::Error {
            return match FromStr::from_str(value) {
                Ok(result) => Ok(result),
                Err(error) => Err(de::Error::custom(error.to_string())),
            };
        }

        fn visit_seq<A>(self, seq: A) -> Result<Components, A::Error>
        where A: SeqAccess<'de>, {
            Deserialize::deserialize(de::value::SeqAccessDeserializer::new(seq))
        }
    }

    deserializer.deserialize_any(StringOrComponents(PhantomData))
}

#[cfg(test)]
mod test_deserialize {

    use crate::lowercase::lwc;
    use crate::module::weidu_mod::WeiduMod;

    use super::{Component, Components};

    #[test]
    fn deserialize_ask() {
        let yaml = r#"
        name: mod_name
        components: ask
        "#;
        let module: WeiduMod = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            module,
            WeiduMod {
                name: lwc!("mod_name"),
                components: Components::Ask,
                ..Default::default()
            }
        );
    }

    #[test]
    fn deserialize_none() {
        let yaml = r#"
        name: mod_name
        components: none
        "#;
        let module: WeiduMod = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            module,
            WeiduMod {
                name: lwc!("mod_name"),
                components: Components::None,
                ..Default::default()
            }
        );
    }

    #[test]
    fn deserialize_list() {
        let yaml = r#"
        name: mod_name
        components:
            - 1
        "#;

        let module: WeiduMod = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            module,
            WeiduMod {
                name: lwc!("mod_name"),
                components: Components::List(vec![Component::Simple(1)]),
                ..Default::default()
            }
        );
    }
}
