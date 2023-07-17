use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::lowercase::LwcString;

use super::location::ConcreteLocation;


#[derive(Deserialize, Serialize, Debug, PartialEq, Default, Clone)]
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
