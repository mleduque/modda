
use std::path::PathBuf;

use anyhow::{Result, bail};

use crate::canon_path::CanonPath;
use crate::lowercase::LwcString;
use crate::module::location::post_install_action::{PostPopulateAction, SymlinkAction};


pub fn run_post_populate_action(action: &PostPopulateAction, game_dir: &CanonPath, mod_name:&LwcString) -> Result<()> {
    let _ = mod_name; //unused ATM
    match action {
        PostPopulateAction::Symlink(symlink_action) => symlink(symlink_action,game_dir),
    }
}

/// Create symlinks, paths need to be relative to game dir
/// Checks the target exists and the symlink names doesn't exist already
fn symlink(symlink_action: &SymlinkAction, game_dir: &CanonPath) -> Result<()> {
    let target_path = PathBuf::from(&symlink_action.symlink_target);
    if target_path.is_absolute() {
        bail!("post_install action - symlink target can't be absolute");
    }
    let target_path = CanonPath::new(target_path)?;
    if !target_path.path().starts_with(game_dir.path()) {
        bail!("post_install action - symlink target {target_path:?} must be inside game directory {game_dir:?}");
    }
    let target_meta = match target_path.path().metadata() {
        Ok(metadata) => metadata,
        Err(err) => bail!("Couldn't get metadata for {target_path:?}\n  {err:?}"),
    };

    let symlinks= symlink_action.symlinks.iter().map(|symlink|{
        let path = PathBuf::from(&symlink);
        if path.is_absolute() {
            bail!("post_install action - symlink path can't be absolute");
        }
        let path = CanonPath::new(path)?;
        if !path.path().starts_with(game_dir.path()) {
            bail!("post_install action - symlink path {path:?} must be inside game directory {game_dir:?}");
        }
        Ok(path)
    }).collect::<Result<Vec<_>>>()?;
    symlinks.iter().try_for_each(|sym|
        match sym.path().exists() {
            true => bail!("symlink {sym:?} can't be created, a file with that name already exists"),
            false => Ok(()),
        }
    )?;
    symlinks.iter().try_for_each(|symlink| {
        let filetype =  target_meta.file_type();
        if filetype.is_dir() {
            if let Err(err) = symlink::symlink_dir(symlink.path(), target_path.path()) {
                bail!("Couldn't create symlink {symlink:?} to {target_path:?}\n  {err:?}")
            } else {Ok(())}
        } else if filetype.is_file() {
            if let Err(err) = symlink::symlink_file(symlink.path(), target_path.path()) {
                bail!("Couldn't create symlink {symlink:?} to {target_path:?}\n  {err:?}")
            } else{ Ok(())}
        } else {
            bail!("Unsupported file type for symlink target {target_path:?} (allowed: file or dir)")
        }
    })?;
    Ok(())
}

