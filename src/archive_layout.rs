
use serde::{Deserialize, Serialize};

use crate::manifest::Source;


#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Default)]
pub struct Layout {
    pub strip_leading: Option<usize>,
    #[serde(default, flatten)]
    pub layout: LayoutContent,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(tag = "layout_type")]
pub enum LayoutContent {
    #[serde(rename = "single_dir")]
    SingleDir,
    #[serde(rename = "single_dir_plus_tp2")]
    SingleDirPlusTp2 { tp2: Option<String> },
    #[serde(rename = "multi_dir")]
    MultipleDirs { dirs: Vec<String> },
    // other layouts to be added as needed
    // All ?
    // Explicit ?
}

impl Default for LayoutContent {
    fn default() -> Self {
        LayoutContent::SingleDir
    }
}

impl Layout {
    pub fn to_glob(&self, module_name: &str, location_source: &Source) -> Vec<String> {
        use LayoutContent::*;
        let prefix = self.strip_pattern(location_source);
        let prefix = if prefix.is_empty() {
            prefix
        } else {
            format!("{}/", prefix)
        };
        match &self.layout {
            SingleDir => vec![format!("{}{}", prefix, module_name)],
            SingleDirPlusTp2 { tp2: Some(tp2) } => vec![
                    format!("{}{}", prefix, module_name),
                    format!("{}{}", prefix, tp2),
                ],
            SingleDirPlusTp2 { tp2: None } => vec![
                    format!("{}{}", prefix, module_name),
                    format!("{}setup-{}.tp2", prefix, module_name),
                ],
            MultipleDirs { dirs } => dirs.iter().map(|dir|
                    format!("{}{}", prefix, dir)
                ).collect::<Vec<_>>(),
        }
    }

    fn strip_level(&self, source: &Source) -> usize {
        match self {
            Layout { strip_leading: None, .. } => source.default_strip_leading(),
            Layout { strip_leading: Some(v), .. } => *v,
        }
    }

