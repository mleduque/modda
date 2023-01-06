

use anyhow::{Result};

use crate::{args::Search, manifest::Manifest};


pub fn search(opts: &Search) -> Result<()> {
    let manifest = Manifest::read_path(&opts.manifest_path)?;
    let mut found = false;
    for (idx, module) in manifest.modules.iter().enumerate() {
        if module.get_name() == &opts.name.to_lowercase() {
            found = true;
            println!("idx: '{} - {}\n\t{:?}", idx, module.describe(), module);
        }
    }

    if !found {
        println!("module {} not found", opts.name);
    }
    Ok(())
}
