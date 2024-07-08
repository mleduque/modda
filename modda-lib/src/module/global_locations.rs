use std::collections::HashMap;
use std::iter::FromIterator;

use serde::{Deserialize, Serialize};

use crate::lowercase::LwcString;

use super::location::location::ConcreteLocation;

///
/// Global locations, used when a mod is not present in game directory and has no location defined in-site.
/// Can contain both a list of location definitions and a list of external files, that each defines locations
///
/// ```yaml
/// entries:
///     TDD:
///         #location definition, same as in-site location
///         http: http://europe.iegmc.net/tdd-weidu/TDDv1.14.rar
/// external:
///     - path: /directory/my-global-locations.yml
///     - local: my-local-loctions.yml
/// ```
///
/// * `entries` locations are looked at first
/// * then (if no match) `external` files are searched, in the order they are listed
///
/// Both `external` and `entries` can be omitted.
#[derive(Deserialize, Serialize, Debug, PartialEq, Default, Clone)]
pub struct GlobalLocations {
    #[serde(default)]
    pub external: Vec<LocationRegistry>,
    #[serde(default)]
    pub entries: HashMap<LwcString, ConcreteLocation>,
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
