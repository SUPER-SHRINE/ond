mod args;
mod json;
use ond_cli::{CompileRequest, LinkRequest};
use ond_protocol::{CompilationManifest, Contract};
use std::io::{self, Read, Write};

fn execute(command: args::Command) -> Result<(), Vec<String>> {
    match command {
        args::Command::Compile {
            root,
            output,
            cache,
            verbose,
            test,
        } => {
            let root = root.canonicalize().map_err(|e| vec![e.to_string()])?;
            let root = if root.is_file() {
                root.parent().unwrap().to_path_buf()
            } else {
                root
            };
            let result = ond_cli::compile(
                CompileRequest {
                    test,
                    excluded_dirs: Vec::new(),
                    contract: Contract::default(),
                    project_root: root.clone(),
                    sources: Default::default(),
                    cache_dir: cache,
                },
                &ond_cli::compiler_identity()?,
            )?;
            if verbose {
                eprintln!("rebuilt: {}", result.rebuilt.join(", "));
                eprintln!("reused: {}", result.reused.join(", "));
            }
            let output = output.unwrap_or_else(|| {
                root.join(if test {
                    "target/test.ondbuild"
                } else {
                    "target/build.ondbuild"
                })
            });
            write(&output, &json::encode(&result.manifest)?)
        }
        args::Command::Link {
            input,
            output,
            layout,
            return_to,
        } => {
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
            let linked = ond_cli::link_with_debug(LinkRequest {
                manifest,
                layout,
                main_return_address: return_to,
            })?;
            let image_bytes = json::encode(&linked.image)?;
            let debug_bytes = json::encode(&linked.debug)?;
            if output == std::path::Path::new("-") {
                write(&output, &image_bytes)?;
            } else {
                let debug_output = output.with_extension("onddebug");
                if output
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| {
                        extension.eq_ignore_ascii_case("onddebug")
                            || (linked.tests.is_some() && extension.eq_ignore_ascii_case("ondtest"))
                    })
                {
                    return Err(vec![
                        "image output path conflicts with a sidecar path".into(),
                    ]);
                }
                write(&debug_output, &debug_bytes)?;
                if let Some(tests) = &linked.tests {
                    write(&output.with_extension("ondtest"), &json::encode(tests)?)?;
                }
                write(&output, &image_bytes)?;
            }
            Ok(())
        }
        args::Command::Help => {
            println!("{}", args::HELP);
            Ok(())
        }
    }
}
fn write(path: &std::path::Path, bytes: &[u8]) -> Result<(), Vec<String>> {
    if path == std::path::Path::new("-") {
        io::stdout()
            .lock()
            .write_all(bytes)
            .map_err(|e| vec![e.to_string()])
    } else {
        let absolute = std::path::absolute(path).map_err(|e| vec![e.to_string()])?;
        ond_cli::write_artifact(&absolute, bytes)
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
    if let Err(diagnostics) = execute(command) {
        for diagnostic in diagnostics {
            eprintln!("{diagnostic}");
        }
        std::process::exit(1);
    }
}
