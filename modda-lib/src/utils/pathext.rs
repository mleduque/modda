
use std::ffi::{OsStr, OsString};
use std::path::{PathBuf, Path};

/// Returns a path with a new dotted extension component appended to the end.
/// Note: does not check if the path is a file or directory; you should do that.
///
pub fn append_extension(ext: impl AsRef<OsStr>, path: &Path) -> PathBuf {
    let mut os_string: OsString = path.into();
    os_string.push(".");
    os_string.push(ext.as_ref());
    os_string.into()
}
