
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use glob::{ glob_with, MatchOptions};
use tempfile::TempDir;

use crate::download::{download, Cache};
use crate::manifest::{Module, Location, Source, GithubDescriptor};
use crate::settings::Config;

// at some point, I'd like to have a pool of downloads with installations done
// concurrently as soon as modules are there
#[tokio::main]
pub async fn get_module(module: &Module, settings: &Config) -> Result<()> {
    let cache = get_cache(settings)?;
    match &module.location {
        None => bail!("No location provided to retrieve missing module {}", module.name),
        Some(location) => {
            let archive = match retrieve_location(&location, &cache, &module).await {
                Ok(archive) => archive,
                Err(error) => bail!("retrieve archive failed for module {}\n-> {:?}", module.name, error),
            };
            extract_files(&archive, &module.name, location)?;
            patch_module(&archive, &location.patch)?;
            Ok(())
        }
    }
}

fn get_cache(settings: &Config) -> Result<Cache> {
    match &settings.archive_cache {
        None => match tempfile::tempdir() {
            Err(error) => bail!("Couldn't set up archive cache\n -> {:?}", error),
            Ok(dir) => Ok(Cache::Tmp(dir),)
        }
        Some(path) => {
            if let Err(error) = std::fs::create_dir_all(&path) {
                bail!("Could not create destination dir{:?}\n -> {:?}", path, error);
            }
            Ok(Cache::Path(PathBuf::from(&path)))
        }
    }
}

async fn retrieve_location(loc: &Location, cache: &Cache, module: &Module) -> Result<PathBuf> {
    use Source::*;

    let dest = cache.join(loc.source.save_subdir()?);
    let save_name = loc.source.save_name(&module.name)?;
    match &loc.source {
        Http { http, .. } => download(http, &dest, save_name).await,
        Local { path } => Ok(PathBuf::from(path)),
        Github(crate::manifest::Github { github_user, repository, descriptor }) => 
                get_github(github_user, repository, descriptor, &dest, save_name).await,
    }
}

fn extract_files(archive: &Path, module_name:&str, location: &Location) -> Result<()> {
    match archive.extension() {
        Some(ext) =>  match ext.to_str() {
            None => bail!("Couldn't determine archive type for file {:?}", archive),
            Some("zip") | Some("iemod") => extract_zip(archive, module_name, location),
            Some("rar") => extract_rar(archive, module_name, location),
            Some("tgz") => extract_tgz(archive, module_name, location),
            Some("gz") => {
                let stem = archive.file_stem();
                match stem {
                    Some(stem) => {
                        let stem_path = PathBuf::from(stem);
                        let sub_ext = stem_path.extension();
                        match sub_ext {
                            None => bail!("unsupported .gz file for archive {:?}", archive),
                            Some(sub_ext) => match sub_ext.to_str() {
                                Some("tar") => extract_tgz(archive, module_name, location),
                                _ =>  bail!("unsupported .gz file for archive {:?}", archive),
                            }
                        }
                    }
                    None => bail!("unsupported .gz file for archive {:?}", archive)
                }
            }
            Some(_) => bail!("unknown file type for archive {:?}", archive),
        }
        None => bail!("archive file has no extension {:?}", archive),
    }
}

fn extract_zip(archive: &Path, module_name:&str, location: &Location) -> Result<()> {
    let file = match File::open(archive) {
        Ok(file) => file,
        Err(error) => bail!("Could not open archive {:?} - {:?}", archive, error)
    };
    let reader = BufReader::new(file);
    let mut zip_archive = match zip::ZipArchive::new(reader) {
        Ok(archive) => archive,
        Err(error) => bail!("Cold not open zip archive at {:?}\n -> {:?}", archive, error),
    };
    let temp_dir_attempt = tempfile::tempdir();
    let temp_dir = match temp_dir_attempt {
        Ok(ref dir) => dir,
        Err(error) => bail!("Could not create temp dir for archive extraction\n -> {:?}", error),
    };
    if let Err(error) = zip_archive.extract(&temp_dir) {
        bail!("Zip extraction failed for {:?} - {:?}", archive, error);
    }
    if let Err(error) = move_from_temp_dir(&temp_dir, module_name, location) {
        bail!("Failed to copy file for archive {:?} from temp dir to game dir\n -> {:?}", archive, error);
    }
    println!("{:?}", temp_dir_attempt);
    
    Ok(())
}

fn extract_rar(_archive: &Path, _module_name:&str, _location: &Location) -> Result<()> {
    bail!("not implemented yet")
}

fn extract_tgz(archive: &Path, module_name:&str, location: &Location) -> Result<()> {
    let tar_gz = File::open(archive)?;
    let tar = flate2::read::GzDecoder::new(tar_gz);
    let mut tar_archive = tar::Archive::new(tar);

    let temp_dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(error) => bail!("Could not create temp dir for archive extraction\n -> {:?}", error),
    };
    if let Err(error) = tar_archive.unpack(&temp_dir) {
        bail!("Tgz extraction failed for {:?} - {:?}", archive, error);
    }
    
    if let Err(error) = move_from_temp_dir(&temp_dir, module_name, location) {
        bail!("Failed to copy file for archive {:?} from temp dir to game dir\n -> {:?}", archive, error);
    }
    
    Ok(())
}

fn patch_module(_archive: &Path, patch_loc: &Option<Source>) -> Result<()> {
    if let Some(patch_loc) = patch_loc {
        bail!("not implemented yet - patch from source {:?}", patch_loc);
    } else {
        Ok(())
    }
}

fn move_from_temp_dir(temp_dir: &TempDir, module_name:&str, location: &Location) -> Result<()> {
    let mut items = std::collections::HashSet::new();

    let patterns = location.layout.to_glob(&module_name);
    for pattern in patterns {
        let options = MatchOptions {
            case_sensitive: false,
            ..Default::default()
        };
        let batch = temp_dir.as_ref().join(pattern);
        let batch = batch.to_str().unwrap();
        println!("copy files from {:?}", batch);
        let glob_result = glob_with(batch, options)?;
        for path in glob_result {
            match path {
                Ok(path) => { items.insert(path); }
                Err(error) => bail!("Failed to construct list of files to copy\n -> {:?}", error),
            };
        }
    }
    let dest = std::env::current_dir()?;
    let copy_options = fs_extra::dir::CopyOptions {
        copy_inside: true,
        ..Default::default()
    };
    let _result = fs_extra::move_items(&items.iter().collect::<Vec<_>>(), dest, &copy_options)?;
    // this is ne number of moved items ; I don't care
    Ok(())
}

async fn get_github(github_user: &str, repository: &str, descriptor: &GithubDescriptor, 
                    dest: &PathBuf, save_name: PathBuf) -> Result<PathBuf> {
    download(
        &descriptor.get_url(github_user, repository), 
        dest, 
        save_name,
    ).await
}