    fn strip_pattern(&self, source: &Source) -> String {
        vec!["*".to_string() ; self.strip_level(source)].join("/")
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FilePattern {
    pub pattern: String,
    pub subdir: String,
}

#[cfg(test)]
impl LayoutContent {
    pub fn with_tp2_default() -> Self { LayoutContent::SingleDirPlusTp2 { tp2: None } }
    pub fn with_tp2(tp2: String) -> Self { LayoutContent::SingleDirPlusTp2 { tp2: Some(tp2) } }
    pub fn multi_dir(dirs: Vec<String>) -> Self { LayoutContent::MultipleDirs { dirs } }
}

#[cfg(test)]
impl Layout {
    pub fn single_dir(strip_lvl: usize) -> Self {
        Layout {
            layout: LayoutContent::SingleDir,
            strip_leading: Some(strip_lvl),
        }
    }
    pub fn with_tp2(tp2: String) -> Self {
        Layout {
            layout: LayoutContent::with_tp2(tp2),
            ..Self::default()
        }
    }
    pub fn with_tp2_default() -> Self {
        Layout {
            layout: LayoutContent::with_tp2_default(),
            ..Self::default()
        }
    }
    pub fn with_tp2_and_strip(tp2: String, strip_lvl: usize) -> Self {
        Layout {
            layout: LayoutContent::with_tp2(tp2),
            strip_leading: Some(strip_lvl),
        }
    }
    pub fn with_tp2_default_and_strip(strip_lvl: usize) -> Self {
        Layout {
            layout: LayoutContent::with_tp2_default(),
            strip_leading: Some(strip_lvl),
        }
    }
    pub fn multi_dir(dirs: Vec<String>) -> Self {
        Layout {
            layout: LayoutContent::multi_dir(dirs),
            ..Self::default()
        }
    }
    pub fn multi_dir_and_strip(dirs: Vec<String>, strip_lvl: usize) -> Self {
        Layout {
            layout: LayoutContent::multi_dir(dirs),
            strip_leading: Some(strip_lvl),
        }
    }
}

#[test]
fn test_strip_pattern() {
    let http_source = Source::http_source();
    assert_eq!(Layout::default().strip_pattern(&http_source), "");
    assert_eq!(Layout::single_dir(0).strip_pattern(&http_source), "");
    assert_eq!(Layout::single_dir(1).strip_pattern(&http_source), "*");
    assert_eq!(Layout::single_dir(2).strip_pattern(&http_source), "*/*");
    assert_eq!(Layout::single_dir(3).strip_pattern(&http_source), "*/*/*");

    let gh_release_source = Source::gh_branch_source();
    assert_eq!(Layout::with_tp2("a".to_owned()).strip_pattern(&gh_release_source), "*");
    assert_eq!(Layout::with_tp2_and_strip("a".to_owned(), 0).strip_pattern(&gh_release_source), "");
    assert_eq!(Layout::with_tp2_and_strip("a".to_owned(), 1).strip_pattern(&gh_release_source), "*");
    assert_eq!(Layout::with_tp2_and_strip("a".to_owned(), 2).strip_pattern(&gh_release_source), "*/*");
    assert_eq!(Layout::with_tp2_and_strip("a".to_owned(), 3).strip_pattern(&gh_release_source), "*/*/*");

    let dirs = vec!["a".to_owned(), "b".to_owned()];
    assert_eq!(Layout::multi_dir(dirs.clone()).strip_pattern(&http_source), "");
    assert_eq!(Layout::multi_dir_and_strip(dirs.clone(), 0).strip_pattern(&http_source), "");
    assert_eq!(Layout::multi_dir_and_strip(dirs.clone(), 1).strip_pattern(&http_source), "*");
    assert_eq!(Layout::multi_dir_and_strip(dirs.clone(), 2).strip_pattern(&http_source), "*/*");
    assert_eq!(Layout::multi_dir_and_strip(dirs.clone(), 3).strip_pattern(&http_source), "*/*/*");
}

#[test]
fn test_to_glob() {
    let http_source = Source::http_source();

    assert_eq!(Layout::default().to_glob("toto", &http_source), vec!["toto".to_string()]);
    assert_eq!(Layout::single_dir(0).to_glob("toto", &http_source), vec!["toto".to_string()]);
    assert_eq!(Layout::single_dir(1).to_glob("toto", &http_source), vec!["*/toto".to_string()]);
    assert_eq!(Layout::single_dir(2).to_glob("toto", &http_source), vec!["*/*/toto".to_string()]);
    assert_eq!(Layout::single_dir(3).to_glob("toto", &http_source), vec!["*/*/*/toto".to_string()]);

    let gh_release_source = Source::gh_release_source();

    assert_eq!(Layout::default().to_glob("toto", &gh_release_source), vec!["toto".to_string()]);
    assert_eq!(Layout::single_dir(0).to_glob("toto", &gh_release_source), vec!["toto".to_string()]);
    assert_eq!(Layout::single_dir(1).to_glob("toto", &gh_release_source), vec!["*/toto".to_string()]);
    assert_eq!(Layout::single_dir(2).to_glob("toto", &gh_release_source), vec!["*/*/toto".to_string()]);
    assert_eq!(Layout::single_dir(3).to_glob("toto", &gh_release_source), vec!["*/*/*/toto".to_string()]);

    let gh_branch_source = Source::gh_branch_source();

    assert_eq!(Layout::default().to_glob("toto", &gh_branch_source), vec!["*/toto".to_string()]);
    assert_eq!(Layout::single_dir(0).to_glob("toto", &gh_branch_source), vec!["toto".to_string()]);
    assert_eq!(Layout::single_dir(1).to_glob("toto", &gh_branch_source), vec!["*/toto".to_string()]);
    assert_eq!(Layout::single_dir(2).to_glob("toto", &gh_branch_source), vec!["*/*/toto".to_string()]);
    assert_eq!(Layout::single_dir(3).to_glob("toto", &gh_branch_source), vec!["*/*/*/toto".to_string()]);

    assert_eq!(Layout::with_tp2("a".to_owned()).to_glob("toto",&http_source),
                                    vec!["toto".to_string(), "a".to_string()]);
    assert_eq!(Layout::with_tp2_and_strip("a".to_owned(), 0).to_glob("toto",&http_source),
                                    vec!["toto".to_string(), "a".to_string()]);
    assert_eq!(Layout::with_tp2_and_strip("a".to_owned(), 1).to_glob("toto",&http_source),
                                    vec!["*/toto".to_string(), "*/a".to_string()]);
    assert_eq!(Layout::with_tp2_and_strip("a".to_owned(), 2).to_glob("toto",&http_source),
                                    vec!["*/*/toto".to_string(), "*/*/a".to_string()]);
    assert_eq!(Layout::with_tp2_and_strip("a".to_owned(), 3).to_glob("toto",&http_source),
                                    vec!["*/*/*/toto".to_string(), "*/*/*/a".to_string()]);


    assert_eq!(
        Layout::with_tp2_default().to_glob("toto",&http_source),
        vec!["toto".to_string(), "setup-toto.tp2".to_string()]
    );
    assert_eq!(
        Layout::with_tp2_default_and_strip(1).to_glob("toto",&http_source),
        vec!["*/toto".to_string(), "*/setup-toto.tp2".to_string()]
    );

    let dirs = vec!["a".to_owned(), "b".to_owned()];
    assert_eq!(
        Layout::multi_dir(dirs.clone()).to_glob("toto",&http_source),
        vec!["a".to_string(), "b".to_string()]
    );
    assert_eq!(
        Layout::multi_dir_and_strip(dirs.clone(), 0).to_glob("toto",&http_source),
        vec!["a".to_string(), "b".to_string()]
    );
    assert_eq!(
        Layout::multi_dir_and_strip(dirs.clone(), 1).to_glob("toto",&http_source),
        vec!["*/a".to_string(), "*/b".to_string()]
    );
    assert_eq!(
        Layout::multi_dir_and_strip(dirs.clone(), 2).to_glob("toto",&http_source),
        vec!["*/*/a".to_string(), "*/*/b".to_string()]
    );
    assert_eq!(
        Layout::multi_dir_and_strip(dirs.clone(), 3).to_glob("toto",&http_source),
        vec!["*/*/*/a".to_string(), "*/*/*/b".to_string()]
    );
}

#[test]
fn deserialize_layout_single_dir_missing_strip() {
    let yaml = r#"
    layout_type: single_dir
    "#;
    let layout: Layout = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        layout,
        Layout::default()
    );
}

#[test]
fn deserialize_layout_single_dir_with_strip() {
    let yaml = r#"
    strip_leading: 1
    layout_type: single_dir
    "#;
    let layout: Layout = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        layout,
        Layout::single_dir(1)
    );
}

#[test]
fn deserialize_layout_single_dir_with_tp2_default() {
    let yaml = r#"
    strip_leading: 1
    layout_type: single_dir_plus_tp2
    "#;
    let layout: Layout = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        layout,
        Layout::with_tp2_default_and_strip(1)
    );
}

#[test]
fn deserialize_layout_single_dir_with_tp2() {
    let yaml = r#"
    strip_leading: 1
    layout_type: single_dir_plus_tp2
    tp2: toto.tp2
    "#;
    let layout: Layout = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        layout,
        Layout::with_tp2_and_strip("toto.tp2".to_string(), 1)
    );
}
