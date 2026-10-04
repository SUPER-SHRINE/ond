use ond_protocol::link::{LinkPlan, MemoryRegion};
use std::{ffi::OsString, path::PathBuf};

pub const HELP: &str = "Usage:
  ond compile [project] [-o FILE|-] [--library-dir DIR]... [--cache-dir DIR] [-v]
  ond link FILE|- --ram BASE:SIZE --stack SIZE --return-to ADDRESS
           [--read-only BASE:SIZE] [-o FILE|-]
  ond help
Defaults: project=., compile output=<project>/target/build.ondbuild,
          link output=a.ondimage. Diagnostics go to stderr.
Library roots may also be supplied through the OND_PATH environment variable
using the operating system's path-list separator.
Numbers: decimal or 0xHEX, optionally suffixed KiB/MiB.
Relative command-line paths are relative to the current directory.";

pub enum Command {
    Compile {
        root: PathBuf,
        output: Option<PathBuf>,
        libraries: Vec<PathBuf>,
        cache: Option<PathBuf>,
        verbose: bool,
    },
    Link {
        input: PathBuf,
        output: PathBuf,
        layout: LinkPlan,
        return_to: u32,
    },
    Help,
}
pub fn parse(args: Vec<OsString>) -> Result<Command, String> {
    if args.is_empty() {
        return Ok(Command::Help);
    }
    let Some(command) = args.first().and_then(|s| s.to_str()) else {
        return Err("missing command".into());
    };
    if args.len() == 1 && matches!(command, "help" | "--help" | "-h") {
        return Ok(Command::Help);
    }
    if !matches!(command, "compile" | "link") {
        return Err(format!("unknown command: {command}"));
    }
    let mut position = None;
    let mut output = None;
    let mut libraries = Vec::new();
    let mut cache = None;
    let (mut ram, mut readonly, mut stack, mut return_to) = (None, None, None, None);
    let mut verbose = false;
    let mut positional = false;
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
            if flag == "-v" && command == "compile" {
                verbose = true;
                continue;
            }
            let valid = flag == "-o"
                || (command == "compile" && matches!(flag, "--library-dir" | "--cache-dir"))
                || (command == "link"
                    && matches!(flag, "--ram" | "--read-only" | "--stack" | "--return-to"));
            if !valid {
                return Err(format!("unknown {command} option: {flag}"));
            }
            let value = iter
                .next()
                .ok_or_else(|| format!("missing value for {flag}"))?;
            match flag {
                "-o" => set(&mut output, PathBuf::from(value), flag)?,
                "--library-dir" => libraries.push(PathBuf::from(value)),
                "--cache-dir" => set(
                    &mut cache,
                    std::path::absolute(value).map_err(|e| e.to_string())?,
                    flag,
                )?,
                "--ram" => set(&mut ram, region(value)?, flag)?,
                "--read-only" => set(&mut readonly, region(value)?, flag)?,
                "--stack" => set(&mut stack, number(value)?, flag)?,
                "--return-to" => set(&mut return_to, number(value)?, flag)?,
                _ => unreachable!(),
            }
        } else if position.replace(PathBuf::from(arg)).is_some() {
            return Err("too many positional arguments".into());
        }
    }
    if command == "compile" {
        if position.as_deref() == Some(std::path::Path::new("-")) {
            return Err("compile takes a project directory, not stdin source".into());
        }
        return Ok(Command::Compile {
            root: position.unwrap_or_else(|| ".".into()),
            output,
            libraries,
            cache,
            verbose,
        });
    }
    let ram = ram.ok_or("missing --ram BASE:SIZE")?;
    Ok(Command::Link {
        input: position.ok_or("missing compilation manifest (use - for stdin)")?,
        output: output.unwrap_or_else(|| "a.ondimage".into()),
        layout: LinkPlan {
            read_only: readonly,
            memory_base: ram.base,
            memory_size: ram.size,
            stack_size: stack.ok_or("missing --stack SIZE")?,
            entry_symbol: "__ond.entry".into(),
            heap_base_symbol: "__ond_heap_base".into(),
            stack_bottom_symbol: "__ond_stack_bottom".into(),
        },
        return_to: return_to.ok_or("missing --return-to ADDRESS")?,
    })
}
fn set<T>(slot: &mut Option<T>, value: T, flag: &str) -> Result<(), String> {
    if slot.replace(value).is_some() {
        Err(format!("duplicate option: {flag}"))
    } else {
        Ok(())
    }
}
fn number(value: &std::ffi::OsStr) -> Result<u32, String> {
    let original = value.to_str().ok_or("non-text number")?;
    let (text, multiplier) = if let Some(n) = original.strip_suffix("MiB") {
        (n, 1024u64 * 1024)
    } else if let Some(n) = original.strip_suffix("KiB") {
        (n, 1024)
    } else {
        (original, 1)
    };
    let n = if let Some(hex) = text.strip_prefix("0x") {
        u64::from_str_radix(hex, 16)
    } else {
        text.parse()
    };
    n.ok()
        .and_then(|v| v.checked_mul(multiplier))
        .and_then(|v| u32::try_from(v).ok())
        .ok_or_else(|| format!("invalid u32 number: {original}"))
}
fn region(value: &std::ffi::OsStr) -> Result<MemoryRegion, String> {
    let (base, size) = value
        .to_str()
        .and_then(|s| s.split_once(':'))
        .ok_or("region must be BASE:SIZE")?;
    Ok(MemoryRegion {
        base: number(base.as_ref())?,
        size: number(size.as_ref())?,
    })
}
