
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use futures_util::stream::StreamExt;

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
    println!("download {} to {:?} ({:?})", url, dest, file_name);
    std::fs::create_dir_all(dest)?;
    let response = match reqwest::get(url).await {
        Ok(response) => response,
        Err(error) => bail!("HTTP download failed\n -> {:?}", error),
    };

    println!("file to download: '{:?}'", file_name);
    let file_name = dest.join(file_name);
    if file_name.exists() {
        println!("File already downloaded before, reusing");
        return Ok(file_name.to_owned());
    }
    println!("will be located under: '{:?}'", file_name);

    let mut dest = match File::create(file_name.clone()) {
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
    Ok(file_name.to_owned())
}
