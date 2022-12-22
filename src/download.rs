
use std::fs::File;
use std::io::Write;
use std::path::{PathBuf};

use anyhow::{bail, Result};
use futures_util::stream::StreamExt;
use log::{debug, info};


#[cfg_attr(test, faux::create)]
pub struct Downloader {}

#[cfg_attr(test, faux::methods)]
impl Downloader {
    pub fn new() -> Self {
        Downloader {}
    }

    pub async fn download(&self, url: &str, dest_dir: &PathBuf, file_name: PathBuf) -> Result<PathBuf> {
        info!("obtaining {:?}, url is {} (cache={:?})", file_name, url, dest_dir);

        // check if archive exists in the cache
        let file_name = dest_dir.join(file_name);
        if self.target_exists(&file_name) {
            info!("File already downloaded before, reusing");
            return Ok(file_name.to_owned());
        }

        debug!("will be located under: '{:?}'", file_name);

        let partial_name = get_partial_filename(&file_name)?;

        self.download_partial(url, &partial_name, &dest_dir).await?;

        self.rename_partial(&partial_name, &file_name)?;

        Ok(file_name)
    }

    pub async fn download_partial(&self, url: &str, partial_name: &PathBuf, dest_dir: &PathBuf)  -> Result<()> {
        info!("download {} to {:?}", url, dest_dir);
        std::fs::create_dir_all(dest_dir)?;

        let mut partial_file = match File::create(&partial_name) {
            Err(error) => bail!("failed to create file {:?}\n -> {:?}", partial_name, error),
            Ok(file) => file,
        };

        let response = match reqwest::get(url).await {
            Ok(response) => response,
            Err(error) => bail!("HTTP download failed\n -> {:?}", error),
        };

        let response = match response.error_for_status() {
            Err(ref error) => bail!("Could not download mod archive at {}\n -> {}", url, error),
            Ok(response) => response,
        };

        let mut stream = response.bytes_stream();

        while let Some(item) = stream.next().await {
            let chunk = match item {
                Err(error) => bail!("Error while downloading file\n -> {:?}", error),
                Ok(chunk) => chunk,
            };
            if let Err(error) = partial_file.write(&chunk) {
                bail!("Error while writing to file\n ->{:?}", error);
            }
        }
        Ok(())
    }

    pub fn rename_partial(&self, partial_file_name: &PathBuf, final_file_name: &PathBuf) -> Result<()> {
        if let Err(error) = std::fs::rename(final_file_name, final_file_name.clone()) {
            bail!("Failed to rename partial file to {:?}\n -> {:?}", final_file_name, error);
        } else {
            debug!("renamed partial download file to {:?}", final_file_name);
        }
        Ok(())
    }

    pub fn target_exists(&self, file_name: &PathBuf) -> bool {
        file_name.exists()
    }
}

fn get_partial_filename(file_name: &PathBuf) -> Result<PathBuf> {
    let extension = match file_name.extension() {
        None => bail!("file to download {:?} has no extension", file_name),
        Some(ext) => ext,
    };
    let mut partial_name: std::ffi::OsString = file_name.clone().into();
    partial_name.push(".");
    partial_name.push(extension);
    partial_name.push(".partial");

    Ok(PathBuf::from(partial_name))
}
