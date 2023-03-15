use std::path::PathBuf;
use std::process::{Stdio, Command};
use std::{path::Path, collections::HashSet};

use std::fs::File;
use std::io::BufReader;

use globwalk::GlobWalkerBuilder;
use log::{debug, info};
use anyhow::{bail, Result};

use crate::canon_path::CanonPath;
use crate::location::Location;
use crate::lowercase::LwcString;
use crate::module::pre_copy_command::PrecopyCommand;
use crate::settings::Config;


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

    pub fn extract_files(&self, archive: &Path, module_name: &LwcString, location: &Location,) -> Result<()> {
        debug!("extract_files from archive {:?} for {}", archive, module_name);
        let result = self._extract_files(archive, module_name, location);
        debug!("done extracting files, ended in {}", result.as_ref().map(|_| "success".to_owned()).unwrap_or_else(|_| "failure".to_owned()));
        result
    }

    fn _extract_files(&self, archive: &Path, module_name: &LwcString, location: &Location) -> Result<()> {
        match archive.extension() {
            Some(ext) =>  match ext.to_str() {
                None => bail!("Couldn't determine archive type for file {:?}", archive),
                Some("zip") | Some("iemod") => self.extract_zip(archive, module_name, location),
                Some("tgz") => self.extract_tgz(archive, module_name, location),
                Some("gz") => {
                    let stem = archive.file_stem();
                    match stem {
                        Some(stem) => {
                            let stem_path = PathBuf::from(stem);
                            let sub_ext = stem_path.extension();
                            match sub_ext {
                                None => bail!("unsupported .gz file for archive {:?}", archive),
                                Some(sub_ext) => match sub_ext.to_str() {
                                    Some("tar") => self.extract_tgz(archive, module_name, location),
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

    fn extract_zip(&self, archive: &Path,  module_name: &LwcString, location: &Location) -> Result<()> {
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
            Ok(ref dir) => dir,
            Err(error) => bail!("Extraction of zip mod {} failed\n -> {:?}", module_name, error),
        };
        debug!("zip extraction starting");
        if let Err(error) = zip_archive.extract(&temp_dir) {
            bail!("Zip extraction failed for {:?}\n-> {:?}", archive, error);
        }
        debug!("zip extraction done");
        if let Some(command) = &location.precopy {
            if let Err(error) = self.run_precopy_command(&temp_dir.as_ref(), command) {
                bail!("Couldn't run precopy command for mod {}\n{}\n{:?}", module_name, command.command, error);
            }
        }
        if let Err(error) = self.move_from_temp_dir(&temp_dir.as_ref(), module_name, location) {
            bail!("Failed to copy file for archive {:?} from temp dir to game dir\n -> {:?}", archive, error);
        }
        debug!("files done moving to final destinatino");

        Ok(())
    }

    fn extract_tgz(&self, archive: &Path, module_name: &LwcString, location: &Location) -> Result<()> {
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

        if let Err(error) = self.move_from_temp_dir(temp_dir.as_ref(), module_name, location) {
            bail!("Failed to copy file for archive {:?} from temp dir to game dir\n -> {:?}", archive, error);
        }

        Ok(())
    }

    fn create_temp_dir(&self) -> Result<tempfile::TempDir> {
        let temp_dir_attempt = match &self.config.extract_location {
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


    fn move_from_temp_dir(&self, temp_dir: &Path, module_name: &LwcString, location: &Location) -> Result<()> {
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

    fn files_to_move(&self, base: &Path, module_name: &LwcString, location:&Location) -> Result<HashSet<PathBuf>> {
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

    fn run_precopy_command(&self, from: &Path, precopy: &PrecopyCommand) -> Result<()> {
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

}
