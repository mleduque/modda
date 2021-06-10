
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

#[derive(Clap)]
#[clap(version = "1.0")]
#[clap(setting = AppSettings::ColoredHelp)]
struct Opts {

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


#[derive(Deserialize)]
struct Module {
    name: String,
    language: usize,
    components: Vec<usize>,
    #[serde(default)]
    ignore_warnings: bool,
}

#[derive(Deserialize)]
struct Manifest {
    #[serde(rename = "lang_dir")]
    game_language: String,
    modules: Vec<Module>,
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    let file = match std::fs::File::open(&opts.manifest_path) {
        Err(error) => return Err(
            anyhow!(format!("Could not open manifest file {} - {:?}", opts.manifest_path, error)
        )),
        Ok(file) => file,
    };
    let reader = BufReader::new(file);
    let manifest: Manifest = serde_yaml::from_reader(reader)?;
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
        let single_result = run_weidu(&tp2_string, module, &opts, &manifest.game_language)?;
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

fn run_weidu(tp2: &str, module: &Module, opts: &Opts, game_lang: &str) -> Result<RunResult> {
    let language = module.language.to_string();
    
    let mut command = Command::new("weidu");
    let mut args = vec![
        tp2.to_owned(),
        "--no-exit-pause".to_owned(),
        "--log".to_owned(),
        format!("setup-{}.debug", module.name),
        "--use-lang".to_owned(),
        game_lang.to_owned(),
        "--language".to_owned(), language,
        "--force-install".to_owned(),
    ];
    args.extend(module.components.iter().map(|id| id.to_string()));
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
