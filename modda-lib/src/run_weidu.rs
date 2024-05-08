
use std::process::{Command, Stdio};

use anyhow::{bail, Result};
use chrono::Utc;
use lazy_static::lazy_static;
use log::{debug, info, warn};
use regex::Regex;

use crate::args::{Install, Reset};
use crate::canon_path::CanonPath;
use crate::global::Global;
use crate::modda_context::WeiduContext;
use crate::module::language::{LanguageOption, LanguageSelection, select_language};
use crate::module::components::{Component, Components};
use crate::lowercase::LwcString;
use crate::module::weidu_mod::{WeiduMod, BareMod};
use crate::run_result::RunResult;

#[cfg(target_os="windows")]
const WEIDU_BIN: &str = "weidu.exe";

#[cfg(not(target_os="windows"))]
const WEIDU_BIN: &str = "weidu";

pub fn run_weidu_install(tp2: &str, module: &WeiduMod, opts: &Install, global: &Global,
                        weidu_context: &WeiduContext) -> Result<RunResult> {
    use LanguageSelection::*;
    let language_id = match select_language(tp2, module, &global.lang_preferences, weidu_context) {
        Ok(Selected(id)) => id,
        Ok(NoMatch(list)) if list.is_empty() => 0,
        Ok(NoPrefSet(available))
        | Ok(NoMatch(available)) => handle_no_language_selected(available, module, global)?,
        Err(err) => return Err(err),
    };
    match &module.components {
        Components::None => Ok(RunResult::Dry("Explicitly requested no components to be installed".to_string())),
        Components::Ask =>
                run_weidu_install_interactive(tp2, module, opts, &global.game_language, weidu_context),
        Components::All =>
                run_weidu_install_all(tp2, module, opts, &global.game_language, language_id, weidu_context),
        Components::List(comp) if comp.is_empty() =>
                run_weidu_install_interactive(tp2, module, opts, &global.game_language, weidu_context),
        Components::List(components) =>
                run_weidu_install_auto(tp2, module, components, opts, &global.game_language, language_id, weidu_context),
    }
}

fn handle_no_language_selected(available: Vec<LanguageOption>, module: &WeiduMod, global: &Global) -> Result<u32> {
    // may one day prompt user for selection and (if ok) same in the yaml file
    bail!(
        r#"No matching language found for module {} with language preferences {:?}
        Available choices are {:?}
        "#,
        module.name, &global.lang_preferences, available);
}

