
use std::io::{BufReader, BufWriter};
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::Stdio;

use ansi_term::Colour::{Green, Red, Yellow};
use anyhow::{anyhow, bail};
use anyhow::Result;
use clap::{AppSettings, Clap};
use glob::{glob_with, MatchOptions};
use serde::{Deserialize};

#[derive(Clap, Debug)]
#[clap(version = "1.0")]
#[clap(setting = AppSettings::ColoredHelp)]
enum Opts {
    Install(Install),
    Search(Search),
}

#[derive(Clap, Debug)]
struct Install {

    #[clap(long, short)]
    manifest_path: String,
    
    #[clap(long)]
    no_stop_on_warn: bool,

    /// index in the module list where we start (zero-based)
    #[clap(long, short = 'f')]
    from_index: Option<usize>,

    /// index in the module list where we stop
    #[clap(long, short = 't')]
    to_index: Option<usize>,

    #[clap(long, short = 'o')]
    output: Option<String>,


    #[clap(long)]
    dry_run: bool,
}

#[derive(Clap, Debug)]
struct Search {

    #[clap(long, short)]
    manifest_path: String,


    #[clap(long, short)]
    name: String,
}

#[derive(Deserialize, Debug)]
struct Module {
    name: String,
    /// Which language index to use (has precedence over manifest-level lang_prefs)
    language: Option<u32>,
    /// List of components to be auto-installed. In None or empty list, run interactively
    components: Option<Vec<u32>>,
    #[serde(default)]
    ignore_warnings: bool,
    add_conf: Option<ModuleConf>,
}

#[derive(Deserialize, Debug)]
struct Manifest {
    #[serde(rename = "lang_dir")]
    game_language: String,
    /// List of language _names_ that should be selected if available, in decreasing order of priority
    /// items in the list are used as regexp (case insensitive by default)
    /// - the simplest case is just putting the expected language names 
    ///   ex. [français, french, english]
    /// - items in the list that start with `#rx#`are interpreted as regexes
    ///   syntax here https://docs.rs/regex/1.5.4/regex/#syntax
    ///   ex. ["#rx#^fran[cç]ais", french, english]
    lang_preferences: Option<Vec<String>>,
    modules: Vec<Module>,
}


