
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

#[derive(Debug, Clone, PartialEq)]
pub struct GlobDesc {
    pub patterns: Vec<String>,
    pub strip: usize,
}
impl GlobDesc {
    pub fn single(pattern: &str, strip: usize) -> Self { Self { patterns: vec![pattern.to_owned()], strip } }
    pub fn from(patterns: &[&str], strip: usize) -> Self {
        Self {
            patterns: patterns.iter().map(|item| item.to_string()).collect(),
            strip,
        }
    }
    pub fn with(patterns: &[String], strip: usize) -> Self {
        Self {
            patterns: patterns.iter().map(|item| item.to_owned()).collect(),
            strip,
        }
    }
}

impl Layout {
    pub fn to_glob(&self, module_name: &str, location_source: &Source) -> GlobDesc {
        use LayoutContent::*;

        let strip_level = self.strip_level(location_source);
        match &self.layout {
            SingleDir => GlobDesc::single(module_name, strip_level),
            SingleDirPlusTp2 { tp2: Some(tp2) } => GlobDesc::from(&vec![module_name, &tp2], strip_level),
            SingleDirPlusTp2 { tp2: None } => GlobDesc::from(&vec![
                module_name,
                &format!("setup-{}.tp2",module_name)
                ], strip_level),
            MultipleDirs { dirs } => GlobDesc::with(&dirs, strip_level),
        }
    }

    fn strip_level(&self, source: &Source) -> usize {
        match self {
            Layout { strip_leading: None, .. } => source.default_strip_leading(),
            Layout { strip_leading: Some(v), .. } => *v,
        }
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
fn test_to_glob() {
    let http_source = Source::http_source();

    assert_eq!(Layout::default().to_glob("toto", &http_source), GlobDesc::single("toto", 0));
    assert_eq!(Layout::single_dir(0).to_glob("toto", &http_source), GlobDesc::single("toto", 0));
    assert_eq!(Layout::single_dir(1).to_glob("toto", &http_source), GlobDesc::single("toto", 1));
    assert_eq!(Layout::single_dir(2).to_glob("toto", &http_source), GlobDesc::single("toto", 2));
    assert_eq!(Layout::single_dir(3).to_glob("toto", &http_source), GlobDesc::single("toto", 3));

    let gh_release_source = Source::gh_release_source();

    assert_eq!(Layout::default().to_glob("toto", &gh_release_source), GlobDesc::single("toto", 0));
    assert_eq!(Layout::single_dir(0).to_glob("toto", &gh_release_source), GlobDesc::single("toto", 0));
    assert_eq!(Layout::single_dir(1).to_glob("toto", &gh_release_source), GlobDesc::single("toto", 1));
    assert_eq!(Layout::single_dir(2).to_glob("toto", &gh_release_source), GlobDesc::single("toto", 2));
    assert_eq!(Layout::single_dir(3).to_glob("toto", &gh_release_source), GlobDesc::single("toto", 3));

    let gh_branch_source = Source::gh_branch_source();

    assert_eq!(Layout::default().to_glob("toto", &gh_branch_source), GlobDesc::single("toto", 1));
    assert_eq!(Layout::single_dir(0).to_glob("toto", &gh_branch_source), GlobDesc::single("toto", 0));
    assert_eq!(Layout::single_dir(1).to_glob("toto", &gh_branch_source), GlobDesc::single("toto", 1));
    assert_eq!(Layout::single_dir(2).to_glob("toto", &gh_branch_source), GlobDesc::single("toto", 2));
    assert_eq!(Layout::single_dir(3).to_glob("toto", &gh_branch_source), GlobDesc::single("toto", 3));

    assert_eq!(Layout::with_tp2("a".to_owned()).to_glob("toto",&http_source),
                                GlobDesc::from(&["toto", "a"], 0));
    assert_eq!(Layout::with_tp2_and_strip("a".to_owned(), 0).to_glob("toto",&http_source),
                                GlobDesc::from(&["toto", "a"], 0));
    assert_eq!(Layout::with_tp2_and_strip("a".to_owned(), 1).to_glob("toto",&http_source),
                                GlobDesc::from(&["toto", "a"], 1));
    assert_eq!(Layout::with_tp2_and_strip("a".to_owned(), 2).to_glob("toto",&http_source),
                                GlobDesc::from(&["toto", "a"], 2));
    assert_eq!(Layout::with_tp2_and_strip("a".to_owned(), 3).to_glob("toto",&http_source),
                                GlobDesc::from(&["toto", "a"], 3));


    assert_eq!(
        Layout::with_tp2_default().to_glob("toto",&http_source),
        GlobDesc::from(&["toto", "setup-toto.tp2"], 0)
    );
    assert_eq!(
        Layout::with_tp2_default_and_strip(1).to_glob("toto",&http_source),
        GlobDesc::from(&["toto", "setup-toto.tp2"], 1)
    );

    let dirs = vec!["a".to_owned(), "b".to_owned()];
    assert_eq!(
        Layout::multi_dir(dirs.clone()).to_glob("toto",&http_source),
        GlobDesc::from(&["a", "b"], 0)
    );
    assert_eq!(
        Layout::multi_dir_and_strip(dirs.clone(), 0).to_glob("toto",&http_source),
        GlobDesc::from(&["a", "b"], 0)
    );
    assert_eq!(
        Layout::multi_dir_and_strip(dirs.clone(), 1).to_glob("toto",&http_source),
        GlobDesc::from(&["a", "b"], 1)
    );
    assert_eq!(
        Layout::multi_dir_and_strip(dirs.clone(), 2).to_glob("toto",&http_source),
        GlobDesc::from(&["a", "b"], 2)
    );
    assert_eq!(
        Layout::multi_dir_and_strip(dirs.clone(), 3).to_glob("toto",&http_source),
        GlobDesc::from(&["a", "b"], 3)
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

#[test]
fn deserialize_layout_multi_dir() {
    let yaml = r#"
    layout_type: multi_dir
    dirs:
        - a
        - b
    "#;
    let layout: Layout = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        layout,
        Layout::multi_dir(vec!["a".to_string(), "b".to_string()])
    );
}
