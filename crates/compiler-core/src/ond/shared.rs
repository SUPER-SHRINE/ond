use std::iter::Peekable;
use std::mem;

use crate::diagnostic::{Diagnostic, Diagnostics, FrontendError};
use crate::ir::ast;
use crate::project::LoadedProject;
use crate::source::{FileId, Span};

#[derive(Debug, Clone)]
struct Token {
    kind: TokenKind,
    text: String,
    span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenKind {
    Identifier,
    IntLit,
    FloatLit,
    StringLit,
    Package,
    Import,
    Const,
    Var,
    Type,
    Func,
    Struct,
    Interface,
    Return,
    Defer,
    Break,
    Continue,
    If,
    Else,
    For,
    True,
    False,
    Nil,
    As,
    New,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Colon,
    Semi,
    Dot,
    Ellipsis,
    Assign,
    ColonAssign,
    PlusAssign,
    MinusAssign,
    StarAssign,
    SlashAssign,
    PercentAssign,
    AmpAssign,
    PipeAssign,
    CaretAssign,
    AmpCaretAssign,
    ShiftLeftAssign,
    ShiftRightAssign,
    Increment,
    Decrement,
    Arrow,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Amp,
    Pipe,
    Caret,
    AmpCaret,
    ShiftLeft,
    ShiftRight,
    Bang,
    EqEq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
    AndAnd,
    OrOr,
    Eof,
}

pub fn parse_project(project: &LoadedProject) -> Result<ast::Project, FrontendError> {
    let mut diagnostics = Diagnostics::new();
    let mut packages = Vec::new();

    for package in &project.packages {
        let mut files = Vec::new();
        for &file_id in &package.files {
            let file = project.sources.file(file_id);
            match parse_file(file_id, file.text()) {
                Ok(file_ast) => files.push(file_ast),
                Err(err) => diagnostics.extend(err),
            }
        }
        packages.push(ast::Package {
            logical_path: package.logical_path.clone(),
            files,
        });
    }

    if diagnostics.is_empty() {
        Ok(ast::Project { packages })
    } else {
        Err(FrontendError::new(diagnostics.into_vec()))
    }
}

fn parse_file(file_id: FileId, text: &str) -> Result<ast::File, Diagnostics> {
    let tokens = tokenize(file_id, text)?;
    Parser::new(file_id, tokens).parse_file()
}

pub fn parse_source_file(file_id: FileId, text: &str) -> Result<ast::File, Diagnostics> {
    parse_file(file_id, text)
}

fn tokenize(file_id: FileId, text: &str) -> Result<Vec<Token>, Diagnostics> {
    let mut lexer = Lexer::new(file_id, text);
    lexer.lex()
}
