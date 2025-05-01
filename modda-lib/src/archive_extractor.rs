use std::borrow::Cow;
use std::path::PathBuf;
use std::process::{Stdio, Command};
use std::{path::Path, collections::HashSet};

use std::fs::File;
use std::io::{BufReader, self};

use globwalk::GlobWalkerBuilder;
use log::{debug, info};
use anyhow::{bail, Result, anyhow};
use tempfile::TempDir;
use zip::ZipArchive;
use zip::result::{ZipResult, ZipError};

use crate::canon_path::CanonPath;
use crate::lowercase::{LwcString, lwc};
use crate::module::location::location::ConcreteLocation;
use crate::module::pre_copy_command::PrecopyCommand;
use crate::config::{Config, ExtractorCommand};


#[cfg_attr(test, faux::create)]
pub struct Extractor<'a> {
    game_dir: &'a CanonPath,
    config: &'a Config,
}

#[cfg_attr(test, faux::methods)]
impl <'a> Extractor<'a> {

    pub fn new(game_dir: &'a CanonPath, config: &'a Config) -> Self {
        Self {
            game_dir,
            config,
        }
    }

    pub fn extract_files(&self, archive: &Path, module_name: &LwcString, location: &ConcreteLocation,) -> Result<()> {
        debug!("extract_files from archive {:?} for {}", archive, module_name);
        let result = self.extract_files_to_temp(archive, module_name, location);
        debug!("done extracting files, ended in {}", result.as_ref().map(|_| "success".to_owned()).unwrap_or_else(|_| "failure".to_owned()));

        let temp_dir = result?;
        if let Some(command) = &location.precopy {
            if let Err(error) = self.run_precopy_command(&temp_dir.as_path_buf(), command) {
                bail!("Couldn't run precopy command for mod {}, command={} with args {:?}\n{:?}",
                        module_name, command.command, command.args, error);
            }
        }

        self.move_content_to_game_dir(&temp_dir, module_name, location)?;

        Ok(())
    }

    fn move_content_to_game_dir(&self, temp_dir: &ExtractLocation,  module_name: &LwcString, location: &ConcreteLocation) -> Result<()> {
        match temp_dir {
            ExtractLocation::Temp(temp_dir) => {
                debug!("Moving mod content to game location ...");
                if let Err(error) = self.move_from_temp_dir(&temp_dir.as_ref(), module_name, location) {
                    bail!("Failed to move files for mod {} from temp dir to game dir\n -> {:?}", module_name, error);
                }
                debug!("files done moving to final destination");

            }
            ExtractLocation::Regular(source) => {
                debug!("Copying mod content to game location ...");
                if let Err(error) = self.copy_to_game_dir(source) {
                    bail!("Failed to copy files for mod {} from source dir to game dir\n -> {:?}", module_name, error);
                }
                debug!("files done copying to final destination");
            }
        }
        Ok(())
    }

    /// Extracts (if needed) the archive to a temporary location.
    /// Returns the path to the extracted content.
    fn extract_files_to_temp(&self, archive: &Path, module_name: &LwcString, location: &ConcreteLocation) -> Result<ExtractLocation> {
        debug!("extracting (?) file archive {:?} to temporary location.", archive);
        if !archive.exists() {
            bail!("archive for '{module_name}' was not found ({:?}", archive);
        }
        if archive.is_dir() {
            if location.precopy.is_some() {
                info!("archive is a directory so this is a simple copy (required because precopy is defined)");
                // precopy could modify the content so make a temp copy to preserve original
                let temp_dir_attempt = self.create_temp_dir();
                let temp_dir = match temp_dir_attempt {
                    Ok(dir) => dir,
                    Err(error) => bail!("Creation of temp copy of mod {} failed\n -> {:?}", module_name, error),
                };
                self.copy_to_temp_dir(archive, temp_dir.as_ref())?;
                debug!("Directory content was copied to {:?} for precopy command", temp_dir);
                Ok(ExtractLocation::Temp(temp_dir))
            } else {
                debug!("... actually no, this is a directory and there is no precopy so not needed");
                // will not change the source directory, no need to create a temporary copy
                Ok(ExtractLocation::Regular(archive.to_owned()))
            }
        } else {
            let tmp_dir = match archive.extension() {
                Some(ext) =>  match ext.to_str() {
                    None => bail!("Couldn't determine archive type for file {:?}", archive),
                    Some("zip") | Some("iemod") => self.extract_zip(archive, module_name),
                    Some("tgz") => self.extract_tgz(archive, module_name),
                    Some("gz") => self.extract_gz(archive, module_name),
                    Some(ext) => self.extract_external(archive, module_name, ext),
                }
                None => bail!("archive file has no extension {:?}", archive),
            };
            tmp_dir.map(|dir| ExtractLocation::Temp(dir))
        }
    }

