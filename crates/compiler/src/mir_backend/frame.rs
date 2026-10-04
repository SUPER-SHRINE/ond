//! Disjoint O0 storage for values, caller-owned arguments and return areas.
use super::*;
use std::collections::BTreeMap;
pub(super) struct CallFrame {
    pub arguments: Vec<Option<u32>>,
    pub returns: u32,
}
pub(super) struct Frame {
    pub size: u32,
    pub locals: Vec<u32>,
    pub values: Vec<u32>,
    pub edges: Vec<u32>,
    pub ra: u32,
    pub incoming: Vec<u32>,
    pub scratch: u32,
    pub calls: BTreeMap<mir::InstructionId, CallFrame>,
}
impl Frame {
    pub fn local(&self, id: mir::LocalId) -> u32 {
        self.locals[id.0 as usize]
    }
    pub fn value(&self, id: mir::ValueId) -> u32 {
        self.values[id.0 as usize]
    }
    pub fn new(
        f: &mir::Function,
        types: &mir::TypeTable,
        abi: &layout::AbiPlan,
    ) -> Result<Self, Diagnostic> {
        let mut cursor = 0;
        for i in f.blocks.iter().flat_map(|b| &b.instructions) {
            if let mir::InstructionKind::Call { signature, .. } = &i.kind {
                cursor = cursor.max(layout::abi(types, signature, i.span)?.stack_bytes);
            }
        }
        fn reserve(c: &mut u32, size: u32, span: Span) -> Result<u32, Diagnostic> {
            *c = layout::align(*c, 4, span)?;
            let at = *c;
            *c = c
                .checked_add(size.max(1))
                .ok_or_else(|| error(span, "frame overflow"))?;
            Ok(at)
        }
        let mut slot = |ty, span| -> Result<u32, Diagnostic> {
            reserve(
                &mut cursor,
                layout::data(types, ty, span)?.size.max(4),
                span,
            )
        };
        let locals = f
            .locals
            .iter()
            .map(|l| slot(l.ty, l.span))
            .collect::<Result<Vec<_>, _>>()?;
        let values = f
            .values
            .iter()
            .map(|v| slot(v.ty, v.span))
            .collect::<Result<Vec<_>, _>>()?;
        let edges = f
            .values
            .iter()
            .map(|v| slot(v.ty, v.span))
            .collect::<Result<Vec<_>, _>>()?;
        let mut scratch_size = 1;
        for i in f.blocks.iter().flat_map(|b| &b.instructions) {
            if let mir::InstructionKind::AggregateCopy { source, .. } = &i.kind {
                scratch_size = scratch_size.max(layout::data(types, source.ty, i.span)?.size);
            }
        }
        let scratch = reserve(&mut cursor, scratch_size, f.span)?;
        let ra = reserve(&mut cursor, 4, f.span)?;
        let incoming = (0..f.parameters.len() + usize::from(abi.hidden.is_some()))
            .map(|_| reserve(&mut cursor, 4, f.span))
            .collect::<Result<Vec<_>, _>>()?;
        let mut calls = BTreeMap::new();
        for i in f.blocks.iter().flat_map(|b| &b.instructions) {
            if let mir::InstructionKind::Call { signature, .. } = &i.kind {
                let a = layout::abi(types, signature, i.span)?;
                let mut arguments = vec![];
                for ty in &signature.parameters {
                    arguments.push(if layout::aggregate(types, *ty) {
                        Some(reserve(
                            &mut cursor,
                            layout::data(types, *ty, i.span)?.size,
                            i.span,
                        )?)
                    } else {
                        None
                    });
                }
                let returns = reserve(&mut cursor, a.return_bytes, i.span)?;
                calls.insert(i.id, CallFrame { arguments, returns });
            }
        }
        let size = layout::align(cursor, 8, f.span)?;
        if size.checked_add(abi.stack_bytes).is_none() {
            return Err(error(
                f.span,
                "frame/incoming arguments exceed 32-bit address range",
            ));
        }
        Ok(Self {
            size,
            locals,
            values,
            edges,
            ra,
            incoming,
            scratch,
            calls,
        })
    }
}
