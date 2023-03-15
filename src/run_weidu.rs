

use std::io::{BufRead};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::{bail, Result};
use lazy_static::lazy_static;
use log::error;
use regex::Regex;

use crate::args::Install;
use crate::global::Global;
use crate::language::{LanguageOption, LanguageSelection, select_language};
use crate::components::{Component, Components};
use crate::module::weidu_mod::WeiduMod;
use crate::run_result::RunResult;
use crate::settings::Config;


pub fn run_weidu(tp2: &str, module: &WeiduMod, opts: &Install, global: &Global, config: &Config) -> Result<RunResult> {
    use LanguageSelection::*;
    let language_id = match select_language(tp2, module, &global.lang_preferences, config) {
        Ok(Selected(id)) => id,
        Ok(NoMatch(list)) if list.is_empty() => 0,
        Ok(NoPrefSet(available))
        | Ok(NoMatch(available)) => handle_no_language_selected(available, module, global)?,
        Err(err) => return Err(err),
    };
    match &module.components {
        Components::None => Ok(RunResult::Dry("Explicitly requested no components to be installed".to_string())),
        Components::Ask => run_weidu_interactive(tp2, module, opts, &global.game_language, config),
        Components::List(comp) if comp.is_empty() => run_weidu_interactive(tp2, module, opts, &global.game_language, config),
        Components::List(components) => run_weidu_auto(tp2, module, components, opts, &global.game_language, language_id, config)
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

fn run_weidu_auto(tp2: &str, module: &WeiduMod, components: &[Component], opts: &Install,
                    game_lang: &str, language_id: u32, config: &Config) -> Result<RunResult> {

    let mut command = Command::new(weidu_command(config, false)?);
    let mut args = vec![
        tp2.to_owned(),
        "--no-exit-pause".to_owned(),
        "--skip-at-view".to_owned(),
        "--log".to_owned(),
        format!("setup-{}.debug", module.name),
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

fn run_weidu_interactive(tp2: &str, module: &WeiduMod, opts: &Install,
                            game_lang: &str, config: &Config) -> Result<RunResult> {
    let mut command = Command::new(weidu_command(config, false)?);
    let args = vec![
        tp2.to_owned(),
        "--no-exit-pause".to_owned(),
        "--skip-at-view".to_owned(),
        "--log".to_owned(),
        format!("setup-{}.debug", module.name),
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


pub fn format_run_result(result: &RunResult, module: &WeiduMod, config: &Config) -> Vec<u8> {
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
    pub index: u32,
    pub number: u32,
    pub forced: bool,
    pub name: String,
    pub subgroup: Option<String>,
    pub group: Vec<String>,
}

pub fn run_weidu_list_components(tp2: &str, lang_id: u32, config: &Config) -> Result<Vec<WeiduComponent>> {
    let mut command = Command::new(weidu_command(config, false)?);
    let args = vec![
        "--list-components-json".to_owned(),
        tp2.to_owned(),
        lang_id.to_string(),
    ];
    command.args(&args);
    let output = command.output()?;
    for (idx, line) in output.stdout.lines().enumerate() {
        let line = match line {
            Err(error) => {
                error!("error reading line {} of weidu stdout - {:?}", idx, error);
                continue;
            }
            Ok(line) => line,
        };
        if !line.starts_with(r##"[{""##) {
            continue;
        }
        let result: Vec<WeiduComponent> = serde_json::from_str(&line)?;
        return Ok(result);
    }
    bail!("weidu json output not parseable")
}


lazy_static! {
    static ref LANGUAGE_REGEX: Regex = Regex::new("^([0-9]*):(.*)$").unwrap();
}

pub fn list_available_languages(tp2: &str, module: &WeiduMod, config: &Config) -> Result<Vec<LanguageOption>> {
    let mut command = Command::new(weidu_command(config, false)?);
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
    let lines = crate::bufread_raw::BufReadRaw::new(&output.stdout[..]).raw_lines();

    let mut lines_ok = vec![];
    for line in lines {
        match line {
            Err(err) => bail!("Couldn't obtain language list for module '{}' [error reading output] _ {:?}",
                            module.name, err),
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

pub fn check_weidu_exe(config: &Config) -> Result<()> {
    let mut command = Command::new(weidu_command(config, false)?);
    command.arg("--help");
    command
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    match command.output() {
        Err(error) => bail!("weidu executable doesn't appear to work\n  {:?}", error),
        _ => Ok(())
    }
}

fn weidu_command(config: &Config, check_exist: bool) -> Result<&str> {
    use crate::progname::PROGNAME;

    match &config.weidu_path {
        Some(path) => {
            if check_exist && !PathBuf::from(path).exists() {
                bail!("file at '{}' doesn't exist. This is the weidu_path config in ${PROGNAME} settings file (${PROGNAME}.yaml)", path)
            }
            Ok(path)
        }
        None => Ok("weidu"),
    }
}
