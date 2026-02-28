use std::fmt::Display;
use std::str::FromStr;

use anyhow::bail;
use bytesize::ByteSize;
use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum SizeHint {
    Bytes(u64),
    Human(ByteSize),
}

impl SizeHint {
    pub fn from_bytes(bytes: u64) -> Self { SizeHint::Bytes(bytes) }

    pub fn size(&self) -> u64 {
        match self {
            SizeHint::Bytes(b) => *b,
            SizeHint::Human(v) => v.as_u64(),
        }
    }
}

impl Display for SizeHint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SizeHint::Bytes(v) => v.fmt(f),
            SizeHint::Human(v) => v.fmt(f)
        }
    }
}

impl FromStr for SizeHint {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match ByteSize::from_str(s) {
            Err(err) => bail!("Can't convert value to byte size {s} : {err}"),
            Ok(v) => Ok(SizeHint::Human(v)),
        }
    }
}

#[cfg(test)]
mod tests {

    use serde::{Deserialize, Serialize};

    use crate::module::location::size_hint::SizeHint;


    #[derive(Deserialize, Serialize, Debug,PartialEq)]
    struct TestStruct {
        pub size: SizeHint,
    }

    #[test]
    fn deserialize_size_hint_as_raw_bytes() {
        let yaml = r#"
        size: 123
        "#;
        let test_struct: TestStruct = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            test_struct,
            TestStruct { size: SizeHint::from_bytes(123u64) },
        )
    }

    #[test]
    fn deserialize_size_hint_as_kilobytes() {
        let yaml = r#"
        size: 123Kb
        "#;
        let test_struct: TestStruct = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            test_struct,
            TestStruct { size: SizeHint::Human(bytesize::ByteSize::kb(123)) },
        )
    }

    #[test]
    fn deserialize_size_hint_incorrect_string_value() {
        let yaml = r#"
        size: aaaz
        "#;
        let test_struct: serde_yaml::Result<TestStruct> = serde_yaml::from_str(yaml);
        assert!(test_struct.is_err())
    }
}
