use std::{ffi::OsString, path::PathBuf};
pub const HELP: &str = "Usage:\n  ond compile [PROJECT] [--role NAME] [-o FILE|-] [-v]\n  ond link [FILE|-] [--project DIR] [-o FILE|-]\n  ond help\nSettings are read from the selected project's ond.toml. Diagnostics go to stderr.\nRelative command-line paths are relative to the current directory.";
pub enum Command {
    Compile {
        root: PathBuf,
        output: Option<PathBuf>,
        verbose: bool,
        role: Option<String>,
    },
    Link {
        input: Option<PathBuf>,
        project: PathBuf,
        output: Option<PathBuf>,
    },
    Help,
}
pub fn parse(args: Vec<OsString>) -> Result<Command, String> {
    let Some(command) = args.first().and_then(|s| s.to_str()) else {
        return if args.is_empty() {
            Ok(Command::Help)
        } else {
            Err("invalid command".into())
        };
    };
    if args.len() == 1 && matches!(command, "help" | "--help" | "-h") {
        return Ok(Command::Help);
    }
    if !matches!(command, "compile" | "link") {
        return Err(format!("unknown command: {command}"));
    }
    let (mut position, mut output, mut project) = (None, None, None);
    let (mut verbose, mut positional) = (false, false);
    let mut role = None;
    let mut iter = args[1..].iter();
    while let Some(arg) = iter.next() {
        if !positional && arg == "--" {
            positional = true;
            continue;
        }
        if !positional && matches!(arg.to_str(), Some("-h" | "--help")) {
            return Ok(Command::Help);
        }
        if !positional && arg.to_str().is_some_and(|s| s.starts_with('-') && s != "-") {
            let flag = arg.to_str().unwrap();
            match flag {
                "--role" if command == "compile" => {
                    if role.is_some() {
                        return Err("duplicate option: --role".into());
                    }
                    let value = iter
                        .next()
                        .and_then(|value| value.to_str())
                        .filter(|value| !value.starts_with('-'))
                        .ok_or("missing value for --role")?;
                    role = Some(value.to_string());
                }
                "-v" if command == "compile" => {
                    if verbose {
                        return Err("duplicate option: -v".into());
                    }
                    verbose = true;
                }
                "-o" | "--project" if flag == "-o" || command == "link" => {
                    let value = iter
                        .next()
                        .ok_or_else(|| format!("missing value for {flag}"))?;
                    if value
                        .to_str()
                        .is_some_and(|s| s.starts_with('-') && s != "-")
                    {
                        return Err(format!("missing value for {flag}"));
                    }
                    let slot = if flag == "-o" {
                        &mut output
                    } else {
                        &mut project
                    };
                    if slot.replace(PathBuf::from(value)).is_some() {
                        return Err(format!("duplicate option: {flag}"));
                    }
                }
                _ => return Err(format!("unknown {command} option: {flag}")),
            }
        } else if position.replace(PathBuf::from(arg)).is_some() {
            return Err("too many positional arguments".into());
        }
    }
    if command == "compile" {
        if position.as_deref() == Some(std::path::Path::new("-")) {
            return Err("compile takes a project directory".into());
        }
        Ok(Command::Compile {
            root: position.unwrap_or_else(|| ".".into()),
            output,
            verbose,
            role,
        })
    } else {
        Ok(Command::Link {
            input: position,
            project: project.unwrap_or_else(|| ".".into()),
            output,
        })
    }
}
