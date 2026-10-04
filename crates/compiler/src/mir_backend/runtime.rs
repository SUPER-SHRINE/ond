//! Compiler-owned Ond helpers compiled by compiler-core and the MIR backend.
//! Type tables stay separate: the boundary is an explicitly checked scalar ABI.
use super::*;
use std::collections::BTreeSet;
mod library;
#[cfg(test)]
mod tests;
const PREFIX: &str = "\u{1}ond.runtime.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Helper {
    F32Add,
    F32Sub,
    F32Mul,
    F32Div,
    F32Eq,
    F32Ne,
    F32Lt,
    F32Le,
    F32Gt,
    F32Ge,
    F32FromU32,
    F32FromI32,
    F32ToU32,
    F32ToI32,
    F32Neg,
    UDiv,
    URem,
    IDiv,
    IRem,
    Alloc,
    Free,
    New,
}
impl Helper {
    pub const ALL: [Self; 22] = [
        Self::F32Add,
        Self::F32Sub,
        Self::F32Mul,
        Self::F32Div,
        Self::F32Eq,
        Self::F32Ne,
        Self::F32Lt,
        Self::F32Le,
        Self::F32Gt,
        Self::F32Ge,
        Self::F32FromU32,
        Self::F32FromI32,
        Self::F32ToU32,
        Self::F32ToI32,
        Self::F32Neg,
        Self::UDiv,
        Self::URem,
        Self::IDiv,
        Self::IRem,
        Self::Alloc,
        Self::Free,
        Self::New,
    ];
    fn name(self) -> &'static str {
        match self {
            Self::F32Add => "__ond_f32_add",
            Self::F32Sub => "__ond_f32_sub",
            Self::F32Mul => "__ond_f32_mul",
            Self::F32Div => "__ond_f32_div",
            Self::F32Eq => "__ond_f32_eq",
            Self::F32Ne => "__ond_f32_ne",
            Self::F32Lt => "__ond_f32_lt",
            Self::F32Le => "__ond_f32_le",
            Self::F32Gt => "__ond_f32_gt",
            Self::F32Ge => "__ond_f32_ge",
            Self::F32FromU32 => "__ond_f32_from_u32",
            Self::F32FromI32 => "__ond_f32_from_i32",
            Self::F32ToU32 => "__ond_f32_to_u32",
            Self::F32ToI32 => "__ond_f32_to_i32",
            Self::F32Neg => "__ond_f32_neg",
            Self::UDiv => "__ond_udiv",
            Self::URem => "__ond_urem",
            Self::IDiv => "__ond_idiv",
            Self::IRem => "__ond_irem",
            Self::Alloc => "__ond_alloc",
            Self::Free => "__ond_free",
            Self::New => "__ond_new",
        }
    }
    /// Reserved linker symbol, not an Ond source-language name.
    pub fn symbol(self) -> String {
        format!("{PREFIX}{}", self.name())
    }
    /// Soft-float arguments/results are bit patterns, not numeric casts.
    pub fn signature(self) -> mir::FunctionType {
        use mir::TypeId as T;
        match self {
            Self::IDiv | Self::IRem => {
                return mir::FunctionType {
                    parameters: vec![T::I32; 2],
                    returns: vec![T::U32],
                };
            }
            Self::Alloc | Self::New => {
                return mir::FunctionType {
                    parameters: vec![T::U32; 4],
                    returns: vec![T::U32],
                };
            }
            Self::Free => {
                return mir::FunctionType {
                    parameters: vec![T::U32; 3],
                    returns: vec![],
                };
            }
            _ => {}
        }
        let unary = matches!(
            self,
            Self::F32FromU32 | Self::F32FromI32 | Self::F32ToU32 | Self::F32ToI32 | Self::F32Neg
        );
        mir::FunctionType {
            parameters: if self == Self::F32FromI32 {
                vec![T::I32]
            } else {
                vec![T::U32; if unary { 1 } else { 2 }]
            },
            returns: vec![T::U32],
        }
    }
}
fn failure(message: impl Into<String>) -> FrontendError {
    FrontendError::new(vec![error(
        Span::synthetic(),
        format!("runtime helper: {}", message.into()),
    )])
}

/// Emit the deduplicated transitive helper closure, without user AST or disk files.
pub fn objects(helpers: &[Helper]) -> Result<Vec<Object>, FrontendError> {
    Ok(objects_with_debug(helpers)?.objects)
}

fn objects_with_debug(helpers: &[Helper]) -> Result<CodegenOutput, FrontendError> {
    if helpers.is_empty() {
        return Ok(CodegenOutput {
            objects: vec![],
            debug: vec![],
        });
    }
    let mut project = library::load(concat!(
        include_str!("../runtime_support.ond"),
        "\n",
        include_str!("runtime/integer.ond"),
        "\n",
        include_str!("runtime/heap.ond")
    ))?;
    for h in helpers {
        let name = format!("runtime.{}", h.name());
        let f = project.packages[0]
            .functions
            .iter()
            .find(|f| f.name == name)
            .ok_or_else(|| failure(format!("missing entry {name}")))?;
        let signature = mir::FunctionType {
            parameters: f
                .parameters
                .iter()
                .map(|id| f.locals[id.0 as usize].ty)
                .collect(),
            returns: f.returns.clone(),
        };
        if signature != h.signature() {
            return Err(failure(format!("ABI mismatch for {name}")));
        }
    }
    library::select(
        &mut project,
        &helpers
            .iter()
            .map(|h| format!("runtime.{}", h.name()))
            .collect::<Vec<_>>(),
    )?;
    let emitted = codegen_raw(&project).map_err(|e| {
        failure(format!(
            "MIR emission failed (unsupported primitive or lowering dependency): {e}"
        ))
    })?;
    let defined = emitted
        .objects
        .iter()
        .flat_map(|o| &o.symbols)
        .map(|s| s.name.as_str())
        .collect::<BTreeSet<_>>();
    for r in emitted.objects.iter().flat_map(|o| &o.relocations) {
        if !defined.contains(r.target.as_str()) && r.target != "__ond_stack_bottom" {
            return Err(failure(format!(
                "unresolved lowering dependency {}; runtime supply is non-recursive",
                r.target
            )));
        }
    }
    Ok(emitted)
}

/// Resolve requests produced by backend call/address relocations exactly once.
pub fn supply(inputs: Vec<Object>) -> Result<Vec<Object>, FrontendError> {
    Ok(supply_with_debug(inputs, Vec::new())?.objects)
}

pub fn supply_with_debug(
    mut inputs: Vec<Object>,
    mut debug: Vec<DebugObject>,
) -> Result<CodegenOutput, FrontendError> {
    for object in &inputs {
        if object.symbols.iter().any(|s| s.name.starts_with(PREFIX)) {
            return Err(failure("user object defines reserved runtime symbol"));
        }
    }
    let mut requested = BTreeSet::new();
    for r in inputs
        .iter()
        .flat_map(|o| &o.relocations)
        .filter(|r| r.target.starts_with(PREFIX))
    {
        let helper = Helper::ALL
            .into_iter()
            .find(|h| h.symbol() == r.target)
            .ok_or_else(|| failure(format!("unknown request {}", r.target)))?;
        requested.insert(helper);
    }
    let emitted = objects_with_debug(&requested.into_iter().collect::<Vec<_>>())?;
    inputs.extend(emitted.objects);
    debug.extend(emitted.debug);
    Ok(CodegenOutput {
        objects: inputs,
        debug,
    })
}