#[derive(Deserialize, Debug)]
struct ModuleConf {
    file_name:String,
    content: ModuleContent,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum ModuleContent {
    Content(String),
    Prompt(String),
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();
    match opts {
        Opts::Install(ref install_opts) => install(install_opts),
        Opts::Search(ref search_opts) => search(search_opts),
    }
}

fn install(opts: &Install) -> Result<()> {

    let manifest = read_manifest(&opts.manifest_path)?;
    check_weidu_conf_lang(&manifest.game_language)?;
    let modules = &manifest.modules;

    let mut log = if let Some(output) = &opts.output {
        let file = match std::fs::OpenOptions::new().create(true).write(true).truncate(true).open(output) {
            Err(error) => return Err(
                anyhow!(format!("Could not create log file {} - {:?}", output, error)
            )),
            Ok(file) => file,
        };
        let buffered = BufWriter::new(file);
        Some(buffered)
    } else {
        None
    };

    let modules = match (opts.from_index, opts.to_index) {
        (Some(from_index), Some(to_index)) => &modules[from_index..to_index],
        (Some(from_index), None) => &modules[from_index..],
        (None, Some(to_index)) => &modules[..to_index],
        (None, None) => &modules,
    };
    let mut finished = false;
    for (index, module) in modules.iter().enumerate() {
        println!("module {} - {}", index, module.name);
        let tp2 = find_tp2(&module.name)?;
        let tp2_string = match tp2.into_os_string().into_string() {
            Ok(string) => string,
            Err(os_string) => {
                let os_str = os_string.as_os_str();
                let msg = os_str.to_string_lossy().to_owned();
                return Err(anyhow!(format!("{}", msg)));
            }
        };
        configure_module(module)?;
        let single_result = run_weidu(&tp2_string, module, &opts, &manifest.lang_preferences, &manifest.game_language)?;
        if let Some(ref mut file) = log {
            let _ = write_run_result(&single_result, file, module);
        }
        match single_result.status_code() {
            Some(0) => {
                let message = format!("module {} (index={}) finished with success.", 
                                module.name, index);
                if let Some(ref mut file) = log {
                    let _ = writeln!(file, "{}", message);
                }
                println!("{}", Green.bold().paint(message));
            }
            Some(3) => {
                let (message, color) = if opts.no_stop_on_warn || module.ignore_warnings {
                    let message = format!("module {} (index={}) finished with warning (status=3), ignoring as requested", 
                                            module.name, index);
                    (message, Yellow)
                } else {
                    finished = true;
                    let message = format!("module {} (index={}) finished with warning (status=3), stopping as requested", 
                                            module.name, index);
                    (message, Red)
                }; 
                if let Some(ref mut file) = log {
                    let _ = writeln!(file, "{}", message);
                }
                println!("{}", color.bold().paint(message));
            }
            Some(value) => {
                finished = true;
                let message = format!("module {} (index={}) finished with error (status={}), stopping.", 
                                        module.name, index, value);
                if let Some(ref mut file) = log {
                    let _ = writeln!(file, "{}", message);
                }
                println!("{}", Red.bold().paint(message));
            }
            None => if !single_result.success() {
                let message = format!("module {} (index={}) finished with success.", 
                                module.name, index);
                if let Some(ref mut file) = log {
                    let _ = writeln!(file, "{}", message);
                }
                println!("{}", Green.bold().paint(message));
            } else {
                finished = true;
                let message = format!("module {} (index={}) finished with error, stopping.", 
                                        module.name, index);
                if let Some(ref mut file) = log {
                    let _ = writeln!(file, "{}", message);
                }
                println!("{}", Red.bold().paint(message));
            }
        }
        if finished {
            break;
        }
    }
    Ok(())
}

fn search(opts: &Search) -> Result<()> {
    let manifest = read_manifest(&opts.manifest_path)?;
    for (idx, module) in manifest.modules.iter().enumerate() {
        if module.name.to_lowercase() == opts.name.to_lowercase() {
            println!("idx: '{}\n {:?}", idx, module);
            return Ok(())
        }
    }
    println!("module {} not found", opts.name);
    Ok(())
}

fn read_manifest(path: &str) -> Result<Manifest> {
    let file = match std::fs::File::open(path) {
        Err(error) => return Err(
            anyhow!(format!("Could not open manifest file {} - {:?}", path, error)
        )),
        Ok(file) => file,
    };
    let reader = BufReader::new(file);
    let manifest: Manifest = serde_yaml::from_reader(reader)?;
    Ok(manifest)
}

/**
 * Given a module name, finds a matching path to a .tp2 file
 * can be any of
 * - ${module}/${module}.tp2
 * - ${module}/setup-${module}.tp2
 * - ${module}.tp2
 * - setup-${module}.tp2
 * with case-insensitive search.
 * Search is done in this order and ignores other matches when one is found.
 */
fn find_tp2(module_name: &str) -> Result<PathBuf> {
    if let Some(path) = check_glob_casefold(&format!("./{}/{}.tp2", module_name, module_name))? {
        return Ok(path);
    }
    if let Some(path) = check_glob_casefold(&format!("./{}/setup-{}.tp2", module_name, module_name))? {
        return Ok(path);
    }
    if let Some(path) = check_glob_casefold(&format!("./{}.tp2", module_name))? {
        return Ok(path);
    }
    if let Some(path) = check_glob_casefold(&format!("./setup-{}.tp2", module_name))? {
        return Ok(path);
    }
    Err(anyhow!("tp2 file {}.tp2 not found", module_name))
}

fn check_glob_casefold(pattern: &str) -> Result<Option<PathBuf>> {
    println!("try {}", pattern);
    let options = MatchOptions {
        case_sensitive: false,
        ..Default::default()
    };
    let mut glob_result = glob_with(pattern, options)?;
    if let Some(path) = glob_result.find_map(|item| {
        match item {
            Err(_) => None,
            Ok(value) => Some(value),
        }
    }) {
        Ok(Some(path))
    } else {
        Ok(None)
    }
}

enum RunResult {
    Dry(String),
    Real(std::process::Output)
}

impl RunResult {
    fn status_code(&self) -> Option<i32> {
        match self {
            RunResult::Dry(_) => Some(0),
            RunResult::Real(output) => output.status.code(),
        }
    }
    fn success(&self) -> bool {
        match self {
            RunResult::Dry(_) => true,
            RunResult::Real(output) => output.status.success(),
        }
    }
}

fn configure_module(module: &Module) -> Result<()> {
    if let Some(conf) = &module.add_conf {
        let conf_path = Path::new(&module.name).join(&conf.file_name);
        let file = match std::fs::OpenOptions::new().create(true).write(true).truncate(true).open(&conf_path) {
            Err(error) => return Err(
                anyhow!(format!("Could not create conf file {:?} - {:?}", conf_path, error)
            )),
            Ok(file) => file,
        };
        let mut buffered = BufWriter::new(file);
        let content = match &conf.content {
            ModuleContent::Content(content) => content,
            ModuleContent::Prompt(_prompt) => {
                // print the prompt and read the content line
                bail!("not implemented yet")
            }
        };
        writeln!(buffered, "{}", content)?;
        buffered.flush()?;
        Ok(())
    } else { Ok(()) }
}

#[derive(Clone, Debug)]
struct LanguageOption {
    index: u32,
    name: String,
}

#[derive(Clone, Debug)]
enum LanguageSelection {
    Selected(u32),
    NoPrefSet(Vec<LanguageOption>),
    NoMatch(Vec<LanguageOption>),
}

fn select_language(tp2:&str, module: &Module, lang_preferences: &Option<Vec<String>>) -> Result<LanguageSelection> {
    use LanguageSelection::*;

    if let Some(idx) = module.language {
        Ok(Selected(idx))
    } else {
        let available = match list_available_languages(tp2, module) {
            Ok(result) => result,
            Err(error) =>  bail!(
                "Couldn't get list of available language for module {} - {:?}",
                module.name,
                error,
            )
        };
        match lang_preferences {
            None => Ok(NoPrefSet(available)),
            Some(names) if names.is_empty() => Ok(NoPrefSet(available)),
            Some(candidates) => {
                for candidate in candidates {
                    let candidate = candidate.trim();
                    if candidate.is_empty() {
                        continue;
                    }
                    match candidate.strip_prefix("#rx#") {
                        Some(reg) => {
                            let lang_re = regex::Regex::new(&format!("(?i){}", reg)).unwrap();
                            for lang in &available {
                                let LanguageOption { index, name } = &lang;
                                if lang_re.is_match(name) {
                                    return Ok(Selected(*index));
                                }
                            }
                        }
                        None => {
                            // use candidate for exact search
                            for lang in &available {
                                let LanguageOption { index, name } = &lang;
                                if candidate.to_lowercase() == name.to_lowercase() {
                                    return Ok(Selected(*index));
                                }
                            }                            
                        }
                    }
                }
                // tried everything, no match
                Ok(NoMatch(available))
            }
        }
    }
}

fn list_available_languages(tp2: &str, module: &Module) -> Result<Vec<LanguageOption>> {
    let mut command = Command::new("weidu");
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
    let lines = output.stdout.lines();
    for line in output.stderr.lines() {
        println!("on stderr {:?}", line);
    }


    use lazy_static::lazy_static;
    use regex::Regex;
    lazy_static! {
        static ref language_regex: Regex = Regex::new("^([0-9]*):(.*)$").unwrap();
    }
    let mut lines_ok = vec![];
    for line in lines {
        match line {
            Err(err) => bail!("Couldn't obtain language list for module{} [error reading output] _ {:?}", 
                            module.name, err),
            Ok(line) => {
                lines_ok.push(line);
            }
        }
    }
    let entries = lines_ok.iter().filter_map(|line| match language_regex.captures(line) {
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

fn run_weidu(tp2: &str, module: &Module, opts: &Install, lang_preferences: &Option<Vec<String>>, 
            game_lang: &str) -> Result<RunResult> {
    use LanguageSelection::*;
    let language_id = match select_language(tp2, module, lang_preferences) {
        Ok(Selected(id)) => id,
        Ok(NoPrefSet(available))
        | Ok(NoMatch(available)) => handle_no_language_selected(available, module, lang_preferences,game_lang)?,
        Err(err) => return Err(err),
    };
    match &module.components {
        None => run_weidu_interactive(tp2, module, opts, game_lang),
        Some(comp) if comp.is_empty() => run_weidu_interactive(tp2, module, opts, game_lang),
        Some(components) => run_weidu_auto(tp2, module, components, opts, game_lang, language_id)
    }
}

fn handle_no_language_selected(available: Vec<LanguageOption>, module: &Module, 
                                lang_pref: &Option<Vec<String>>, _game_lang: &str) -> Result<u32> {
    // may one day prompt user for selection and (if ok) same in the yaml file
    bail!(
        r#"No matching language found for module {} with language preferences {:?}
        Available choices are {:?}
        "#,
        module.name, lang_pref, available);
}

fn run_weidu_auto(tp2: &str, module: &Module, components: &Vec<u32>, opts: &Install, 
                    game_lang: &str, language_id: u32) -> Result<RunResult> {
    
    let mut command = Command::new("weidu");
    let mut args = vec![
        tp2.to_owned(),
        "--no-exit-pause".to_owned(),
        "--log".to_owned(),
        format!("setup-{}.debug", module.name),
        "--use-lang".to_owned(),
        game_lang.to_owned(),
        "--language".to_owned(), language_id.to_string(),
        "--force-install-list".to_owned(),
    ];
    args.extend(components.iter().map(|id| id.to_string()));
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

fn run_weidu_interactive(tp2: &str, module: &Module, opts: &Install, 
                            game_lang: &str) -> Result<RunResult> {
    let mut command = Command::new("weidu");
    let args = vec![
        "install".to_owned(),
        tp2.to_owned(),
        "--no-exit-pause".to_owned(),
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

fn write_run_result(result: &RunResult, file: &mut BufWriter<std::fs::File>, module: &Module) -> Result<()> {
    match result {
        RunResult::Real(result) => {
            let _ = file.write(&result.stdout)?;
            let _ = file.write(&result.stderr)?;
            let _ = writeln!(file, "\n==\nmodule {} finished with status {:?}\n", 
                                module.name, result.status.code());
        }
        RunResult::Dry(cmd) => {
            let _ = writeln!(file, "dry-run: {}", cmd)?;
        }
    }
    let _ = file.flush();
    Ok(())
}

fn check_weidu_conf_lang(lang: &str) -> Result<()> {
    if !Path::new("weidu.conf").exists() {
        return Ok(())
    }
    let file = match std::fs::File::open("weidu.conf") {
        Err(error) => return Err(
            anyhow!(format!("Could not open weidu.conf - {:?}", error)
        )),
        Ok(file) => file,
    };
    let regex = regex::Regex::new(r##"(?i)lang_dir(\s)+=(\s)+([a-z_]+)"##)?;
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line?;
        if let Some(caps) = regex.captures_iter(&line).next() {
            if caps[3].to_lowercase() == lang.to_lowercase() {
                return Ok(())
            } else {
                bail!(
                    "lang_dir (in manifest) {} doesn't match value in weidu.conf {}",
                    lang, &caps[3]
                );
            }
        }
    }
    Ok(())
}
