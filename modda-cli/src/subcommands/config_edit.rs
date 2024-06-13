
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::io::{Result as IoResult, Write};

use anyhow::{bail, Result};
use handlebars::Handlebars;
use modda_lib::config::{global_conf_dir, Config, Settings};
use modda_lib::progname::PROGNAME;

const CONFIG_TEMPLATE: &'static str = include_str!("config_template.yml");

pub fn edit_global_config_dir(config: &Config) -> Result<()> {
    let directory = global_conf_dir()
        .expect("Could not determine the global config location");
    if !directory.exists() {
        println!("Global config directory doesn't exist, creating {dir}",
                dir = directory.as_os_str().to_string_lossy());
        if let Err(error) = std::fs::create_dir_all(&directory) {
            bail!("Could not create global config directory {dir}\n  {error}",
                    dir = directory.as_os_str().to_string_lossy())
        }
    }
    let config_path = match Settings::find_config_in_dir(&directory)? {
        None => {
            println!("Global config file doesn't exist, creating in {dir}",
                    dir = directory.as_os_str().to_string_lossy());
            create_config_yml(&directory)?
        }
        Some(path) => path,
    };
    println!("Opening global config file {config_file_path}",
            config_file_path = config_path.as_os_str().to_string_lossy());
    match &config.code_editor {
        Some(editor) => match open::with_detached(config_path, editor) {
            IoResult::Ok(_) => Ok(()),
            IoResult::Err(error) => bail!("Could not start editor program {editor}\n  {error}")
        }
        None => match open::that_detached(config_path) {
            IoResult::Ok(_) => Ok(()),
            IoResult::Err(error) => bail!("Could not start system editor\n  {error}"),
        }
    }
}

fn create_config_yml(directory: &PathBuf) -> Result<PathBuf> {
    let registry = Handlebars::new();
    let config_content = registry.render_template(CONFIG_TEMPLATE, &())?;

    let config_path = directory.join(format!("{}.yml", PROGNAME));

    let mut dest = match OpenOptions::new().create(true).write(true).open(&config_path) {
        Err(err) => bail!("Could not create config file\n  {}", err),
        std::io::Result::Ok(file) => file,
    };
    dest.write_all(config_content.as_bytes())?;
    Ok(config_path)
}
