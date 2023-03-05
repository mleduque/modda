
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, PartialEq, Default)]
pub struct InstallationComments {
    pub general: Option<String>,
    pub before: Option<String>,
    pub after: Option<String>,
}
