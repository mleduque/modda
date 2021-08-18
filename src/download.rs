
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use futures_util::stream::StreamExt;
use log::{debug, info};

#[derive(Debug)]
pub enum Cache {
    Tmp(tempfile::TempDir),
    Path(PathBuf),
}
impl Cache {
    pub fn join<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        match self {
            Cache::Tmp(tmpdir) => tmpdir.path().join(path),
            Cache::Path(base_path) => base_path.join(path),
        }
    }
}

pub async fn download(url: &str, dest: &PathBuf, file_name: PathBuf) -> Result<PathBuf> {
    info!("obtaining {:?}, url is {} (cache={:?})", file_name, url, dest);

    // check if archive exixts in the cache
    let file_name = dest.join(file_name);
    if file_name.exists() {
        info!("File already downloaded before, reusing");
        return Ok(file_name.to_owned());
    }

    info!("download {} to {:?} ({:?})", url, dest, file_name);
    std::fs::create_dir_all(dest)?;
    let response = match reqwest::get(url).await {
        Ok(response) => response,
        Err(error) => bail!("HTTP download failed\n -> {:?}", error),
    };

    let response = match response.error_for_status() {
        Err(ref error) => bail!("Could not download mod archive at {}\n -> {}", url, error),
        Ok(response) => response,
    };
    debug!("will be located under: '{:?}'", file_name);

    let extension = match file_name.extension() {
        None => bail!("download result has no extension for url {}", url),
        Some(ext) => ext,
    };
    let mut partial_name: std::ffi::OsString = file_name.clone().into();
    partial_name.push(".");
    partial_name.push(extension);
    partial_name.push(".partial");
    let partial = PathBuf::from(partial_name);
    let mut dest = match File::create(partial.clone()) {
        Err(error) => bail!("failed to create file {:?}\n -> {:?}", file_name, error),
        Ok(file) => file,
    };

    let mut stream = response.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = match item {
            Err(error) => bail!("Error while downloading file\n -> {:?}", error),
            Ok(chunk) => chunk,
        };
        if let Err(error) = dest.write(&chunk) {
            bail!("Error while writing to file\n ->{:?}", error);
        }
    }
    if let Err(error) = std::fs::rename(partial, file_name.clone()) {
        bail!("Failed to rename partial file to {:?}\n -> {:?}", file_name, error);
    } else {
        debug!("renamed partial download file to {:?}", file_name);
    }
    Ok(file_name)
}
