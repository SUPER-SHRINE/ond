mod completion;
mod hover;
mod hover_reference;
mod navigation;
mod protocol;
mod rename;
mod rename_state;
mod semantic_tokens;
mod state;
mod symbol_index;
mod symbols;
mod util;

use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use protocol::{
    CompletionParams, DefinitionParams, DidChangeTextDocumentParams, DidChangeWatchedFilesParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, DocumentSymbolParams, HoverParams,
    PrepareRenameParams, ReferenceParams, RenameParams, SemanticTokensParams, deserialize_params,
    publish_diagnostics, read_message, write_json,
};
use serde_json::{Value, json};
use state::ServerState;
use util::{log_stderr, normalize_path, path_from_uri};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            let _ = writeln!(io::stderr(), "{message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        None | Some("stdio") => run_stdio(),
        Some("inspect-project") => {
            let Some(root) = args.next() else {
                return Err("usage: ond-lsp inspect-project <project-dir|ond.toml>".to_string());
            };
            inspect_project(PathBuf::from(root))
        }
        Some("help" | "--help" | "-h") => {
            println!("{}", usage());
            Ok(())
        }
        Some(other) => Err(format!("unknown command `{other}`\n\n{}", usage())),
    }
}

fn usage() -> String {
    [
        "Usage:",
        "  ond-lsp stdio",
        "  ond-lsp inspect-project <project-dir|ond.toml>",
        "  ond-lsp help",
    ]
    .join("\n")
}

fn run_stdio() -> Result<(), String> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = stdout.lock();
    let mut state = ServerState::with_library_roots(library_roots_from_env()?);

    run_transport(&mut reader, &mut writer, &mut state)
}

