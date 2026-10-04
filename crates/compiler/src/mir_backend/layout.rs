//! Natural target layout and Direct/Indirect ABI plans; unsupported types fail closed.
use super::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScalarLayout {
    pub size: u32,
    pub alignment: u32,
    pub signed: bool,
    pub boolean: bool,
}
pub fn scalar(
    types: &mir::TypeTable,
    ty: mir::TypeId,
    span: Span,
) -> Result<ScalarLayout, Diagnostic> {
    use mir::TypeKind::*;
    let (size, signed, boolean) = match types.underlying_kind(ty) {
        Some(Bool) => (1, false, true),
        Some(U8) => (1, false, false),
        Some(I8) => (1, true, false),
        Some(U16) => (2, false, false),
        Some(I16) => (2, true, false),
        Some(U32) => (4, false, false),
        Some(I32) => (4, true, false),
        Some(F32) => (4, false, false),
        Some(Pointer(_) | Function(_)) => (4, false, false),
        _ => {
            return Err(error(
                span,
                format!("unsupported scalar type {}", types.display(ty)),
            ));
        }
    };
    Ok(ScalarLayout {
        size,
        alignment: size,
        signed,
        boolean,
    })
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Argument {
    Register(u8),
    Stack(u32),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbiPlan {
    pub arguments: Vec<Argument>,
    pub stack_bytes: u32,
    pub result: Option<u8>,
    pub hidden: Option<Argument>,
    pub return_offsets: Vec<u32>,
    pub return_bytes: u32,
}
pub fn abi(
    types: &mir::TypeTable,
    signature: &mir::FunctionType,
    span: Span,
) -> Result<AbiPlan, Diagnostic> {
    for ty in signature.parameters.iter().chain(&signature.returns) {
        data(types, *ty, span)?;
    }
    let memory =
        signature.returns.len() > 4 || signature.returns.iter().any(|t| aggregate(types, *t));
    let mut return_offsets = Vec::new();
    let mut return_bytes = 0;
    if memory {
        for ty in &signature.returns {
            let d = data(types, *ty, span)?;
            return_bytes = align(return_bytes, d.alignment, span)?;
            return_offsets.push(return_bytes);
            return_bytes = return_bytes
                .checked_add(if aggregate(types, *ty) { d.size } else { 4 })
                .ok_or_else(|| error(span, "return area overflow"))?;
        }
        return_bytes = align(return_bytes, 4, span)?;
    }
    let word = |i: usize| {
        if i < 6 {
            Argument::Register(i as u8 + 1)
        } else {
            Argument::Stack((i as u32 - 6) * 4)
        }
    };
    let extra = (signature.parameters.len() + usize::from(memory)).saturating_sub(6);
    let bytes = u32::try_from(extra)
        .ok()
        .and_then(|n| n.checked_mul(4))
        .and_then(|n| n.checked_add(7))
        .map(|n| n & !7)
        .ok_or_else(|| error(span, "argument area too large"))?;
    Ok(AbiPlan {
        arguments: (0..signature.parameters.len())
            .map(|i| word(i + usize::from(memory)))
            .collect(),
        stack_bytes: bytes,
        result: (!memory && !signature.returns.is_empty()).then_some(1),
        hidden: memory.then_some(word(0)),
        return_offsets,
        return_bytes,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataLayout {
    pub size: u32,
    pub alignment: u32,
    pub fields: Vec<u32>,
}
pub fn aggregate(types: &mir::TypeTable, ty: mir::TypeId) -> bool {
    matches!(
        types.underlying_kind(ty),
        Some(mir::TypeKind::Array { .. } | mir::TypeKind::Struct(_) | mir::TypeKind::Interface(_))
    )
}
pub(super) fn align(n: u32, a: u32, span: Span) -> Result<u32, Diagnostic> {
    n.checked_add(a - 1)
        .map(|n| n & !(a - 1))
        .ok_or_else(|| error(span, "layout overflow"))
}
pub fn data(types: &mir::TypeTable, ty: mir::TypeId, span: Span) -> Result<DataLayout, Diagnostic> {
    fn visit(
        types: &mir::TypeTable,
        ty: mir::TypeId,
        span: Span,
        depth: usize,
    ) -> Result<DataLayout, Diagnostic> {
        if depth > types.definitions().len() {
            return Err(error(span, "recursive value layout"));
        }
        let mut d = DataLayout {
            size: 0,
            alignment: 1,
            fields: vec![],
        };
        match types.underlying_kind(ty) {
            Some(mir::TypeKind::Array { length, element }) => {
                let e = visit(types, *element, span, depth + 1)?;
                d.size = e
                    .size
                    .checked_mul(*length)
                    .ok_or_else(|| error(span, "array layout overflow"))?;
                d.alignment = e.alignment;
            }
            Some(mir::TypeKind::Struct(s)) => {
                for f in &s.fields {
                    let e = visit(types, f.ty, span, depth + 1)?;
                    d.size = align(d.size, e.alignment, span)?;
                    d.fields.push(d.size);
                    d.size = d
                        .size
                        .checked_add(e.size)
                        .ok_or_else(|| error(span, "struct layout overflow"))?;
                    d.alignment = d.alignment.max(e.alignment);
                }
                d.size = align(d.size, d.alignment, span)?;
            }
            Some(mir::TypeKind::Interface(interface)) => {
                for ty in [interface.data_pointer, interface.vtable] {
                    let e = visit(types, ty, span, depth + 1)?;
                    d.size = align(d.size, e.alignment, span)?;
                    d.fields.push(d.size);
                    d.size = d
                        .size
                        .checked_add(e.size)
                        .ok_or_else(|| error(span, "interface layout overflow"))?;
                    d.alignment = d.alignment.max(e.alignment);
                }
                d.size = align(d.size, d.alignment, span)?;
            }
            _ => {
                let s = scalar(types, ty, span)?;
                d.size = s.size;
                d.alignment = s.alignment;
            }
        }
        Ok(d)
    }
    visit(types, ty, span, 0)
}

/// Scalar leaves in declaration/index order; never accesses padding.
pub(super) fn leaves(
    types: &mir::TypeTable,
    ty: mir::TypeId,
    span: Span,
) -> Result<Vec<(u32, ScalarLayout)>, Diagnostic> {
    fn walk(
        types: &mir::TypeTable,
        ty: mir::TypeId,
        offset: u32,
        span: Span,
        out: &mut Vec<(u32, ScalarLayout)>,
    ) -> Result<(), Diagnostic> {
        let d = data(types, ty, span)?;
        if d.size == 0 {
            return Ok(());
        }
        match types.underlying_kind(ty) {
            Some(mir::TypeKind::Array { length, element }) => {
                let stride = data(types, *element, span)?.size;
                for i in 0..*length {
                    walk(types, *element, offset + i * stride, span, out)?;
                }
            }
            Some(mir::TypeKind::Struct(s)) => {
                for (f, o) in s.fields.iter().zip(d.fields) {
                    walk(types, f.ty, offset + o, span, out)?;
                }
            }
            Some(mir::TypeKind::Interface(interface)) => {
                for (ty, field_offset) in [interface.data_pointer, interface.vtable]
                    .into_iter()
                    .zip(d.fields)
                {
                    walk(types, ty, offset + field_offset, span, out)?;
                }
            }
            _ => out.push((offset, scalar(types, ty, span)?)),
        }
        Ok(())
    }
    let mut out = vec![];
    walk(types, ty, 0, span, &mut out)?;
    Ok(out)
}
