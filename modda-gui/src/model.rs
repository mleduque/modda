
use std::path::PathBuf;

use modda_lib::module::manifest::Manifest;

#[derive(Clone, Default, PartialEq)]
pub struct GameDir{
    path: Option<PathBuf>
}

impl GameDir {
    pub fn none() -> Self { Self { path: None }}
    pub fn some(path: PathBuf) -> Self { Self { path: Some(path)} }

    pub fn path(&self) -> Option<&PathBuf> {
        self.path.as_ref()
    }
}

#[derive(Clone, Default, PartialEq)]
pub struct ManifestPath {
    pub path: Option<PathBuf>
}