fn run_transport(
    reader: &mut impl BufRead,
    mut writer: &mut impl Write,
    state: &mut ServerState,
) -> Result<(), String> {
    while let Some(message) = read_message(reader)? {
        let method = message
            .get("method")
            .and_then(Value::as_str)
            .map(str::to_string);
        let id = message.get("id").cloned();

        if method.as_deref() == Some("exit") {
            break;
        }

        // Keep request/notification errors inside this iteration. A `?` in a
        // handler must not tear down the stdio transport and trigger VS Code's
        // crash-restart loop.
        let result = (|| -> Result<(), String> {
            match method.as_deref() {
                Some("initialize") => write_json(&mut writer, &initialize_response(id.clone())),
                Some("initialized") => Ok(()),
                Some("shutdown") => write_json(
                    &mut writer,
                    &json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": Value::Null
                    }),
                ),
                Some("exit") => Ok(()),
                Some("textDocument/didOpen") => {
                    let params: DidOpenTextDocumentParams =
                        deserialize_params(message.get("params"))?;
                    let path = normalize_path(path_from_uri(&params.text_document.uri)?);
                    log_stderr(&format!("didOpen: {}", path.display()));
                    state.open_document(
                        params.text_document.uri,
                        params.text_document.text,
                        &mut writer,
                    )
                }
                Some("textDocument/didChange") => {
                    let params: DidChangeTextDocumentParams =
                        deserialize_params(message.get("params"))?;
                    let path = normalize_path(path_from_uri(&params.text_document.uri)?);
                    log_stderr(&format!("didChange: {}", path.display()));
                    if params.content_changes.is_empty() {
                        Ok(())
                    } else {
                        state.change_document(
                            params.text_document.uri,
                            params.content_changes,
                            &mut writer,
                        )
                    }
                }
                Some("textDocument/didClose") => {
                    let params: DidCloseTextDocumentParams =
                        deserialize_params(message.get("params"))?;
                    let path = normalize_path(path_from_uri(&params.text_document.uri)?);
                    log_stderr(&format!("didClose: {}", path.display()));
                    state.close_document(&params.text_document.uri)?;
                    publish_diagnostics(&mut writer, &params.text_document.uri, Vec::new())
                }
                Some("workspace/didChangeWatchedFiles") => {
                    let params: DidChangeWatchedFilesParams =
                        deserialize_params(message.get("params"))?;
                    let relevant = params.changes.iter().any(|change| {
                        path_from_uri(&change.uri).is_ok_and(|path| {
                            !path
                                .components()
                                .any(|component| component.as_os_str() == ".ond")
                        })
                    });
                    if !relevant {
                        return Ok(());
                    }
                    log_stderr("workspace files changed; reloading Ond projects");
                    state.watched_files_changed(&mut writer)
                }
                Some("textDocument/documentSymbol") => {
                    let params: DocumentSymbolParams = deserialize_params(message.get("params"))?;
                    let symbols = state.document_symbols(&params.text_document.uri)?;
                    write_json(
                        &mut writer,
                        &json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": symbols
                        }),
                    )
                }
                Some("textDocument/hover") => {
                    let params: HoverParams = deserialize_params(message.get("params"))?;
                    let hover = state.hover(
                        &params.text_document.uri,
                        params.position.line,
                        params.position.character,
                    )?;
                    write_json(
                        &mut writer,
                        &json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": hover
                        }),
                    )
                }
                Some("textDocument/definition") => {
                    let params: DefinitionParams = deserialize_params(message.get("params"))?;
                    let definition = state.definition(
                        &params.text_document.uri,
                        params.position.line,
                        params.position.character,
                    )?;
                    write_json(
                        &mut writer,
                        &json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": definition
                        }),
                    )
                }
                Some("textDocument/prepareRename") => {
                    let params: PrepareRenameParams = deserialize_params(message.get("params"))?;
                    let prepared = state.prepare_rename(
                        &params.text_document.uri,
                        params.position.line,
                        params.position.character,
                    )?;
                    write_json(
                        &mut writer,
                        &json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": prepared
                        }),
                    )
                }
                Some("textDocument/references") => {
                    let params: ReferenceParams = deserialize_params(message.get("params"))?;
                    let references = state.references(
                        &params.text_document.uri,
                        params.position.line,
                        params.position.character,
                        params.context.include_declaration,
                    )?;
                    write_json(
                        &mut writer,
                        &json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": references
                        }),
                    )
                }
                Some("textDocument/rename") => {
                    let params: RenameParams = deserialize_params(message.get("params"))?;
                    let edit = state.rename(
                        &params.text_document.uri,
                        params.position.line,
                        params.position.character,
                        &params.new_name,
                    )?;
                    write_json(
                        &mut writer,
                        &json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": edit
                        }),
                    )
                }
                Some("textDocument/semanticTokens/full") => {
                    let params: SemanticTokensParams = deserialize_params(message.get("params"))?;
                    let tokens = state.semantic_tokens(&params.text_document.uri)?;
                    write_json(
                        &mut writer,
                        &json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": {
                                "data": tokens
                            }
                        }),
                    )
                }
                Some("textDocument/completion") => {
                    let params: CompletionParams = deserialize_params(message.get("params"))?;
                    log_stderr(&format!(
                        "completion: uri={} line={} character={}",
                        params.text_document.uri, params.position.line, params.position.character
                    ));
                    let items = state.completions(
                        &params.text_document.uri,
                        params.position.line,
                        params.position.character,
                    )?;
                    log_stderr(&format!("completionItems: count={}", items.len()));
                    write_json(
                        &mut writer,
                        &json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": {
                                "isIncomplete": false,
                                "items": items
                            }
                        }),
                    )
                }
                _ => {
                    if let Some(id) = id.clone() {
                        write_json(
                            &mut writer,
                            &json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "error": {
                                    "code": -32601,
                                    "message": "method not found"
                                }
                            }),
                        )
                    } else {
                        Ok(())
                    }
                }
            }
        })();

        if let Err(error) = result {
            if let Some(id) = id {
                write_json(
                    &mut writer,
                    &json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {
                            "code": -32603,
                            "message": error
                        }
                    }),
                )?;
            } else {
                let _ = writeln!(io::stderr(), "{error}");
            }
        }
    }

    Ok(())
}

