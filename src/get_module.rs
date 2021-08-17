
use std::collections::HashSet;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use globwalk::GlobWalkerBuilder;

use crate::apply_patch::patch_module;
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

            let dest = std::env::current_dir()?;
            extract_files(&archive, &dest, &module.name, location)?;
            patch_module(&dest, &module.name, &location.patch).await?;
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
            let expanded = shellexpand::tilde(path);
            Ok(Cache::Path(PathBuf::from(&*expanded)))
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

fn extract_files(archive: &Path, game_dir: &Path, module_name:&str, location: &Location) -> Result<()> {
    match archive.extension() {
        Some(ext) =>  match ext.to_str() {
            None => bail!("Couldn't determine archive type for file {:?}", archive),
            Some("zip") | Some("iemod") => extract_zip(archive, game_dir, module_name, location),
            Some("rar") => extract_rar(archive, game_dir, module_name, location),
            Some("tgz") => extract_tgz(archive, game_dir, module_name, location),
            Some("gz") => {
                let stem = archive.file_stem();
                match stem {
                    Some(stem) => {
                        let stem_path = PathBuf::from(stem);
                        let sub_ext = stem_path.extension();
                        match sub_ext {
                            None => bail!("unsupported .gz file for archive {:?}", archive),
                            Some(sub_ext) => match sub_ext.to_str() {
                                Some("tar") => extract_tgz(archive, game_dir, module_name, location),
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

fn extract_zip(archive: &Path, game_dir: &Path, module_name:&str, location: &Location) -> Result<()> {
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
        bail!("Zip extraction failed for {:?}\n-> {:?}", archive, error);
    }
    if let Err(error) = move_from_temp_dir(&temp_dir.as_ref(), game_dir, module_name, location) {
        bail!("Failed to copy file for archive {:?} from temp dir to game dir\n -> {:?}", archive, error);
    }

    Ok(())
}

fn extract_rar(archive: &Path, game_dir: &Path, module_name:&str, location: &Location) -> Result<()> {
    let string_path = match archive.as_os_str().to_str() {
        None => bail!("invalid path for archive {:?}", archive),
        Some(value) => value.to_owned(),
    };
    let rar_archive = unrar::archive::Archive::new(string_path);


    let temp_dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(error) => bail!("Could not create temp dir for archive extraction\n -> {:?}", error),
    };

    let temp_dir_path = temp_dir.path();
    let temp_dir_str = match temp_dir_path.as_os_str().to_str() {
        None => bail!("invalid path for temp dir "),
        Some(ref value) => value.to_string(),
    };
    if let Err(error) = rar_archive.extract_to(temp_dir_str) {
        bail!("RAR extraction failed for {:?} - {:?}", archive, error);
    }
    if let Err(error) = move_from_temp_dir(temp_dir.as_ref(), game_dir, module_name, location) {
        bail!("Failed to copy file for archive {:?} from temp dir to game dir\n -> {:?}", archive, error);
    }
    Ok(())
}

fn extract_tgz(archive: &Path, game_dir: &Path, module_name:&str, location: &Location) -> Result<()> {
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

    if let Err(error) = move_from_temp_dir(temp_dir.as_ref(), game_dir, module_name, location) {
        bail!("Failed to copy file for archive {:?} from temp dir to game dir\n -> {:?}", archive, error);
    }

    Ok(())
}

fn move_from_temp_dir(temp_dir: &Path, game_dir: &Path, module_name: &str, location: &Location) -> Result<()> {
    let items = match files_to_move(temp_dir, module_name, location) {
        Ok(items) => items,
        Err(error) => bail!("Failed to prepare list of files to move\n -> {:?}", error),
    };
    let copy_options = fs_extra::dir::CopyOptions {
        copy_inside: true,
        ..Default::default()
    };
    let _result = fs_extra::move_items(&items.iter().collect::<Vec<_>>(), game_dir, &copy_options)?;
    // this is ne number of moved items ; I don't care
    Ok(())
}

fn files_to_move(base: &Path, module_name: &str, location:&Location) -> Result<HashSet<PathBuf>> {
    let mut items = HashSet::new();
    println!("move_from_temp_dir temp dir={:?}", base);

    let glob_descs = location.layout.to_glob(module_name, &location.source);
    if glob_descs.patterns.is_empty() || glob_descs.patterns.iter().all(|entry| entry.trim().is_empty()) {
        bail!("No file patterns to copy from archive for module {}", module_name);
    }
    println!("Copy files from patterns: {:?}", glob_descs);
    let glob_builder = GlobWalkerBuilder::from_patterns(base, &glob_descs.patterns)
            .case_insensitive(true)
            .min_depth(glob_descs.strip)
            .max_depth(glob_descs.strip + 1);
    let glob = match glob_builder.build() {
        Err(error) => bail!("Could not evaluate patterns {:?}\n -> {:?}", glob_descs, error),
        Ok(glob) => glob,
    };
    for item in glob.into_iter().filter_map(Result::ok) {
        items.insert(item.into_path());
    }
    Ok(items)
}

async fn get_github(github_user: &str, repository: &str, descriptor: &GithubDescriptor,
                    dest: &PathBuf, save_name: PathBuf) -> Result<PathBuf> {
    download(
        &descriptor.get_url(github_user, repository),
        dest,
        save_name,
    ).await
}