fn run_weidu_install_auto(tp2: &str, module: &WeiduMod, components: &[Component], opts: &Install,
                    game_lang: &str, language_id: u32, weidu_context: &WeiduContext) -> Result<RunResult> {

    let mut command = Command::new(weidu_command(weidu_context)?);
    let mut args = vec![
        tp2.to_owned(),
        "--no-exit-pause".to_owned(),
        "--skip-at-view".to_owned(),
        "--log".to_owned(),    // Log output and details to X.
        format!("setup-{}.debug", module.name),
        "--logapp".to_owned(), // Append to log file instead of overwriting it.
        "--use-lang".to_owned(),
        game_lang.to_owned(),
        "--language".to_owned(),
        language_id.to_string(),
    ];
    // component list
    args.push("--force-install-list".to_owned());
    args.extend(components.iter().map(|id| id.index().to_string()));

    command.args(&args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    if opts.dry_run {
        println!("would execute {:?}", command);
        Ok(RunResult::Dry(format!("{:?}", command)))
    } else {
        Ok(RunResult::Real(command.output()?))
    }
}

fn run_weidu_install_interactive(tp2: &str, module: &WeiduMod, opts: &Install,
                            game_lang: &str, weidu_context: &WeiduContext) -> Result<RunResult> {
    let mut command = Command::new(weidu_command(weidu_context)?);
    let args = vec![
        tp2.to_owned(),
        "--no-exit-pause".to_owned(),
        "--skip-at-view".to_owned(),
        "--log".to_owned(),    // Log output and details to X.
        format!("setup-{}.debug", module.name),
        "--logapp".to_owned(), // Append to log file instead of overwriting it.
        "--use-lang".to_owned(),
        game_lang.to_owned(),
    ];
    command.args(&args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    if opts.dry_run {
        println!("would execute {:?}", command);
        Ok(RunResult::Dry(format!("{:?}", command)))
    } else {
        Ok(RunResult::Real(command.output()?))
    }
}

fn run_weidu_install_all(tp2: &str, module: &WeiduMod, opts: &Install,
                    game_lang: &str, language_id: u32, weidu_context: &WeiduContext) -> Result<RunResult> {
    let list = match run_weidu_list_components(tp2, language_id, weidu_context) {
        Err(error) => bail!("Could not get component list for 'All' mod\n{error}"),
        Ok(list) => list,
    };
    let components = list.iter()
        .map(|weidu_comp| Component::Simple(weidu_comp.number))
        .collect::<Vec<_>>();
    run_weidu_install_auto(tp2, module, &components, opts, game_lang, language_id, weidu_context)
}

pub fn format_install_result(result: &RunResult, module: &WeiduMod) -> Vec<u8> {
    return match result {
        RunResult::Real(result) => {
            let summary = format!("\n==\nmodule {} finished with status {:?}\n", module.name, result.status.code()).into_bytes();
            let mut output: Vec<u8> = Vec::with_capacity(result.stdout.len() + result.stderr.len() + summary.len() + 1);
            output.extend(&result.stdout);
            output.push('\n' as u8);
            output.extend(&result.stderr);
            output.extend(summary);
            output
        }
        RunResult::Dry(cmd) => {
            format!("dry-run: {}\n", cmd).into_bytes()
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct WeiduComponent {
    // apparition order in component list
    pub index: u32,
    // component number (as defined by DESIGNATED)
    pub number: u32,
    // Dunno... FORCED_SUBCOMPONENT maybe?
    pub forced: bool,
    // Component name (which appears in weidu.log)
    pub name: String,
    // when the component is one of multiple options GROUP+SUBCOMPONENT - name of the parent group
    pub subgroup: Option<String>,
    // Component grouping by category for ease of installation
    pub group: Vec<String>,
}

pub fn run_weidu_list_components(tp2: &str, lang_id: u32, weidu_context: &WeiduContext) -> Result<Vec<WeiduComponent>> {
    let mut command = Command::new(weidu_command(weidu_context)?);
    let args = vec![
        "--list-components-json".to_owned(),
        tp2.to_owned(),
        lang_id.to_string(),
    ];
    command.args(&args);
    let output = command.output()?;
    let output = String::from_utf8_lossy(&output.stdout);
    for line in output.lines() {
        if !line.starts_with(r##"[{""##) {
            continue;
        }
        let result: Vec<WeiduComponent> = serde_json::from_str(&line)?;
        return Ok(result);
    }
    bail!("weidu json output not parsable")
}


lazy_static! {
    static ref LANGUAGE_REGEX: Regex = Regex::new("^([0-9]*):(.*)$").unwrap();
}

pub fn list_available_languages(tp2: &str, mod_name: &LwcString, weidu_context: &WeiduContext) -> Result<Vec<LanguageOption>> {
    let mut command = Command::new(weidu_command(weidu_context)?);
    let args = vec![
        "--no-exit-pause".to_owned(),
        "--list-languages".to_owned(),
        tp2.to_owned(),
    ];
    command.args(&args);
    let output = command.output()?;

    // the first line show a version string starting with [weidu], then some lines
    // with [<some file name>] ...
    // then n language lines in the form
    // <integer>COLON<string(language name)>
    let lines = crate::utils::bufread_raw::BufReadRaw::new(&output.stdout[..]).raw_lines();

    let mut lines_ok = vec![];
    for line in lines {
        match line {
            Err(err) => bail!("Couldn't obtain language list for module '{}' [error reading output] _ {:?}",
                            mod_name, err),
            Ok(line) => {
                lines_ok.push(line);
            }
        }
    }
    let lines_str = lines_ok.iter().map(|line| String::from_utf8_lossy(line)).collect::<Vec<_>>();
    let entries = lines_str.iter().filter_map(|line| match LANGUAGE_REGEX.captures(line) {
        None => None,
        Some(cap) => {
            match (cap.get(1), cap.get(2)) {
                (Some(index), Some(name)) => match u32::from_str_radix(index.as_str(), 10) {
                    Ok(index) => Some((index, name.as_str().to_owned())),
                    Err(_err) => None
                }
                _  => None
            }
        }
    }).collect::<Vec<_>>();
    Ok(entries.into_iter().map(|(index, name)| LanguageOption { index, name }).collect())
}

pub fn check_weidu_exe(weidu_context: &WeiduContext) -> Result<()> {
    let mut command = Command::new(weidu_command(weidu_context)?);
    command.arg("--help");
    command
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    match command.output() {
        Err(error) => bail!("weidu executable doesn't appear to work\n  {:?}", error),
        _ => Ok(())
    }
}

fn weidu_command(weidu_context: &WeiduContext) -> Result<String> {
    match &weidu_context.config.weidu_path {
        Some(path) => {
            let expanded = match shellexpand::full(path) {
                Err(error) => bail!("Weidu path expansion failed\n  {error}"),
                Ok(expanded) => expanded.to_string(),
            };
            Ok(expanded)
        }
        None => if weidu_context.config.ignore_current_dir_weidu.unwrap_or(false) {
            Ok(WEIDU_BIN.to_string())
        } else {
            match fallback_weidu(weidu_context.current_dir) {
                Some(value) => Ok(value),
                None => Ok(WEIDU_BIN.to_string(),)
            }
        }
    }
}

fn fallback_weidu(game_loc: &CanonPath) -> Option<String> {
    match game_loc.join(WEIDU_BIN) {
        Ok(checked_bin) => {
            let checked_bin = checked_bin.to_path_buf();
            if checked_bin.exists() {
                match checked_bin.as_os_str().to_str() {
                    Some(weidu_path) => {
                        info!("Will use weidu binary in the game directory");
                        Some(weidu_path.to_string())
                    }
                    None => None
                }
            } else {
                None
            }
        }
        Err(error) => {
            warn!("Could not build path to local (game location) weidu binary\n  {error}");
            None
        }
    }
}

pub fn run_weidu_uninstall(tp2: &str, module: &BareMod, opts: &Reset, weidu_context: &WeiduContext) -> Result<()> {
    let now = Utc::now().naive_local().format("%Y-%m-%d_%H:%M:%S");

    let mut command = Command::new(weidu_command(weidu_context)?);
    let mut args = vec![
        tp2.to_owned(),
        "--no-exit-pause".to_owned(),
        "--skip-at-view".to_owned(),
        "--language".to_owned(),
        "0".to_owned(), // no need to select a specific language just to uninstall
        "--log".to_owned(),    // Log output and details to X.
        format!("setup-{}-uninstall-{}.debug", module.name, now),
    ];
    // component list
    args.push("--force-uninstall-list".to_owned());
    args.extend(module.components.iter().map(|id| id.index.to_string()));

    command.args(&args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    debug!("uninstall command:\n{:?}", command);

    if opts.dry_run {
        println!("would execute {:?}", command);
        Ok(())
    } else {
        command.output()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::modda_context::WeiduContext;
    use crate::run_weidu::weidu_command;
    use crate::settings::Config;
    use super::WEIDU_BIN;

    #[test]
    fn weidu_command_bin_present_in_current_dir_ignore_not_set() {
        // prepare game dir with weidu "binary" inside
        let (temp_dir, test_game_dir) = setup_test_game_dir();
        std::fs::OpenOptions::new().write(true).create_new(true)
            .open(test_game_dir.join(WEIDU_BIN).unwrap())
            .expect("Could not create test structure");

        let config = Config {
            weidu_path: None,
            ignore_current_dir_weidu: None,
            ..Default::default()
        };
        let weidu_context = WeiduContext {
            config: &config,
            current_dir: &test_game_dir,
        };

        let temp_dir_path = temp_dir.as_ref();
        let game_dir_path = temp_dir_path.join("game");
        let expected_path_buf = game_dir_path.join(WEIDU_BIN);
        let expected = expected_path_buf.as_os_str().to_str().expect("could not build expected result path");
        assert_eq!(
            weidu_command(&weidu_context).expect("Expected success but got an error"),
            expected
        )
    }

    #[test]
    fn weidu_command_bin_present_in_current_dir_ignore_false() {
        // prepare game dir with weidu "binary" inside
        let (temp_dir, test_game_dir) = setup_test_game_dir();
        std::fs::OpenOptions::new().write(true).create_new(true)
            .open(test_game_dir.join(WEIDU_BIN).unwrap())
            .expect("Could not create test structure");

        let config = Config {
            weidu_path: None,
            ignore_current_dir_weidu: Some(false),
            ..Default::default()
        };
        let weidu_context = WeiduContext {
            config: &config,
            current_dir: &test_game_dir,
        };

        let temp_dir_path = temp_dir.as_ref();
        let game_dir_path = temp_dir_path.join("game");
        let expected_path_buf = game_dir_path.join(WEIDU_BIN);
        let expected = expected_path_buf.as_os_str().to_str().expect("could not build expected result path");
        assert_eq!(
            weidu_command(&weidu_context).expect("Expected success but got an error"),
            expected
        )
    }

    #[test]
    fn weidu_command_bin_present_in_current_dir_ignore_true() {
        // prepare game dir with weidu "binary" inside
        let (_temp_dir, test_game_dir) = setup_test_game_dir();
        std::fs::OpenOptions::new().write(true).create_new(true)
            .open(test_game_dir.join(WEIDU_BIN).unwrap())
            .expect("Could not create test structure");

        let config = Config {
            weidu_path: None,
            ignore_current_dir_weidu: Some(true),
            ..Default::default()
        };
        let weidu_context = WeiduContext {
            config: &config,
            current_dir: &test_game_dir,
        };

        assert_eq!(
            weidu_command(&weidu_context).expect("Expected success but got an error"),
            WEIDU_BIN
        )
    }

    #[test]
    fn weidu_command_bin_not_present_in_current_dir_ignore_false() {
        // prepare game dir with weidu "binary" inside
        let (_temp_dir, test_game_dir) = setup_test_game_dir();

        let config = Config {
            ignore_current_dir_weidu: Some(false),
            ..Default::default()
        };
        let weidu_context = WeiduContext {
            config: &config,
            current_dir: &test_game_dir,
        };

        assert_eq!(
            weidu_command(&weidu_context).expect("Expected success but got an error"),
            WEIDU_BIN
        )
    }

    #[test]
    fn weidu_command_bin_not_present_in_current_dir_ignore_true() {
        // prepare game dir with weidu "binary" inside
        let (_temp_dir, test_game_dir) = setup_test_game_dir();

        let config = Config {
            weidu_path: None,
            ignore_current_dir_weidu: Some(true),
            ..Default::default()
        };
        let weidu_context = WeiduContext {
            config: &config,
            current_dir: &test_game_dir,
        };

        assert_eq!(
            weidu_command(&weidu_context).expect("Expected success but got an error"),
            WEIDU_BIN
        )
    }

    #[test]
    fn weidu_command_weidu_path_set_in_config_bin_not_in_game_loc() {
        // prepare game dir with weidu "binary" inside
        let (_temp_dir, test_game_dir) = setup_test_game_dir();

        let config_value = "a/b/c";
        let config = Config {
            weidu_path: Some(config_value.to_string()),
            ..Default::default()
        };
        let weidu_context = WeiduContext {
            config: &config,
            current_dir: &test_game_dir,
        };

        assert_eq!(
            weidu_command(&weidu_context).expect("Expected success but got an error"),
            config_value
        )
    }

    #[test]
    fn weidu_command_weidu_path_set_in_config_bin_present_in_game_loc() {
        // prepare game dir with weidu "binary" inside
        let (_temp_dir, test_game_dir) = setup_test_game_dir();
        std::fs::OpenOptions::new().write(true).create_new(true)
            .open(test_game_dir.join(WEIDU_BIN).unwrap())
            .expect("Could not create test structure");

        let config_value = "a/b/c";
        let config = Config {
            weidu_path: Some(config_value.to_string()),
            ..Default::default()
        };
        let weidu_context = WeiduContext {
            config: &config,
            current_dir: &test_game_dir,
        };

        assert_eq!(
            weidu_command(&weidu_context).expect("Expected success but got an error"),
            config_value
        )
    }

    #[test]
    fn weidu_command_weidu_path_set_in_config_with_tilda_expansion() {
        // prepare game dir with weidu "binary" inside
        let (_temp_dir, test_game_dir) = setup_test_game_dir();

        let config_value = "~/a/b/c";
        let config = Config {
            weidu_path: Some(config_value.to_string()),
            ..Default::default()
        };
        let weidu_context = WeiduContext {
            config: &config,
            current_dir: &test_game_dir,
        };

        let user_dirs = directories::UserDirs::new().unwrap();
        let home_dir = user_dirs.home_dir();
        let expected = home_dir.join("a/b/c");
        assert_eq!(
            weidu_command(&weidu_context).expect("Expected success but got an error"),
            expected.as_os_str().to_str().unwrap()
        )
    }

    fn setup_test_game_dir() -> (tempfile::TempDir, crate::canon_path::CanonPath) {
        let tempdir = tempfile::tempdir().unwrap();
        let test_game_dir = tempdir.path().join("game");
        std::fs::create_dir_all(&test_game_dir).unwrap();
        std::fs::OpenOptions::new().write(true).create_new(true)
            .open(test_game_dir.join("chitin.key"))
            .expect("Could not create test structure");
        (tempdir, crate::canon_path::CanonPath::new(test_game_dir).unwrap())
    }
}
