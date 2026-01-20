use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use globwalk::GlobWalkerBuilder;
use log::debug;


pub fn find_insensitive<P,S> (base: P, pattern: S) -> Result<Option<PathBuf>>
    where
        P: AsRef<Path> + std::fmt::Debug,
        S: AsRef<str> + std::fmt::Debug {
    debug!("Looking for file matching pattern {:?} in {:?}", pattern, base);
    let glob_builder = GlobWalkerBuilder::new(&base, &pattern)
        .case_insensitive(true)
        .max_depth(1);
    let glob = match glob_builder.build() {
        Err(error) => bail!("Could not look up files matching {:?} in {:?}\n -> {:?}", pattern, base, error),
        Ok(glob) => glob,
    };
    let candidates = glob.into_iter().filter_map(Result::ok)
        .map(|entry| {
            debug!("Found file matching pattern {:?} (in {:?}): '{}'", pattern, base, entry.file_name().to_string_lossy());
            entry.file_name().to_owned()
        })
        .collect::<Vec<_>>();
    match candidates[..] {
        [] => {
            debug!("Found no matches for {pattern:?} in {base:?}");
            Ok(None)
        },
        [ref name] => Ok(Some(PathBuf::from(name))),
        _ => bail!("More than one candidate for pattern {:?} in {:?}", pattern, base),
    }
}
