mod args;
mod init;
mod json;
mod publish;
use ond_cli::CompileRequest;
use ond_protocol::{CompilationManifest, Contract};
use std::io::{self, Read, Write};
fn execute(command: args::Command) -> Result<(), Vec<String>> {
    match command {
        args::Command::Compile {
            root,
            output,
            verbose,
            role,
        } => {
            let root = root.canonicalize().map_err(|e| vec![e.to_string()])?;
            let root = if root.is_file() {
                root.parent().unwrap().to_path_buf()
            } else {
                root
            };
            let settings = ond_cli::settings::read_settings(&root)?;
            let result = ond_cli::compile(
                CompileRequest {
                    role,
                    excluded_dirs: Vec::new(),
                    contract: Contract::default(),
                    project_root: root.clone(),
                    sources: Default::default(),
                    cache_dir: Some(root.join(&settings.compile.cache_dir)),
                },
                &ond_cli::compiler_identity()?,
            )?;
            if verbose {
                eprintln!("rebuilt: {}", result.rebuilt.join(", "));
                eprintln!("reused: {}", result.reused.join(", "));
            }
            let output = output.unwrap_or_else(|| root.join(settings.compile.output));
            write(&output, &json::encode(&result.manifest)?)
        }
        args::Command::Link {
            input,
            project,
            output,
        } => {
            let root = project.canonicalize().map_err(|e| vec![e.to_string()])?;
            let settings = ond_cli::settings::read_settings(&root)?;
            let link = settings
                .link
                .ok_or_else(|| vec!["missing [link] settings".into()])?;
            link.layout()?;
            let input = input.unwrap_or_else(|| root.join(settings.compile.output));
            let bytes = if input == std::path::Path::new("-") {
                let mut bytes = Vec::new();
                io::stdin()
                    .read_to_end(&mut bytes)
                    .map_err(|e| vec![e.to_string()])?;
                bytes
            } else {
                std::fs::read(&input).map_err(|e| vec![format!("{}: {e}", input.display())])?
            };
            let mut manifest: CompilationManifest = serde_json::from_slice(&bytes)
                .map_err(|e| vec![format!("invalid compilation manifest: {e}")])?;
            if input != std::path::Path::new("-") {
                let parent = input.parent().unwrap_or_else(|| std::path::Path::new("."));
                for reference in &mut manifest.packages {
                    if reference.path.is_relative() {
                        reference.path = parent.join(&reference.path);
                    }
                }
            }
            let output = output.unwrap_or_else(|| root.join(&link.output));
            let (linked, bytes) = ond_cli::link_executable(manifest, &link)?;
            if output == std::path::Path::new("-") {
                write(&output, &bytes)
            } else {
                let files = vec![
                    (output.clone(), bytes),
                    (
                        output.with_extension("onddebug"),
                        json::encode(&linked.debug)?,
                    ),
                ];
                publish::publish(&files)
            }
        }
        args::Command::Help => {
            println!("{}", args::HELP);
            Ok(())
        }
        args::Command::Init => init::run(),
    }
}
fn write(path: &std::path::Path, bytes: &[u8]) -> Result<(), Vec<String>> {
    if path == std::path::Path::new("-") {
        io::stdout()
            .lock()
            .write_all(bytes)
            .map_err(|e| vec![e.to_string()])
    } else {
        ond_cli::write_artifact(
            &std::path::absolute(path).map_err(|e| vec![e.to_string()])?,
            bytes,
        )
    }
}
fn main() {
    let command = match args::parse(std::env::args_os().skip(1).collect()) {
        Ok(command) => command,
        Err(message) => {
            eprintln!("{message}\n{}", args::HELP);
            std::process::exit(2);
        }
    };
    if let args::Command::Compile {
        root,
        role: Some(_),
        ..
    } = &command
        && let Ok(loaded) = ond_compiler_core::load_project(root)
        && !loaded
            .packages
            .iter()
            .any(|package| package.logical_path == ".")
    {
        eprintln!("--role is only valid for an executable project");
        std::process::exit(2);
    }
    if let Err(diagnostics) = execute(command) {
        for diagnostic in diagnostics {
            eprintln!("{diagnostic}");
        }
        std::process::exit(1);
    }
}
