
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, PartialEq, Default)]
pub struct InstallationComments {
    pub general: Option<String>,
    pub before: Option<String>,
    pub after: Option<String>,
}
