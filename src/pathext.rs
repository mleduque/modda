
use std::ffi::{OsStr, OsString};
use std::path::{PathBuf};

/// Returns a path with a new dotted extension component appended to the end.
/// Note: does not check if the path is a file or directory; you should do that.
/// # Example
/// ```
/// use pathext::append_extension;
/// use std::path::PathBuf;
/// let path = PathBuf::from("foo/bar/baz.txt");
/// if !path.is_dir() {
///    assert_eq!(append_ext("app", path), PathBuf::from("foo/bar/baz.txt.app"));
/// }
/// ```
///
pub fn append_extension(ext: impl AsRef<OsStr>, path: PathBuf) -> PathBuf {
    let mut os_string: OsString = path.into();
    os_string.push(".");
    os_string.push(ext.as_ref());
    os_string.into()
}
