use std::io::{BufRead, Write};

use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Deserialize)]
pub struct DidOpenTextDocumentParams {
    #[serde(rename = "textDocument")]
    pub text_document: TextDocumentItem,
}

#[derive(Debug, Deserialize)]
pub struct DidChangeTextDocumentParams {
    #[serde(rename = "textDocument")]
    pub text_document: VersionedTextDocumentIdentifier,
    #[serde(rename = "contentChanges")]
    pub content_changes: Vec<TextDocumentContentChangeEvent>,
}

#[derive(Debug, Deserialize)]
pub struct DidCloseTextDocumentParams {
    #[serde(rename = "textDocument")]
    pub text_document: TextDocumentIdentifier,
}

#[derive(Debug, Deserialize)]
pub struct DidChangeWatchedFilesParams {
    pub changes: Vec<FileEvent>,
}

#[derive(Debug, Deserialize)]
pub struct FileEvent {
    pub uri: String,
}

#[derive(Debug, Deserialize)]
pub struct DocumentSymbolParams {
    #[serde(rename = "textDocument")]
    pub text_document: TextDocumentIdentifier,
}

#[derive(Debug, Deserialize)]
pub struct CompletionParams {
    #[serde(rename = "textDocument")]
    pub text_document: TextDocumentIdentifier,
    pub position: Position,
}

#[derive(Debug, Deserialize)]
pub struct HoverParams {
    #[serde(rename = "textDocument")]
    pub text_document: TextDocumentIdentifier,
    pub position: Position,
}

pub type DefinitionParams = HoverParams;

pub type PrepareRenameParams = HoverParams;

#[derive(Debug, Deserialize)]
pub struct ReferenceParams {
    #[serde(rename = "textDocument")]
    pub text_document: TextDocumentIdentifier,
    pub position: Position,
    pub context: ReferenceContext,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceContext {
    #[serde(rename = "includeDeclaration")]
    pub include_declaration: bool,
}

#[derive(Debug, Deserialize)]
pub struct RenameParams {
    #[serde(rename = "textDocument")]
    pub text_document: TextDocumentIdentifier,
    pub position: Position,
    #[serde(rename = "newName")]
    pub new_name: String,
}

#[derive(Debug, Deserialize)]
pub struct SemanticTokensParams {
    #[serde(rename = "textDocument")]
    pub text_document: TextDocumentIdentifier,
}

#[derive(Debug, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Deserialize)]
pub struct TextDocumentItem {
    pub uri: String,
    #[serde(rename = "version")]
    pub _version: i32,
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub struct VersionedTextDocumentIdentifier {
    pub uri: String,
    #[serde(rename = "version")]
    pub _version: i32,
}

#[derive(Debug, Deserialize)]
pub struct TextDocumentIdentifier {
    pub uri: String,
}

#[derive(Debug, Deserialize)]
pub struct TextDocumentContentChangeEvent {
    pub text: String,
    #[serde(default)]
    pub range: Option<LspRange>,
}

#[derive(Debug, Deserialize)]
pub struct LspRange {
    pub start: Position,
    pub end: Position,
}

pub fn deserialize_params<T: for<'de> Deserialize<'de>>(
    params: Option<&Value>,
) -> Result<T, String> {
    serde_json::from_value(params.cloned().unwrap_or(Value::Null)).map_err(|err| err.to_string())
}

pub fn lsp_offset(text: &str, position: &Position) -> Result<usize, String> {
    let mut current_line = 0u32;
    let mut line_start = 0usize;

    for (index, ch) in text.char_indices() {
        if ch == '\n' {
            if current_line == position.line {
                return offset_in_line(text, line_start, index, position.character);
            }
            current_line += 1;
            line_start = index + ch.len_utf8();
        }
    }

    if current_line == position.line {
        return offset_in_line(text, line_start, text.len(), position.character);
    }

    Err(format!(
        "content change line {} is outside the document",
        position.line
    ))
}

fn offset_in_line(
    text: &str,
    line_start: usize,
    line_end: usize,
    character: u32,
) -> Result<usize, String> {
    let line = &text[line_start..line_end];
    let mut utf16_units = 0u32;
    for (index, ch) in line.char_indices() {
        if utf16_units >= character {
            return Ok(line_start + index);
        }
        utf16_units += ch.len_utf16() as u32;
    }
    if utf16_units >= character {
        Ok(line_end)
    } else {
        Err("content change character is outside the line".to_string())
    }
}

pub fn cursor_is_in_comment(text: &str, line: u32, character: u32) -> bool {
    let position = Position { line, character };
    let Ok(offset) = lsp_offset(text, &position) else {
        return false;
    };
    let mut state = CommentScanState::Code;
    let bytes = text.as_bytes();
    let mut index = 0usize;

    while index < offset && index < bytes.len() {
        match state {
            CommentScanState::Code => match bytes[index] {
                b'/' if index + 1 < offset && bytes.get(index + 1) == Some(&b'/') => {
                    state = CommentScanState::Line;
                    index += 2;
                }
                b'/' if index + 1 < offset && bytes.get(index + 1) == Some(&b'*') => {
                    state = CommentScanState::Block;
                    index += 2;
                }
                b'"' => {
                    state = CommentScanState::String;
                    index += 1;
                }
                b'`' => {
                    state = CommentScanState::RawString;
                    index += 1;
                }
                _ => index += 1,
            },
            CommentScanState::Line => {
                if bytes[index] == b'\n' {
                    state = CommentScanState::Code;
                }
                index += 1;
            }
            CommentScanState::Block => {
                if index + 1 < offset && bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/')
                {
                    state = CommentScanState::Code;
                    index += 2;
                } else {
                    index += 1;
                }
            }
            CommentScanState::String => {
                if bytes[index] == b'\\' {
                    index = (index + 2).min(bytes.len());
                } else if bytes[index] == b'"' {
                    state = CommentScanState::Code;
                    index += 1;
                } else {
                    index += 1;
                }
            }
            CommentScanState::RawString => {
                if bytes[index] == b'`' {
                    state = CommentScanState::Code;
                }
                index += 1;
            }
        }
    }

    matches!(state, CommentScanState::Line | CommentScanState::Block)
}

#[derive(Debug, Clone, Copy)]
enum CommentScanState {
    Code,
    Line,
    Block,
    String,
    RawString,
}

pub fn read_message(reader: &mut impl BufRead) -> Result<Option<Value>, String> {
    let mut content_length = None;
    let mut header_started = false;

    loop {
        let mut line = String::new();
        let read = reader.read_line(&mut line).map_err(|err| err.to_string())?;
        if read == 0 {
            return if header_started {
                Err("unexpected EOF while reading LSP headers".to_string())
            } else {
                Ok(None)
            };
        }
        header_started = true;

        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            break;
        }

        if let Some(value) = line.strip_prefix("Content-Length:") {
            content_length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|err| format!("invalid Content-Length: {err}"))?,
            );
        }
    }

    let Some(content_length) = content_length else {
        return Err("missing Content-Length header".to_string());
    };

    let mut body = vec![0; content_length];
    reader
        .read_exact(&mut body)
        .map_err(|err| err.to_string())?;
    serde_json::from_slice(&body).map_err(|err| err.to_string())
}

