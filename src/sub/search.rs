

use anyhow::{Result};

use crate::args::Search;

use crate::manifest::{read_manifest};

pub fn search(opts: &Search) -> Result<()> {
    let manifest = read_manifest(&opts.manifest_path)?;
    let mut found = false;
    for (idx, module) in manifest.modules.iter().enumerate() {
        if module.name.to_lowercase() == opts.name.to_lowercase() {
            found = true;
            println!("idx: '{} - {}\n\t{:?}", idx, module.describe(), module);
        }
    }

    if !found {
        println!("module {} not found", opts.name);
    }
    Ok(())
}
