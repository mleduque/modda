use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, PartialEq, Default, Clone)]
pub struct Automation {
    pub when: String,
    pub answer: String,
}
