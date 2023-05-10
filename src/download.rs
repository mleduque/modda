
use std::cmp::min;
use std::fs::File;
use std::io::Write;
use std::path::{PathBuf};

use anyhow::{bail, Result};
use futures_util::stream::StreamExt;
use indicatif::{ProgressBar, ProgressStyle, ProgressState};
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

        if let Err(error) = self.download_partial(url, &partial_name, &dest_dir).await {
            bail!("download_partial failed for {} to {:?}\n  {}", url, partial_name, error);
        };

        if let Err(error) = self.rename_partial(&partial_name, &file_name) {
            bail!("rename_partial failed for {:?} to {:?}\n  {}", partial_name, file_name, error);
        };

        Ok(file_name)
    }

    pub async fn download_partial(&self, url: &str, partial_name: &PathBuf, dest_dir: &PathBuf)  -> Result<()> {
        info!("download {} to {:?}", url, dest_dir);
        std::fs::create_dir_all(dest_dir)?;

        let client = reqwest::Client::new();

        let mut partial_file = match File::create(&partial_name) {
            Err(error) => bail!("failed to create file {:?}\n -> {:?}", partial_name, error),
            Ok(file) => file,
        };

        let response = match client.get(url).send().await {
            Ok(response) => response,
            Err(error) => bail!("HTTP download failed\n -> {:?}", error),
        };
        let total_size = response.content_length();


        // Indicatif setup
        let pb = match total_size {
            Some(total_size) => {
                let pb = ProgressBar::new(total_size);
                pb.set_style(ProgressStyle::default_bar()
                    .template("{msg}\n{spinner:.green} [{elapsed_precise}] {percent:>3}% of {total_bytes} {smoothed_eta:>10}")?
                    .progress_chars("#>-")
                    // https://github.com/console-rs/indicatif/issues/394
                    .with_key("smoothed_eta",
                        |s: &ProgressState, w: &mut dyn std::fmt::Write| match (s.pos(), s.len()) {
                            (pos, Some(len)) if pos != 0 =>
                                write!(w, "{:#}",
                                    indicatif::HumanDuration(std::time::Duration::from_millis(
                                        (s.elapsed().as_millis() * (len as u128 - pos as u128) / (pos as u128)) as u64
                                    ))
                                ).unwrap(),
                            _ => write!(w, "-").unwrap(),
                        },
                    )
                );
                pb
            }
            None => {
                let pb = ProgressBar::new_spinner();
                pb.set_style(ProgressStyle::default_bar()
                    .template("{msg}\n{spinner:.green} [{elapsed_precise}]  {bytes}/(unknown size)")?
                );
                pb
            }
        };
        pb.set_message(format!("Downloading {}", url));

        let response = match response.error_for_status() {
            Err(ref error) => bail!("Could not download mod archive at {}\n -> {}", url, error),
            Ok(response) => response,
        };

        let mut stream = response.bytes_stream();
        let mut downloaded: u64 = 0;

        while let Some(item) = stream.next().await {
            let chunk = match item {
                Err(error) => bail!("Error while downloading file\n -> {:?}", error),
                Ok(chunk) => chunk,
            };
            if let Err(error) = partial_file.write(&chunk) {
                bail!("Error while writing to file\n ->{:?}", error);
            }
            if let Some(total_size) = total_size {
                let new = min(downloaded + (chunk.len() as u64), total_size);
                downloaded = new;
                pb.set_position(new);
            } else {
                let new = downloaded + (chunk.len() as u64);
                downloaded = new;
                pb.set_position(new);
            }
        }
        pb.finish_with_message(format!("Download from {} finished", url));
        Ok(())
    }

    pub fn rename_partial(&self, partial_file_name: &PathBuf, final_file_name: &PathBuf) -> Result<()> {
        if let Err(error) = std::fs::rename(partial_file_name, final_file_name.clone()) {
            bail!("Failed to rename partial file {:?} to {:?}\n -> {:?}", partial_file_name, final_file_name, error);
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
