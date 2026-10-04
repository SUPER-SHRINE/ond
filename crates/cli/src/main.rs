mod args;
use ond_cli::{CompileRequest, LinkRequest};
use ond_protocol::{CompilationManifest, Contract};
use std::io::{self, Read, Write};

fn execute(command: args::Command) -> Result<(), Vec<String>> {
    match command {
        args::Command::Compile {
            root,
            output,
            mut libraries,
            cache,
            verbose,
        } => {
            let root = root.canonicalize().map_err(|e| vec![e.to_string()])?;
            let root = if root.is_file() {
                root.parent().unwrap().to_path_buf()
            } else {
                root
            };
            let resolved = ond_cli::resolve_dependencies(&root)?;
            if let Some(value) = std::env::var_os("OND_PATH") {
                if !value.is_empty() {
                    for path in std::env::split_paths(&value) {
                        if path.as_os_str().is_empty() {
                            return Err(vec!["OND_PATH contains an empty path".into()]);
                        }
                        libraries.push(path);
                    }
                }
            }
            let mut excluded_dirs = libraries
                .iter()
                .map(|p| p.canonicalize().map_err(|e| vec![e.to_string()]))
                .collect::<Result<Vec<_>, _>>()?;
            if excluded_dirs.contains(&root) {
                return Err(vec!["project root cannot also be a library root".into()]);
            }
            excluded_dirs.extend(resolved.ignored_directories);
            let mut library_sources = ond_cli::library_sources(&root, &libraries)?;
            for (name, source) in resolved.sources {
                if let Some(previous) = library_sources.insert(name.clone(), source.text.clone())
                    && previous != source.text
                {
                    return Err(vec![format!("conflicting dependency source: {name}")]);
                }
            }
            let result = ond_cli::compile(
                CompileRequest {
                    excluded_dirs,
                    contract: Contract::default(),
                    project_root: root.clone(),
                    sources: Default::default(),
                    libraries: library_sources,
                    cache_dir: cache,
                },
                &ond_cli::compiler_identity()?,
            )?;
            if verbose {
                eprintln!("rebuilt: {}", result.rebuilt.join(", "));
                eprintln!("reused: {}", result.reused.join(", "));
            }
            let output = output.unwrap_or_else(|| root.join("target/build.ondbuild"));
            write(
                &output,
                &serde_json::to_vec(&result.manifest).map_err(|e| vec![e.to_string()])?,
            )
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
            let manifest: CompilationManifest = serde_json::from_slice(&bytes)
                .map_err(|e| vec![format!("invalid compilation manifest: {e}")])?;
            let linked = ond_cli::link_with_debug(LinkRequest {
                manifest,
                layout,
                main_return_address: return_to,
            })?;
            let image_bytes = serde_json::to_vec(&linked.image).map_err(|e| vec![e.to_string()])?;
            let debug_bytes = serde_json::to_vec(&linked.debug).map_err(|e| vec![e.to_string()])?;
            if output == std::path::Path::new("-") {
                write(&output, &image_bytes)?;
            } else {
                let debug_output = output.with_extension("onddebug");
                write(&debug_output, &debug_bytes)?;
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
