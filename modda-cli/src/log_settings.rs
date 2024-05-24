

use log:: LevelFilter;

#[derive(Debug, Clone)]
pub struct LogSettings {
    pub max_level: LevelFilter,
    pub log_var_name: String,
    pub log_var_value: String,
    pub log_style_name: String,
    pub log_style_value: String,
}
