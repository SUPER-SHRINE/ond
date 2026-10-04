use std::fmt;

use crate::source::{SourceDb, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    pub span: Span,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub primary: Label,
    pub related: Vec<Label>,
    pub notes: Vec<String>,
}

impl Diagnostic {
    pub fn error(span: Span, message: impl Into<String>) -> Self {
        let message = message.into();
        Self {
            severity: Severity::Error,
            primary: Label {
                span,
                message: message.clone(),
            },
            message,
            related: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn with_related(mut self, span: Span, message: impl Into<String>) -> Self {
        self.related.push(Label {
            span,
            message: message.into(),
        });
        self
    }

    pub fn reanchor(mut self, span: Span, related_message: impl Into<String>) -> Self {
        if self.primary.span != span && !self.primary.span.is_synthetic() {
            let original = std::mem::replace(
                &mut self.primary,
                Label {
                    span,
                    message: self.message.clone(),
                },
            );
            self.related.push(Label {
                span: original.span,
                message: related_message.into(),
            });
        }
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn render(&self, sources: &SourceDb) -> String {
        if self.primary.span.is_synthetic() {
            return self.message.clone();
        }

        let file = sources.file(self.primary.span.file);
        let (line, col) = file.line_col(self.primary.span.start);
        let mut rendered = format!(
            "{}:{}:{}: error: {}",
            file.path().display(),
            line,
            col,
            self.message
        );
        for note in &self.notes {
            rendered.push_str(&format!("\nnote: {note}"));
        }
        for related in &self.related {
            if related.span.is_synthetic() {
                rendered.push_str(&format!("\nnote: {}", related.message));
                continue;
            }
            let related_file = sources.file(related.span.file);
            let (related_line, related_col) = related_file.line_col(related.span.start);
            rendered.push_str(&format!(
                "\n{}:{}:{}: note: {}",
                related_file.path().display(),
                related_line,
                related_col,
                related.message
            ));
        }
        rendered
    }
}

#[derive(Debug, Clone, Default)]
pub struct Diagnostics {
    items: Vec<Diagnostic>,
}

impl Diagnostics {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.items.push(diagnostic);
    }

    pub fn extend(&mut self, diagnostics: Self) {
        self.items.extend(diagnostics.items);
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn into_vec(self) -> Vec<Diagnostic> {
        self.items
    }
}

impl IntoIterator for Diagnostics {
    type Item = Diagnostic;
    type IntoIter = std::vec::IntoIter<Diagnostic>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

#[derive(Debug, Clone)]
pub struct FrontendError {
    diagnostics: Vec<Diagnostic>,
}

impl FrontendError {
    pub fn new(diagnostics: Vec<Diagnostic>) -> Self {
        Self { diagnostics }
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

impl fmt::Display for FrontendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(first) = self.diagnostics.first() {
            write!(f, "{}", first.message)
        } else {
            write!(f, "frontend error")
        }
    }
}

impl std::error::Error for FrontendError {}
