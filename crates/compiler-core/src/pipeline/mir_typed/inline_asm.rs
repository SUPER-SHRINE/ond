//! 全inputをsnapshot化し、全outputを得てから通常代入へ戻す。
use super::*;
use crate::inline_asm::{Program, Register};
impl FunctionBuilder<'_> {
    pub(super) fn lower_asm(
        &mut self,
        noreturn: bool,
        inputs: &[(Register, hir::Expr)],
        outputs: &[(Register, hir::Expr)],
        program: &Program,
        span: Span,
    ) -> Option<()> {
        let inputs = inputs
            .iter()
            .map(|(reg, expr)| Some((reg.clone(), self.lower_expr(expr)?)))
            .collect::<Option<Vec<_>>>()?;
        let result_ids = outputs
            .iter()
            .map(|(_, expr)| {
                let id = mir::ValueId(self.values.len() as u32);
                self.values.push(mir::Value {
                    id,
                    ty: expr.ty,
                    span: expr.span,
                });
                id
            })
            .collect::<Vec<_>>();
        self.emit(
            result_ids.clone(),
            mir::InstructionKind::InlineAssembly {
                program: program.clone(),
                noreturn,
                inputs,
                outputs: outputs
                    .iter()
                    .map(|(reg, expr)| (reg.clone(), expr.ty))
                    .collect(),
            },
            span,
        );
        if noreturn {
            self.terminate(mir::TerminatorKind::Unreachable, span);
        } else {
            for ((_, expr), id) in outputs.iter().zip(result_ids) {
                let place = self.lower_place(expr)?;
                self.store_value(place, id, expr.span);
            }
        }
        Some(())
    }
}
