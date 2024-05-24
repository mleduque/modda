use std::path::PathBuf;

use anyhow::Result;
use handlebars::Handlebars;
use serde_json::json;

use modda_lib::args::Introspect;
use modda_lib::canon_path::CanonPath;
use modda_lib::config::{ConfigSource, Settings, ARCHIVE_CACHE_ENV_VAR, EXTRACT_LOCATION_ENV_VAR, IGNORE_CURRENT_DIR_WEIDU_ENV_VAR, WEIDU_PATH_ENV_VAR};

use crate::log_settings::LogSettings;


static TEMPLATE : &'static str =
r#"
Global config directory: {{global_config_dir}}
Found global config file: {{global_config_file}}

Game dir: {{game_dir}}
Found local config file: {{local_config_file}}

Config options from environment variables:
{{environment}}

Concrete config:
{{concrete_config}}

Log settings:
max level: {{max_level}}
{{log_var_name}}="{{log_var_value}}"
{{log_style_name}}="{{log_style_value}}"
"#;

pub fn introspect(params:&Introspect, settings: &Settings, game_dir: &CanonPath,
                    global_conf_dir: &Option<PathBuf>, log_settings: &LogSettings) -> Result<()> {
    let registry = Handlebars::new();

    let context = &json!({
        "game_dir": game_dir.to_path_buf().as_os_str().to_string_lossy(),
        "global_config_dir": match global_conf_dir {
            None => "undetermined".to_string(),
            Some(value) => value.as_os_str().to_string_lossy().to_string(),
        },
        "global_config_file": match &settings.global {
            None => "no".to_string(),
            Some(file_name) => file_name.id.clone(),
        },
        "local_config_file": match &settings.local {
            None => "no".to_string(),
            Some(file_name) => file_name.id.clone(),
        },
        "environment": display_environment(&settings.env_config)
            .iter()
            .map(|(key, value)| format!(r#"{key} = "{value}""#))
            .collect::<Vec<_>>()
            .join("\n"),
        "concrete_config": match serde_yaml::to_string(&settings.combined) {
            Ok(value) => value,
            Err(error) => format!("[error, could not serialize configuration:\n{error}]")
        },
        "max_level": log_settings.max_level.to_string(),
        "log_var_name": log_settings.log_var_name,
        "log_var_value": log_settings.log_var_value,
        "log_style_name": log_settings.log_style_name,
        "log_style_value": log_settings.log_style_value,
    });
    println!("{}", registry.render_template(TEMPLATE, context)?);
    Ok(())
}

const TRUE: &'static str = "true";
const FALSE: &'static str = "false";

fn display_environment(env_config: &ConfigSource) -> Vec<(String, String)> {
    vec![
        (ARCHIVE_CACHE_ENV_VAR, &env_config.config.as_ref().and_then(|value| value.archive_cache.to_owned())),
        (EXTRACT_LOCATION_ENV_VAR, &env_config.config.as_ref().and_then(|value| value.extract_location.to_owned())),
        (WEIDU_PATH_ENV_VAR, &env_config.config.as_ref().and_then(|value| value.weidu_path.to_owned())),
        (IGNORE_CURRENT_DIR_WEIDU_ENV_VAR, &env_config.config.as_ref()
            .and_then(|value|
                value.ignore_current_dir_weidu.map(|value|
                    (if value { TRUE } else { FALSE }).to_owned()
                )
            )
        ),
    ].into_iter()
    .filter_map(|(key, value)|
        match value {
            None => None,
            Some(s) if s.trim() == "" => None,
            Some(value) => Some((key.to_owned(), value.to_owned()))
        }
    ).collect()
}