    fn extract_gz(&self, archive: &Path, module_name: &LwcString) -> Result<TempDir> {
        let stem = archive.file_stem();
        match stem {
            Some(stem) => {
                let stem_path = PathBuf::from(stem);
                let sub_ext = stem_path.extension();
                match sub_ext {
                    None => bail!("unsupported .gz file for archive {:?}", archive),
                    Some(sub_ext) => match sub_ext.to_str() {
                        Some("tar") => self.extract_tgz(archive, module_name),
                        _ =>  bail!("unsupported .gz file for archive {:?}", archive),
                    }
                }
            }
            None => bail!("unsupported .gz file for archive {:?}", archive)
        }
    }

    fn extract_zip(&self, archive: &Path,  module_name: &LwcString) -> Result<TempDir> {
        let file = match File::open(archive) {
            Ok(file) => file,
            Err(error) => bail!("Could not open archive {:?} - {:?}", archive, error)
        };
        let reader = BufReader::new(file);
        let mut zip_archive = match zip::ZipArchive::new(reader) {
            Ok(archive) => archive,
            Err(error) => bail!("Cold not open zip archive at {:?}\n -> {:?}", archive, error),
        };
        let temp_dir_attempt = self.create_temp_dir();
        let temp_dir = match temp_dir_attempt {
            Ok(dir) => dir,
            Err(error) => bail!("Extraction of zip mod {} failed\n -> {:?}", module_name, error),
        };
        debug!("zip extraction starting");
        if let Err(error) = extract_zip_archive(&mut zip_archive, &temp_dir) {
            bail!("Zip extraction failed for {:?}\n-> {:?}", archive, error);
        }
        debug!("zip extraction done");

        Ok(temp_dir)
    }

    fn extract_tgz(&self, archive: &Path, module_name: &LwcString) -> Result<TempDir> {
        let tar_gz = File::open(archive)?;
        let tar = flate2::read::GzDecoder::new(tar_gz);
        let mut tar_archive = tar::Archive::new(tar);

        let temp_dir_attempt = self.create_temp_dir();
        let temp_dir = match temp_dir_attempt {
            Ok(dir) => dir,
            Err(error) => bail!("Extraction of tgz mod {} failed\n -> {:?}", module_name, error),
        };
        if let Err(error) = tar_archive.unpack(&temp_dir) {
            bail!("Tgz extraction failed for {:?} - {:?}", archive, error);
        }

        Ok(temp_dir)
    }

    fn extract_external(&self, archive: &Path, module_name: &LwcString, extension: &str) -> Result<TempDir> {
        let temp_dir_attempt = self.create_temp_dir();
        let temp_dir = match temp_dir_attempt {
            Ok(dir) => dir,
            Err(error) => bail!("Extraction of '{}' mod {} failed\n -> {:?}", extension, module_name, error),
        };

        if let Err(error) = self.external_extractor_tool(archive, extension, &temp_dir) {
            bail!("Extraction with external tool failed for {:?} - {:?}", archive, error);
        }

        Ok(temp_dir)
    }

