


use std::fs::File;
use std::io::BufReader;

use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use regex::{Regex, RegexBuilder};
use crate::bufread_raw::BufReadRaw;
use crate::lowercase::LwcString;
use crate::manifest::Module;

// doesn't support --quick-log generated logs ATM
// just need to actually look at then and set field as optional and update regexes
pub struct LogRow {
    pub module: String,
    pub lang_index: u32,
    pub component_index: u32,
    pub component_name: String,
    pub mod_version: String,
}

lazy_static! {
    static ref TP2_REGEX: Regex = RegexBuilder::new(r##"^~(?:.*/)?(?:setup-)?(.*)\.tp2~\s+#([0-9]+)\s+#([0-9]+)\s+//\s+(.*):(.*)$"##)
                                        .case_insensitive(true).build().unwrap();
}

pub fn parse_weidu_log(mod_filter: Option<LwcString>) -> Result<Vec<LogRow>> {
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
                if !line.starts_with("//") {
                    println!("WARN: potential garbage in weidu.log");
                }
                None
            }
            Some(cap) => {
                let module = cap.get(1).unwrap().as_str().to_owned();
                if let Some(filter) = &mod_filter {
                    if filter != &module { // no need to unpack the rest
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
                let mod_version = cap.get(4).unwrap().as_str().to_owned();
                Some(Ok(LogRow {
                    module,
                    lang_index,
                    component_index,
                    component_name,
                    mod_version,
                }))
            }
        }
    }).collect();
    result
}


lazy_static! {
    static ref WARNING_REGEX: Regex = Regex::new(r##"^INSTALLED WITH WARNINGS\s+(.*)$"##).unwrap();
}
pub fn find_components_with_warning(module: &Module) -> Result<Vec<String>> {
    let filename = format!("setup-{}.debug", module.name);
    let module_debug = match std::fs::File::open(&filename) { // TODO: handle case variations
        Err(error) => return Err(
            anyhow!(format!("Could not open module log file {} - {:?}", filename, error)
        )),
        Ok(file) => file,
    };
    let reader = BufReadRaw::<BufReader<File>>::from_file(module_debug);

    let mut result = vec![];
    for line in reader.raw_lines() {
        match line {
            Ok(line) => {
                let line = String::from_utf8_lossy(&line);
                if let Some(cap) = WARNING_REGEX.captures(&line) {
                    result.push(cap.get(1).unwrap().as_str().to_owned())
                }
            }
            Err(error) => eprintln!("error reading module debug file line - {:?}", error),
        }
    }
    Ok(result)
}
