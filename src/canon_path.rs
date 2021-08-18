
use std::{fmt::Debug, path::{Path, PathBuf}};

use anyhow::Result;
use path_absolutize::*;

pub struct CanonPath (PathBuf);
impl CanonPath {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> { Ok(Self((path.as_ref().absolutize()?).into_owned())) }
    pub fn path(&self) -> &Path { &self.0 }
    pub fn join<P: AsRef<Path>>(&self, p: P) -> PathBuf { self.0.join(p) }
    pub fn starts_with<P: AsRef<Path>>(&self, base: P) -> bool { self.0.starts_with(base) }
}

impl AsRef<Path> for CanonPath {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl Debug for CanonPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
