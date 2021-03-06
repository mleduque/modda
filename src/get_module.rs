
use std::collections::HashSet;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{bail, Result};
use globwalk::GlobWalkerBuilder;
use log::{debug, info};

use crate::apply_patch::patch_module;
use crate::args::{Install};
use crate::canon_path::CanonPath;
use crate::download::{download, Cache};
use crate::manifest::{GithubDescriptor, Location, Module, PrecopyCommand, Source};
use crate::settings::Config;

// at some point, I'd like to have a pool of downloads with installations done
// concurrently as soon as modules are there
#[tokio::main]
pub async fn get_module(module: &Module, config: &Config, opts: &Install) -> Result<()> {
    let cache = get_cache(config)?;
    match &module.location {
        None => bail!("No location provided to retrieve missing module {}", module.name),
        Some(location) => {
            let archive = match retrieve_location(&location, &cache, &module).await {
                Ok(archive) => archive,
                Err(error) => bail!("retrieve archive failed for module {}\n-> {:?}", module.name, error),
            };

            let dest = std::env::current_dir()?;
            let dest = CanonPath::new(dest)?;
            extract_files(&archive, &dest, &module.name, location, config)?;
            patch_module(&dest, &module.name, &location.patch, opts).await?;
            Ok(())
        }
    }
}

pub fn get_cache(config: &Config) -> Result<Cache> {
    match &config.archive_cache {
        None => match tempfile::tempdir() {
            Err(error) => bail!("Couldn't set up archive cache\n -> {:?}", error),
            Ok(dir) => Ok(Cache::Tmp(dir),)
        }
        Some(path) => {
            let expanded = shellexpand::tilde(path);
            if let Err(error) = std::fs::create_dir_all(&*expanded) {
                bail!("Could not create destination dir{:?}\n -> {:?}", expanded, error);
            }
            Ok(Cache::Path(PathBuf::from(&*expanded)))
        }
    }
}

