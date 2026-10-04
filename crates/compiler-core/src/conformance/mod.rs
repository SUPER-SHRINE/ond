//! Test-only spec cases. No production API or external serialization contract.
mod boundaries;
mod cases;
mod constant_contexts;
mod control_headers;
mod diagnostics;
mod evaluation;
mod fixture;
mod flow;
mod name_duplicates;
mod names;
mod numeric;
mod precedence;
mod runner;
mod scope;
#[cfg(test)]
mod tests;

use crate::{diagnostic::Severity, ir::ast, mir};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
    Parse,
    Core,
    /// Execute only the bounded subset supported by memory_test_vm.
    MemoryVm,
}

#[derive(Clone, Copy)]
pub struct SourceFile<'a> {
    pub path: &'static str,
    pub text: &'a str,
}

#[derive(Clone, Copy)]
pub struct Case<'a> {
    /// Unique case ID, e.g. NUM-02.exact-intermediate.
    pub id: &'a str,
    /// Checklist IDs, separate from this case's unique identifier.
    pub specs: &'static [&'static str],
    pub layer: Layer,
    /// Whether the fixture contains the required project manifest.
    pub manifest: bool,
    pub files: &'a [SourceFile<'a>],
    pub expected: Expected<'a>,
}

#[derive(Clone, Copy)]
pub enum Expected<'a> {
    Accept(&'a [Check]),
    /// Exact diagnostic count; matching is independent of diagnostic order.
    Reject(&'a [ExpectedDiagnostic<'a>]),
}

#[derive(Clone, Copy)]
pub enum Scalar {
    Integer(u64), // Signed integers use sign-extended two's-complement bits.
    Float32(u32),
    Bool(bool),
    Bytes(&'static [u8]),
    Nil,
}

#[derive(Clone, Copy)]
pub enum Check {
    MemoryExecution {
        description: &'static str,
        function: &'static str,
        input: &'static [u64],
        expected: &'static [u64],
    },
    Ast {
        description: &'static str,
        verify: fn(&ast::Project) -> Result<(), String>,
    },
    HirConstant {
        package: &'static str,
        name: &'static str,
        ty: &'static str,
        value: Scalar,
    },
    Mir {
        description: &'static str,
        verify: fn(&mir::Project) -> Result<(), String>,
    },
}

#[derive(Clone, Copy)]
pub enum Message {
    Exact(&'static str),
    Contains(&'static str),
}

#[derive(Clone, Copy)]
pub enum Location<'a> {
    /// Zero-width diagnostic at EOF, including an empty source file.
    EndOfFile { file: &'static str },
    /// Exact UTF-8 byte span of the zero-based occurrence of text in file.
    Source {
        file: &'static str,
        text: &'a str,
        occurrence: usize,
    },
    /// Explicitly allow a synthetic span for project-level diagnostics only.
    Project,
}

#[derive(Clone, Copy)]
pub struct ExpectedDiagnostic<'a> {
    pub severity: Severity,
    pub message: Message,
    pub location: Location<'a>,
}

pub use runner::{run, validate_registry};
