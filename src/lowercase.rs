
use std::fmt::{Display, Formatter, Result};

/// A string that is guaranteed to be lowercase
#[derive(Debug, Clone, PartialEq)]
pub struct LwcString(String);

impl LwcString {
    pub fn new(origin: &str) -> LwcString {
        LwcString(origin.to_lowercase())
    }

    pub fn inner(&self) -> &str {
        &self.0
    }
}

impl PartialEq<String> for LwcString {
    fn eq(&self, other: &String) -> bool {
        &self.0 == &other.to_lowercase()
    }
}

impl PartialEq<LwcString> for String {
    fn eq(&self, other: &LwcString) -> bool {
        &other == &self
    }
}

impl Display for LwcString {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.0)
    }
}
