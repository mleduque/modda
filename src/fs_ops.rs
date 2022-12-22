
use std::fs::File;
use std::path::PathBuf;
/**
 * Exists only so I can mock away fs operations.
 */

use std::{fs::rename as std_fs_rename, path::Path};
use std::{fs::create_dir_all as std_fs_create_dir_all};
use std::io;

#[cfg_attr(test, faux::create)]
pub struct Fs {}

#[cfg_attr(test, faux::methods)]
impl Fs {
    //see https://github.com/nrxus/faux/issues/18
    pub fn new() -> Self { Fs {} }

    pub fn rename(&self, from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<()>  {
        std_fs_rename(from, to)
    }

    pub fn create_dir_all(&self, path: impl AsRef<Path>) -> io::Result<()> {
        std_fs_create_dir_all(path)
    }

    pub fn pathbuf_exists(&self, path_buf: &PathBuf) -> bool {
        path_buf.exists()
    }

    pub fn file_create(&self, path: impl AsRef<Path>) -> io::Result<File> {
        File::create(path)
    }
}