pub async fn retrieve_location(loc: &Location, cache: &Cache, module: &Module) -> Result<PathBuf> {
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

fn extract_files(archive: &Path, game_dir: &CanonPath, module_name:&str, location: &Location, config: &Config) -> Result<()> {
    debug!("extract_files from archive {:?} for {}", archive, module_name);
    let result = _extract_files(archive, game_dir, module_name, location, config);
    debug!("done extracting files, ended in {}", result.as_ref().map(|_| "success".to_owned()).unwrap_or_else(|_| "failure".to_owned()));
    result
}

fn _extract_files(archive: &Path, game_dir: &CanonPath, module_name:&str, location: &Location, config: &Config) -> Result<()> {
    match archive.extension() {
        Some(ext) =>  match ext.to_str() {
            None => bail!("Couldn't determine archive type for file {:?}", archive),
            Some("zip") | Some("iemod") => extract_zip(archive, game_dir, module_name, location, config),
            Some("rar") => extract_rar(archive, game_dir, module_name, location, config),
            Some("tgz") => extract_tgz(archive, game_dir, module_name, location, config),
            Some("gz") => {
                let stem = archive.file_stem();
                match stem {
                    Some(stem) => {
                        let stem_path = PathBuf::from(stem);
                        let sub_ext = stem_path.extension();
                        match sub_ext {
                            None => bail!("unsupported .gz file for archive {:?}", archive),
                            Some(sub_ext) => match sub_ext.to_str() {
                                Some("tar") => extract_tgz(archive, game_dir, module_name, location, config),
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

fn extract_zip(archive: &Path, game_dir: &CanonPath, module_name:&str, location: &Location, config: &Config) -> Result<()> {
    let file = match File::open(archive) {
        Ok(file) => file,
        Err(error) => bail!("Could not open archive {:?} - {:?}", archive, error)
    };
    let reader = BufReader::new(file);
    let mut zip_archive = match zip::ZipArchive::new(reader) {
        Ok(archive) => archive,
        Err(error) => bail!("Cold not open zip archive at {:?}\n -> {:?}", archive, error),
    };
    let temp_dir_attempt = create_temp_dir(config);
    let temp_dir = match temp_dir_attempt {
        Ok(ref dir) => dir,
        Err(error) => bail!("Extraction of zip mod {} failed\n -> {:?}", module_name, error),
    };
    debug!("zip extraction starting");
    if let Err(error) = zip_archive.extract(&temp_dir) {
        bail!("Zip extraction failed for {:?}\n-> {:?}", archive, error);
    }
    debug!("zip extraction done");
    if let Some(command) = &location.precopy {
        if let Err(error) = run_precopy_command(&temp_dir.as_ref(), command) {
            bail!("Couldn't run precopy command for mod {}\n{}\n{:?}", module_name, command.command, error);
        }
    }
    if let Err(error) = move_from_temp_dir(&temp_dir.as_ref(), game_dir, module_name, location) {
        bail!("Failed to copy file for archive {:?} from temp dir to game dir\n -> {:?}", archive, error);
    }
    debug!("files done moving to final destinatino");

    Ok(())
}

fn create_temp_dir(config: &Config) -> Result<tempfile::TempDir> {
    let temp_dir_attempt = match &config.extract_location {
        None => tempfile::tempdir(),
        Some(location) => {
            let expanded = shellexpand::tilde(location);
            debug!("using {:?} for extraction location", expanded);
            if let Err(error) = std::fs::create_dir_all(&*expanded) {
                bail!("Error creating extraction location from config: {}\n -> {:?}", expanded, error);
            }
            tempfile::tempdir_in(&*expanded)
        }
    };
    match temp_dir_attempt {
        Ok(dir) => Ok(dir),
        Err(error) => bail!("Could not create temp dir for archive extraction\n -> {:?}", error),
    }
}

fn extract_rar(archive: &Path, game_dir: &CanonPath, module_name:&str, location: &Location, config: &Config) -> Result<()> {
    let string_path = match archive.as_os_str().to_str() {
        None => bail!("invalid path for archive {:?}", archive),
        Some(value) => value.to_owned(),
    };
    let rar_archive = unrar::archive::Archive::new(string_path);

    let temp_dir_attempt = create_temp_dir(config);
    let temp_dir = match temp_dir_attempt {
        Ok(dir) => dir,
        Err(error) => bail!("Extraction of rar mod {} failed\n -> {:?}", module_name, error),
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

fn extract_tgz(archive: &Path, game_dir: &CanonPath, module_name:&str, location: &Location, config: &Config) -> Result<()> {
    let tar_gz = File::open(archive)?;
    let tar = flate2::read::GzDecoder::new(tar_gz);
    let mut tar_archive = tar::Archive::new(tar);

    let temp_dir_attempt = create_temp_dir(config);
    let temp_dir = match temp_dir_attempt {
        Ok(dir) => dir,
        Err(error) => bail!("Extraction of tgz mod {} failed\n -> {:?}", module_name, error),
    };
    if let Err(error) = tar_archive.unpack(&temp_dir) {
        bail!("Tgz extraction failed for {:?} - {:?}", archive, error);
    }

    if let Err(error) = move_from_temp_dir(temp_dir.as_ref(), game_dir, module_name, location) {
        bail!("Failed to copy file for archive {:?} from temp dir to game dir\n -> {:?}", archive, error);
    }

    Ok(())
}

fn move_from_temp_dir(temp_dir: &Path, game_dir: &CanonPath, module_name: &str, location: &Location) -> Result<()> {
    let items = match files_to_move(temp_dir, module_name, location) {
        Ok(items) => items,
        Err(error) => bail!("Failed to prepare list of files to move\n -> {:?}", error),
    };
    let copy_options = fs_extra::dir::CopyOptions {
        copy_inside: true,
        ..Default::default()
    };
    let _result = fs_extra::move_items(&items.iter().collect::<Vec<_>>(), game_dir.path(), &copy_options)?;
    // this is ne number of moved items ; I don't care
    Ok(())
}

fn files_to_move(base: &Path, module_name: &str, location:&Location) -> Result<HashSet<PathBuf>> {
    let mut items = HashSet::new();
    debug!("move_from_temp_dir temp dir={:?}", base);

    let glob_descs = location.layout.to_glob(module_name, &location.source);
    if glob_descs.patterns.is_empty() || glob_descs.patterns.iter().all(|entry| entry.trim().is_empty()) {
        bail!("No file patterns to copy from archive for module {}", module_name);
    }
    debug!("Copy files from patterns: {:?}", glob_descs);
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

fn run_precopy_command(from: &Path, precopy: &PrecopyCommand) -> Result<()> {
    info!("Running precommand `{}` with args {:?} from path `{:?}`", precopy.command, precopy.args, from);
    let mut command = Command::new(&precopy.command);
    let workdir = match &precopy.subdir {
        None => from.to_path_buf(),
        Some(subdir) => from.join(subdir),
    };
    command.current_dir(workdir)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    if let Some(args) = &precopy.args {
        command.args(args);
    }
    debug!("command: {:?}", command);
    return match command.status() {
        Ok(status) => {
            if status.success() {
                Ok(())
            } else {
                bail!("precopy command failed with status\n{:?}", status.code())
            }
        }
        Err(error) => bail!("failure running precopy command\n{:?}", error),
    }
}
