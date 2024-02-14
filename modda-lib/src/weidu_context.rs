use std::cell::RefCell;
use std::fs::File;
use std::io::BufWriter;

use std::io::Write;

use anyhow::Result;

use crate::args::Install;
use crate::canon_path::CanonPath;
use crate::file_installer::FileInstaller;
use crate::get_module::ModuleDownload;
use crate::settings::Config;


pub struct WeiduContext<'a> {
    pub current: &'a CanonPath,
    pub settings: &'a Config,
    pub opts: &'a Install,
    pub module_downloader: &'a ModuleDownload<'a>,
    pub file_installer: &'a FileInstaller<'a>,
    pub log: RefCell<Option<BufWriter<File>>>
}

impl <'a> WeiduContext<'a> {
    pub fn log(&self, message: &str) -> Result<()> {
        let mut log = self.log.borrow_mut();
        if let Some(ref mut file) = *log {
            let _ = writeln!(file, "{}", message);
            let _ = file.flush()?;
        }
        Ok(())
    }

    pub fn log_bytes(&self, message: &[u8]) -> Result<()> {
        let mut log = self.log.borrow_mut();
        if let Some(ref mut file) = *log {
            let _ = file.write(message)?;
            let _ = file.write(b"\n")?;
            let _ = file.flush()?;
        }
        Ok(())
    }
}
