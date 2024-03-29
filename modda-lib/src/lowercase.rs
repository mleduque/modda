
use std::fmt::{Display, Formatter, Result, Debug};
use std::str::FromStr;

use serde::{Serialize, Deserialize};

/// A string that is guaranteed to be lowercase
#[derive(Clone, PartialEq, Serialize, Default, Hash, Eq)]
#[serde(into = "String")]
#[serde(from = "String")]
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

impl AsRef<String> for LwcString {
    fn as_ref(&self) -> &String {
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

impl FromStr for LwcString {
    type Err = Void;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(lwc!(s))
    }
}

impl Display for LwcString {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.0)
    }
}

impl <'de> Deserialize<'de> for LwcString {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
            where D: serde::Deserializer<'de> {
        let raw = String::deserialize(deserializer)?;
        Ok(Self::new(&raw))
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
impl Into<String> for LwcString {
    fn into(self) -> String {
        self.0
    }
}
impl From<String> for LwcString {
    fn from(input: String) -> Self {
        LwcString::new(&input)
    }
}

impl Debug for LwcString {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_str(&self.0)
    }
}

macro_rules! lwc {
    () => { LwcString::from("") };
    ($e: expr) => {{
        let base: &str = $e;
        crate::lowercase::LwcString::from(base)
    }};
}
pub(crate) use lwc;
use void::Void;

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



#[cfg(test)]
mod lowercase_string_tests {
    use super::LwcString;

    #[test]
    fn deserialize() {
        let result: LwcString = serde_yaml::from_str("AaBb123").unwrap();
        assert_eq!(
            result,
            lwc!("aabb123")
        )
    }

    #[test]
    fn serialize() {
        let input = lwc!("aabb123");
        let result = serde_yaml::to_string(&input).unwrap();
        assert_eq!(
            result,
            "aabb123\n"
        )
    }
}