fn library_roots_from_env() -> Result<Vec<PathBuf>, String> {
    let Some(value) = std::env::var_os("OND_PATH") else {
        return Ok(Vec::new());
    };
    if value.is_empty() {
        return Ok(Vec::new());
    }
    std::env::split_paths(&value)
        .map(|path| {
            if path.as_os_str().is_empty() {
                return Err("OND_PATH contains an empty path".to_string());
            }
            path.canonicalize()
                .map_err(|error| format!("library {}: {error}", path.display()))
        })
        .collect()
}

fn initialize_response(id: Option<Value>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "capabilities": {
                "textDocumentSync": 1,
                "documentSymbolProvider": true,
                "hoverProvider": true,
                "definitionProvider": true,
                "referencesProvider": true,
                "renameProvider": {
                    "prepareProvider": true
                },
                "semanticTokensProvider": {
                    "legend": {
                        "tokenTypes": ["namespace", "type", "function", "property", "variable", "receiver", "method", "parameter", "typeParameter"],
                        "tokenModifiers": ["readonly", "declaration"]
                    },
                    "full": true
                },
                "completionProvider": {
                    "resolveProvider": false,
                    "triggerCharacters": [".", "\"", "/", "+", "-", "*", "%", "=", "&", "|", "^", "!", "<", ">", ",", "(", "["]
                }
            },
            "serverInfo": {
                "name": "ond-lsp",
                "version": env!("CARGO_PKG_VERSION")
            }
        }
    })
}

fn inspect_project(root: PathBuf) -> Result<(), String> {
    let project = compiler::load_project(&root).map_err(util::render_frontend_error)?;
    println!("root: {}", project.root.display());
    println!("manifest: {}", project.manifest_path.display());
    println!("packages: {}", project.packages.len());
    println!("files: {}", project.sources.files().len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::{self, Cursor, Write};

    use serde_json::json;

    use super::{ServerState, initialize_response, run_transport};

    struct FailingWriter;

    impl Write for FailingWriter {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "simulated write failure",
            ))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn initialize_reports_package_version() {
        let response = initialize_response(Some(json!(1)));
        assert_eq!(
            response["result"]["serverInfo"]["version"],
            env!("CARGO_PKG_VERSION")
        );
    }

    #[test]
    fn initialize_advertises_navigation_and_rename_support() {
        let response = initialize_response(Some(json!(7)));
        assert_eq!(response["id"], 7);
        assert_eq!(
            response["result"]["capabilities"]["definitionProvider"],
            true
        );
        assert_eq!(
            response["result"]["capabilities"]["referencesProvider"],
            true
        );
        assert_eq!(
            response["result"]["capabilities"]["renameProvider"]["prepareProvider"],
            true
        );
    }

    #[test]
    fn request_error_does_not_end_the_transport_loop() {
        let malformed_completion = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/completion",
            "params": {}
        });
        let initialize = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "initialize",
            "params": {}
        });
        let exit = json!({
            "jsonrpc": "2.0",
            "method": "exit"
        });
        let input = [malformed_completion, initialize, exit]
            .into_iter()
            .map(|message| {
                let body = serde_json::to_vec(&message).unwrap();
                format!("Content-Length: {}\r\n\r\n", body.len())
                    .into_bytes()
                    .into_iter()
                    .chain(body)
                    .collect::<Vec<_>>()
            })
            .flatten()
            .collect::<Vec<_>>();
        let mut reader = Cursor::new(input);
        let mut writer = Vec::new();
        let mut state = ServerState::default();

        run_transport(&mut reader, &mut writer, &mut state).unwrap();

        let output = String::from_utf8(writer).unwrap();
        assert!(output.contains("\"id\":1"), "{output}");
        assert!(output.contains("\"code\":-32603"), "{output}");
        assert!(output.contains("\"id\":2"), "{output}");
        assert!(output.contains("\"serverInfo\""), "{output}");
    }

    #[test]
    fn request_error_response_write_failure_ends_the_transport_loop() {
        let message = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/completion",
            "params": {}
        });
        let body = serde_json::to_vec(&message).unwrap();
        let mut input = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
        input.extend(body);
        let mut reader = Cursor::new(input);
        let mut writer = FailingWriter;
        let mut state = ServerState::default();

        let error = run_transport(&mut reader, &mut writer, &mut state).unwrap_err();

        assert_eq!(error, "simulated write failure");
    }
}