    fn create_temp_dir(&self) -> Result<tempfile::TempDir> {
        let temp_dir_attempt = match &self.config.extract_location {
            None => tempfile::tempdir(),
            Some(location) => {
                let expanded = match shellexpand::full(location) {
                    Err(error) => bail!("Temporary dir expansion failed\n  {error}"),
                    Ok(expanded) => expanded.to_string(),
                };
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

    fn copy_to_temp_dir(&self, source: &Path,  temp_dir: &Path) -> Result<()> {
        let copy_options = fs_extra::dir::CopyOptions {
            copy_inside: true,
            content_only: true,
            ..Default::default()
        };
        if let Err(error) = fs_extra::dir::copy(source, temp_dir, &copy_options) {
            bail!("Could not copy dir source to temp location - {:?} to {:?}\n  {}", source, temp_dir, error);
        } else {
            Ok(())
        }
    }

    fn copy_to_game_dir(&self, source: &Path) -> Result<()> {
        let copy_options = fs_extra::dir::CopyOptions {
            copy_inside: true,
            content_only: true,
            ..Default::default()
        };
        if let Err(error) = fs_extra::dir::copy(source, &self.game_dir.path(), &copy_options) {
            bail!("Could not copy dir source to game location - {:?} to {:?}\n  {}", source, &self.game_dir.path(), error);
        } else {
            Ok(())
        }
    }

    fn move_from_temp_dir(&self, temp_dir: &Path, module_name: &LwcString, location: &ConcreteLocation) -> Result<()> {
        let items = match self.files_to_move(temp_dir, module_name, location) {
            Ok(items) => items,
            Err(error) => bail!("Failed to prepare list of files to move\n -> {:?}", error),
        };
        let copy_options = fs_extra::dir::CopyOptions {
            copy_inside: true,
            ..Default::default()
        };
        let _result = fs_extra::move_items(&items.iter().collect::<Vec<_>>(), &self.game_dir.path(), &copy_options)?;
        // this is ne number of moved items ; I don't care
        Ok(())
    }

    fn files_to_move(&self, base: &Path, module_name: &LwcString, location:&ConcreteLocation) -> Result<HashSet<PathBuf>> {
        let mut items = HashSet::new();
        debug!("files_to_move temp dir={:?}", base);

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

    fn run_precopy_command(&self, from: &Path, precopy: &PrecopyCommand) -> Result<()> {
        info!("Running precopy command `{}` with args {:?} from path `{:?}` in subdir {:?}",
                precopy.command, precopy.args, from, precopy.subdir);
        let mut command = Command::new(&precopy.command);
        let work_dir = match &precopy.subdir {
            None => from.to_path_buf(),
            Some(subdir) => from.join(subdir),
        };
        command.current_dir(work_dir)
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

    fn external_extractor_tool(&self, archive: &Path, extension: &str,  tmp_dir: &TempDir) -> Result<()> {
        let extractor_command = self.extractor_command(extension)?;
        let mut command = Command::new(&extractor_command.command);
        let args = extractor_command.args.iter().map(|arg| {
            match arg.as_str() {
                s if s.contains("${input}") => {
                    match archive.as_os_str().to_str().ok_or(anyhow!("Error extracting archive path")) {
                        Err(error) => Err(error),
                        Ok(input) => Ok(s.replace("${input}", input)),
                    }
                }
                s if s.contains("${target}") => {
                    match tmp_dir.as_ref().as_os_str().to_str().ok_or(anyhow!("Error extracting target path")) {
                        Err(error) => Err(error),
                        Ok(target) => Ok(s.replace("${target}", target)),
                    }
                }
                other => Ok(other.to_string()),
            }
        }).collect::<Vec<_>>();
        let (successes, failures): (Vec<_>, Vec<_>) = args.into_iter().partition(|entry| entry.is_ok());
        let failures = failures.iter().map(|entry| format!("{}", entry.as_ref().unwrap_err())).collect::<Vec<_>>();

        if !failures.is_empty() {
            bail!("Could not prepare external extraction command\n  {}", failures.join("\n  "));
        }

        let args = successes.iter().map(|entry| entry.as_ref().unwrap());
        info!("execute {:?}", args);
        command.args(args)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        command.output()?;
        Ok(())
    }

    fn extractor_command(&self, extension: &str) -> Result<&ExtractorCommand> {
        match self.config.extractors.get(&lwc!(extension)) {
            Some(extractor) => Ok(extractor),
            None => bail!("No extractor configured for {extension}"),
        }
    }
}

// duplicated from zip-rs source
fn extract_zip_archive<P: AsRef<Path>>(zip_archive: &mut ZipArchive<BufReader<File>>, directory: P) -> ZipResult<()> {
    use std::fs;

    for i in 0..zip_archive.len() {
        let mut file = zip_archive.by_index(i)?;
        let filepath = file
            .enclosed_name()
            .ok_or(ZipError::InvalidArchive(Cow::Owned("Invalid file path".to_string())))?;

        let outpath = directory.as_ref().join(filepath);

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p)?;
                }
            }
            let mut outfile = fs::File::create(&outpath)?;
            io::copy(&mut file, &mut outfile)?;
        }
    }
    Ok(())
}

enum ExtractLocation {
    Temp(TempDir),
    Regular(PathBuf),
}

impl ExtractLocation {
    pub fn as_path_buf(&self) -> PathBuf {
        match self {
            ExtractLocation::Temp(temp_dir) => temp_dir.path().to_owned(),
            ExtractLocation::Regular(path_buf) => path_buf.to_owned()
        }
    }
}
