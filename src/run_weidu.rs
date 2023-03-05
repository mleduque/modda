

use std::io::{BufRead};
use std::process::{Command, Stdio};

use anyhow::{bail, Result};
use log::error;

use crate::args::Install;
use crate::global::Global;
use crate::language::{LanguageOption, LanguageSelection, select_language};
use crate::components::{Component, Components};
use crate::module::weidu_mod::WeiduMod;
use crate::run_result::RunResult;


pub fn run_weidu(tp2: &str, module: &WeiduMod, opts: &Install, global: &Global) -> Result<RunResult> {
    use LanguageSelection::*;
    let language_id = match select_language(tp2, module, &global.lang_preferences) {
        Ok(Selected(id)) => id,
        Ok(NoMatch(list)) if list.is_empty() => 0,
        Ok(NoPrefSet(available))
        | Ok(NoMatch(available)) => handle_no_language_selected(available, module, global)?,
        Err(err) => return Err(err),
    };
    match &module.components {
        Components::None => Ok(RunResult::Dry("Explicitly requested no components to be installed".to_string())),
        Components::Ask => run_weidu_interactive(tp2, module, opts, &global.game_language),
        Components::List(comp) if comp.is_empty() => run_weidu_interactive(tp2, module, opts, &global.game_language),
        Components::List(components) => run_weidu_auto(tp2, module, components, opts, &global.game_language, language_id)
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
                    game_lang: &str, language_id: u32) -> Result<RunResult> {

    let mut command = Command::new("weidu");
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
                            game_lang: &str) -> Result<RunResult> {
    let mut command = Command::new("weidu");
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


pub fn format_run_result(result: &RunResult, module: &WeiduMod) -> Vec<u8> {
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

pub fn run_weidu_list_components(tp2: &str, lang_id: u32) -> Result<Vec<WeiduComponent>> {
    let mut command = Command::new("weidu");
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
