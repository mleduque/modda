use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::disable_condition::DisableCondition;


#[derive(Deserialize, Serialize, Debug, PartialEq, Default, Clone)]
pub struct ManifestConditions (HashMap<String, DisableCondition>);

impl ManifestConditions {
    pub fn new(conditions: HashMap<String, DisableCondition>) -> Self { Self(conditions) }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn get(&self, key: &str) -> Option<&DisableCondition>{
        self.0.get(key)
    }
}
