


use std::collections::HashSet;
use std::fs::File;
use std::io::BufReader;

use anyhow::{anyhow, bail, Result};
use lazy_static::lazy_static;
use log::{warn, info};
use regex::{Regex, RegexBuilder};

use crate::bufread_raw::BufReadRaw;
use crate::components::Components;
use crate::lowercase::LwcString;
use crate::module::{WeiduMod, Module};

// doesn't support --quick-log generated logs ATM
// just need to actually look at then and set field as optional and update regexes
pub struct LogRow {
    pub module: String,
    pub lang_index: u32,
    pub component_index: u32,
    pub component_name: String,
}

lazy_static! {
    static ref TP2_REGEX: Regex = RegexBuilder::new(r##"^~(?:.*/)?(?:setup-)?(.*)\.tp2~\s+#([0-9]+)\s+#([0-9]+)\s+//\s+(.*)$"##)
                                        .case_insensitive(true).build().unwrap();
}

pub fn parse_weidu_log(mod_filter: Option<&LwcString>) -> Result<Vec<LogRow>> {
    let weidu_log = match std::fs::File::open("weidu.log") { // TODO: handle case variations
        Err(error) => return Err(
            anyhow!(format!("Could not open weidu.log - {:?}", error)
        )),
        Ok(file) => file,
    };
    let reader = BufReadRaw::<BufReader<File>>::from_file(weidu_log);
    let result: Result<Vec<_>>  = reader.raw_lines().filter_map(|opt_line| {
        let line = match opt_line {
            Err(error) => return Some(Err(anyhow!("Couldn't read weidu.log line - {:?}", error))),
            Ok(line) => line,
        };
        let line = String::from_utf8_lossy(&line);
        match TP2_REGEX.captures(&line) {
            None => {
                // probably header comment
                if !line.starts_with("//") && !line.is_empty() {
                    warn!("potential garbage in weidu.log");
                    info!("line: =>{}", line);
                }
                None
            }
            Some(cap) => {
                let module = cap.get(1).unwrap().as_str().to_owned();
                if let Some(filter) = &mod_filter {
                    if *filter != &module { // no need to unpack the rest
                        return None; // return from filter_map
                    }
                }
                let lang_capture = cap.get(2).unwrap();
                let lang_index = match u32::from_str_radix(lang_capture.as_str(), 10) {
                    Err(_) => return Some(Err(anyhow!("invalid language index `{:?}` in weidu.log line\n>\t{}", lang_capture, line))),
                    Ok(value) => value,
                };
                let component_capture = cap.get(3).unwrap();
                let component_index = match u32::from_str_radix(component_capture.as_str(), 10) {
                    Err(_) => return Some(Err(anyhow!("invalid component index `{:?}` in weidu.log line\n>\t{}", lang_capture, line))),
                    Ok(value) => value,
                };
                let component_name = cap.get(3).unwrap().as_str().to_owned();
                Some(Ok(LogRow {
                    module,
                    lang_index,
                    component_index,
                    component_name,
                }))
            }
        }
    }).collect();
    result
}

pub fn check_install_complete(module: &Module) -> Result<()> {
    match module {
        Module::Mod { weidu_mod } => match check_installed_components(weidu_mod) {
            Err(err) => return Err(err),
            Ok(missing) => if !missing.is_empty() {
                bail!("All requested components for mod {} could not be installed.\nMissing: {:?}", module.get_name(), missing);
            } else { Ok(()) }
        }
        Module::File { file } => Ok(()),
    }
}
pub fn check_installed_components(module: &WeiduMod) -> Result<Vec<u32>> {
    match &module.components {
        Components::None => Ok(vec![]),
        Components::Ask => Ok(vec![]),
        Components::List(components) => {
            let log_rows = match parse_weidu_log(Some(&module.name)) {
                Ok(log_rows) => log_rows,
                Err(err) => bail!("Could not check installed components\n -> {:?}", err),
            };
            let installed = log_rows.iter().map(|row| row.component_index).collect::<HashSet<_>>();
            info!("installed={:?}", installed);

            let missing = components.iter().filter(|component|
                !installed.contains(&component.index())
            )
            .map(|component| component.index())
            .collect::<Vec<_>>();
            Ok(missing)
        }
    }
}
