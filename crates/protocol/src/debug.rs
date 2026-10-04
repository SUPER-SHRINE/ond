use crate::object::SectionKind;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DebugObject {
    pub package: String,
    pub functions: Vec<ObjectFunctionDebug>,
    pub ranges: Vec<ObjectDebugRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceLocation {
    pub path: String,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ObjectFunctionDebug {
    pub symbol: String,
    pub section: SectionKind,
    pub offset: u32,
    pub size: u32,
    pub frame_size: u32,
    pub return_address_offset: u32,
    pub source: Option<SourceLocation>,
    pub unwind: Vec<ObjectUnwindRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ObjectUnwindRow {
    pub offset: u32,
    pub cfa_sp_offset: u32,
    pub return_address: ReturnAddressLocation,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReturnAddressLocation {
    LinkRegister,
    StackOffset(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ObjectDebugRange {
    pub section: SectionKind,
    pub start: u32,
    pub end: u32,
    pub operation: DebugOperation,
    pub message: Option<String>,
    pub source: Option<SourceLocation>,
    pub function: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DebugOperation {
    Source,
    Call,
    MemoryRead,
    MemoryWrite,
    BoundsCheck,
    DivisionByZero,
    SignedDivisionOverflow,
    InvalidConversion,
    StackOverflow,
    ExplicitTrap,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DebugInfo {
    pub build_id: String,
    pub stack_bottom: u32,
    pub stack_top: u32,
    pub functions: Vec<FunctionDebug>,
    pub ranges: Vec<DebugRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FunctionDebug {
    pub symbol: String,
    pub address: u32,
    pub size: u32,
    pub frame_size: u32,
    pub return_address_offset: u32,
    pub source: Option<SourceLocation>,
    pub unwind: Vec<UnwindRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UnwindRow {
    pub address: u32,
    pub cfa_sp_offset: u32,
    pub return_address: ReturnAddressLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DebugRange {
    pub start: u32,
    pub end: u32,
    pub operation: DebugOperation,
    pub message: Option<String>,
    pub source: Option<SourceLocation>,
    pub function: String,
}