pub fn write_json(writer: &mut impl Write, value: &Value) -> Result<(), String> {
    let body = serde_json::to_vec(value).map_err(|err| err.to_string())?;
    write!(writer, "Content-Length: {}\r\n\r\n", body.len()).map_err(|err| err.to_string())?;
    writer.write_all(&body).map_err(|err| err.to_string())?;
    writer.flush().map_err(|err| err.to_string())
}

pub fn publish_diagnostics(
    writer: &mut impl Write,
    uri: &str,
    diagnostics: Vec<Value>,
) -> Result<(), String> {
    write_json(
        writer,
        &json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {
                "uri": uri,
                "diagnostics": diagnostics
            }
        }),
    )
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{Position, cursor_is_in_comment, lsp_offset, read_message};

    #[test]
    fn rejects_eof_after_lsp_header_reading_has_started() {
        let mut input = Cursor::new(b"Content-Length: 5\r\n");

        assert_eq!(
            read_message(&mut input).unwrap_err(),
            "unexpected EOF while reading LSP headers"
        );
    }

    #[test]
    fn detects_line_and_block_comments() {
        assert!(cursor_is_in_comment("// comment\nvalue", 0, 5));
        assert!(cursor_is_in_comment("/* comment */\nvalue", 0, 5));
        assert!(cursor_is_in_comment("/* comment\nvalue", 1, 3));
        assert!(!cursor_is_in_comment("// comment\nvalue", 1, 3));
        assert!(!cursor_is_in_comment("value // comment", 0, 2));
    }

    #[test]
    fn does_not_treat_a_single_slash_as_a_comment() {
        assert!(!cursor_is_in_comment("/ value", 0, 1));
    }

    #[test]
    fn converts_utf16_positions_to_byte_offsets() {
        let text = "a😀\nvalue";
        assert_eq!(
            lsp_offset(
                text,
                &Position {
                    line: 0,
                    character: 1,
                }
            )
            .unwrap(),
            "a".len()
        );
        assert_eq!(
            lsp_offset(
                text,
                &Position {
                    line: 1,
                    character: 3,
                }
            )
            .unwrap(),
            "a😀\nval".len()
        );
    }
}
