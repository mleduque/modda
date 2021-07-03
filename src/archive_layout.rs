
use serde::{Deserialize, Serialize};


#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum Layout {
    #[serde(rename = "single_dir")]
    SingleDir { strip_leading: Option<usize> },
    #[serde(rename = "single_dir_plus_tp2")]
    SingleDirPlusTp2 { tp2: String, strip_leading: Option<usize> },
    // other layouts to be added as needed
    // All ?
    // Explicit ?
}

impl Default for Layout {
    fn default() -> Self {
        Layout::SingleDir { strip_leading: None }
    }
}

impl Layout {
    pub fn to_glob(&self, module_name: &str) -> Vec<String> {
        use Layout::*;
        let prefix = self.strip_pattern();
        let prefix = if prefix.is_empty() {
            prefix
        } else {
            format!("{}/", prefix)
        };
        match self {
            SingleDir { .. } => vec![format!("{}{}", prefix, module_name)],
            SingleDirPlusTp2 { tp2, .. } => vec![
                    format!("{}{}", prefix, module_name), 
                    format!("{}{}", prefix, tp2),
                ],
        }
    }

    fn strip_level(&self) -> usize {
        use Layout::*;
        match self {
            SingleDir { strip_leading: None } 
            | SingleDirPlusTp2 { strip_leading: None, .. } => 0,
            SingleDir { strip_leading: Some(v) } => *v,
            SingleDirPlusTp2 { strip_leading: Some(v), .. } => *v,
        }
    }

    fn strip_pattern(&self) -> String {
        vec!["*".to_string() ; self.strip_level()].join("/")
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FilePattern {
    pub pattern: String, 
    pub subdir: String,
}

#[cfg(test)]
impl Layout {
    pub fn single_dir(level: usize) -> Self { Layout::SingleDir { strip_leading: Some(level) } }
    pub fn with_tp2(tp2: String) -> Self { Layout::SingleDirPlusTp2 { tp2, strip_leading: None } }
    pub fn with_tp2_and_strip(tp2: String, level: usize) -> Self { 
        Layout::SingleDirPlusTp2 { tp2, strip_leading: Some(level) } 
    }
}

#[test]
fn test_strip_pattern() {
    assert_eq!(Layout::default().strip_pattern(), "");
    assert_eq!(Layout::single_dir(0).strip_pattern(), "");
    assert_eq!(Layout::single_dir(1).strip_pattern(), "*");
    assert_eq!(Layout::single_dir(2).strip_pattern(), "*/*");
    assert_eq!(Layout::single_dir(3).strip_pattern(), "*/*/*");

    assert_eq!(Layout::with_tp2("a".to_owned()).strip_pattern(), "");
    assert_eq!(Layout::with_tp2_and_strip("a".to_owned(), 0).strip_pattern(), "");
    assert_eq!(Layout::with_tp2_and_strip("a".to_owned(), 1).strip_pattern(), "*");
    assert_eq!(Layout::with_tp2_and_strip("a".to_owned(), 2).strip_pattern(), "*/*");
    assert_eq!(Layout::with_tp2_and_strip("a".to_owned(), 3).strip_pattern(), "*/*/*");
}

#[test]
fn test_to_glob() {
    assert_eq!(Layout::default().to_glob("toto"), vec!["toto".to_string()]);
    assert_eq!(Layout::single_dir(0).to_glob("toto"), vec!["toto".to_string()]);
    assert_eq!(Layout::single_dir(1).to_glob("toto"), vec!["*/toto".to_string()]);
    assert_eq!(Layout::single_dir(2).to_glob("toto"), vec!["*/*/toto".to_string()]);
    assert_eq!(Layout::single_dir(3).to_glob("toto"), vec!["*/*/*/toto".to_string()]);

    assert_eq!(Layout::with_tp2("a".to_owned()).to_glob("toto"), 
                                    vec!["toto".to_string(), "a".to_string()]);
    assert_eq!(Layout::with_tp2_and_strip("a".to_owned(), 0).to_glob("toto"), 
                                    vec!["toto".to_string(), "a".to_string()]);
    assert_eq!(Layout::with_tp2_and_strip("a".to_owned(), 1).to_glob("toto"), 
                                    vec!["*/toto".to_string(), "*/a".to_string()]);
    assert_eq!(Layout::with_tp2_and_strip("a".to_owned(), 2).to_glob("toto"), 
                                    vec!["*/*/toto".to_string(), "*/*/a".to_string()]);
    assert_eq!(Layout::with_tp2_and_strip("a".to_owned(), 3).to_glob("toto"), 
                                    vec!["*/*/*/toto".to_string(), "*/*/*/a".to_string()]);
}
