use std::collections::HashMap;
use std::iter::FromIterator;

use serde::{Deserialize, Serialize};

use crate::lowercase::LwcString;

use super::location::ConcreteLocation;

#[derive(Deserialize, Serialize, Debug, PartialEq, Default, Clone)]
pub struct GlobalLocations {
    #[serde(default)]
    external: Vec<LocationRegistry>,
    #[serde(default)]
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

    pub fn with_external(mut self, external_item: LocationRegistry) -> Self {
        self.external.push(external_item);
        self
    }
}

impl <const N: usize> From<[(LwcString, ConcreteLocation); N]> for GlobalLocations {
    fn from(arr: [(LwcString, ConcreteLocation); N]) -> Self {
        Self { external: vec![], entries: HashMap::from_iter(arr) }
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
pub enum LocationRegistry {
    Absolute { path: String },
    Local { local: String },
}
