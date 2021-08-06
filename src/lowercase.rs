
use std::fmt::{Display, Formatter, Result};

/// A string that is guaranteed to be lowercase
#[derive(Debug, Clone, PartialEq)]
pub struct LwcString(String);

impl LwcString {
    pub fn new(origin: &str) -> LwcString {
        LwcString(origin.to_lowercase())
    }

    #[allow(unused)]
    pub fn inner(self) -> String {
        self.0
    }
}

impl AsRef<str> for LwcString {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl PartialEq<String> for LwcString {
    fn eq(&self, other: &String) -> bool {
        self == &other.as_str()
    }
}

impl PartialEq<&str> for LwcString {
    fn eq(&self, other: &&str) -> bool {
        &self.0 == &other.to_lowercase()
    }
}

impl From<&str> for LwcString {
    fn from(base: &str) -> Self { LwcString(base.to_lowercase()) }
}

impl Display for LwcString {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.0)
    }
}

impl std::ops::Add<&LwcString> for LwcString {
    type Output = Self;
    fn add(self, other: &LwcString) -> Self {
        Self(self.0 + &other.0)
    }
}

impl std::ops::Add<&str> for LwcString {
    type Output = Self;
    fn add(self, other: &str) -> Self {
        Self(self.0 + &other.to_lowercase())
    }
}

macro_rules! lwc {
    () => { LwcString::from("") };
    ($e: expr) => {{
        let base: &str = $e;
        LwcString::from(base)
    }};
}

pub trait ContainsStr {
    fn contains_str(&self, value: &str) -> bool;
    fn find_str(&self, value: &str) -> Option<usize>;
}
impl ContainsStr for Vec<LwcString> {
    fn contains_str(&self, value: &str) -> bool {
        self.iter().any(|item| item == &value)
    }
    fn find_str(&self, value: &str) -> Option<usize> {
        self.iter().enumerate().find(|(_, item)| item == &&value).map(|(idx, _)| idx)
    }
}
