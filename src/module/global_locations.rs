use std::collections::HashMap;
use std::collections::hash_map::RandomState;
use std::hash::Hash;
use std::iter::FromIterator;

use serde::{Deserialize, Serialize};

use crate::lowercase::LwcString;

use super::location::ConcreteLocation;

#[derive(Debug, PartialEq, Default, Clone)]
pub struct GlobalLocations {
    entries: HashMap<LwcString, ConcreteLocation>,
}

impl GlobalLocations {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn find(&self, name: &LwcString) -> Option<&ConcreteLocation> {
        self.entries.iter()
            .find(|(key, _)| key == &name)
            .map(|(_, value)| value)
    }

    pub fn put(mut self, key: &LwcString, value: ConcreteLocation) -> GlobalLocations {
        self.entries.insert(key.clone(), value);
        self
    }
}

impl <const N: usize> From<[(LwcString, ConcreteLocation); N]> for GlobalLocations {
    fn from(arr: [(LwcString, ConcreteLocation); N]) -> Self {
        Self { entries: HashMap::from_iter(arr) }
    }
}

impl <'de> Deserialize<'de> for GlobalLocations {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: serde::Deserializer<'de> {
        let val = HashMap::deserialize(deserializer)?;
        Ok(GlobalLocations { entries: val })
    }
}

impl <'de> Serialize for GlobalLocations {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        self.entries.serialize(serializer)
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Default, Clone)]
struct LocationEntry {
    key: LwcString,
    #[serde(flatten)]
    value: ConcreteLocation,
}
