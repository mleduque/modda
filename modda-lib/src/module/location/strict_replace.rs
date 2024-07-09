use std::num::NonZeroU32;
use std::sync::OnceLock;

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;


#[derive(Debug, PartialEq, Clone)]
pub enum CheckReplace {
    BoolValue(bool),
    Exact(NonZeroU32),
    MoreThan(NonZeroU32),
}

impl Default for CheckReplace {
    fn default() -> Self {
        Self::BoolValue(false)
    }
}

impl Serialize for CheckReplace {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where S: serde::Serializer {
        match self {
            CheckReplace::BoolValue(value) => value.serialize(serializer),
            CheckReplace::Exact(value) => value.serialize(serializer),
            CheckReplace::MoreThan(value) => format!(">{value}").serialize(serializer),
        }
    }
}

fn more_than_regex() -> &'static Regex {
    static DISABLE_FILE_REGEX: OnceLock<Regex> = OnceLock::new();
    DISABLE_FILE_REGEX.get_or_init(|| {
        Regex::new(r"^\s*>\s*(?<num>\d+)\s*$").unwrap()
    })
}

impl <'de> Deserialize<'de> for CheckReplace {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where D: serde::Deserializer<'de> {
        let helper = Value::deserialize(deserializer)?;
        match helper {
            Value::Bool(ref value) => Ok(Self::BoolValue(*value)),
            Value::Number(value) => {
                match value.as_i64() {
                    None => Err(serde::de::Error::custom("'strict' property can't be a floating point number")),

                    Some(num) if num < 0 => Err(serde::de::Error::custom("'strict' property can be negative")),
                    Some(num) => match u32::try_from(num) {
                        Ok(num) => match NonZeroU32::new(num) {
                            None => Err(serde::de::Error::custom("'strict' property can't be zero")),
                            Some(num) => Ok(Self::Exact(num)),
                        }
                        Err(error) => Err(serde::de::Error::custom(format!("'strict' property is invalid (hint: maybe too big?)\n{error}"))),
                    }
                }
            }
            Value::String(value) => {
                match more_than_regex().captures(&value) {
                    Some(captures) => match captures.name("num") {
                        Some(s) => match s.as_str(). parse::<i64>() {
                            Ok(num) => match u32::try_from(num) {
                                Ok(num) =>  match NonZeroU32::new(num) {
                                    None => Ok(Self::BoolValue(true)),
                                    Some(num) => Ok(Self::MoreThan(num)),
                                }
                                Err(err) => Err(serde::de::Error::custom(format!("'strict' property is invalid (hint: maybe too big or negative?)\n{err}"))),
                            }
                            Err(err) => Err(serde::de::Error::custom(format!("'strict' property is invalid (hint: maybe too big?)\n{err}"))),
                        }
                        None => Err(serde::de::Error::custom(format!("'strict' property, missing number after `>` (expected format is `>123`)"))),
                    }
                    None => Err(serde::de::Error::custom(format!("'strict' property, invalid content (expected `true`, `false`, number (ex. `12`) or 'more than number' (ex. `>123`)"))),
                }
            }
            _ => Err(serde::de::Error::custom("'strict' property should be `true`, `false`, number (ex. `12`) or 'more than number' (ex. `>123`"))
        }
    }
}
