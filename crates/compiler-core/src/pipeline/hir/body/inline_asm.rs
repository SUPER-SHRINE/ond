//! asm headerの通常名解決・型検査と、命令CFGの静的検査。
use super::BodyLowerer;
use crate::{
    inline_asm,
    ir::{ast, hir},
};

impl BodyLowerer<'_> {
    pub(super) fn lower_asm(&mut self, source: &ast::InlineAssembly) -> Option<hir::Stmt> {
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        let errors = inline_asm::validate_source(source);
        let mut valid = errors.is_empty();
        for error in errors {
            self.recovered_diagnostics.push(error);
        }
        for binding in &source.bindings {
            let reg = &binding.register;
            let Some(value) = self.asm_binding_value(binding) else {
                valid = false;
                if let Some(diagnostic) = self.failure.take() {
                    self.recovered_diagnostics.push(diagnostic);
                }
                continue;
            };
            if !inline_asm::scalar(self.types, value.ty) {
                self.recovered_diagnostics.push(crate::Diagnostic::error(
                    binding.value.span(),
                    "asm binding requires a scalar type, not an aggregate",
                ));
                valid = false;
            }
            if binding.direction.input() {
                inputs.push((reg.clone(), value.clone()));
            }
            if binding.direction.output() {
                outputs.push((reg.clone(), value));
            }
        }
        if !valid {
            // 原因診断は上で回収済み。block回復時のgeneric派生診断を抑える。
            self.suppressed_failure = true;
        }
        valid.then(|| hir::Stmt::InlineAssembly {
            noreturn: source.noreturn,
            inputs,
            outputs,
            program: source.program.clone(),
            span: source.span,
        })
    }

    fn asm_binding_value(&mut self, binding: &ast::AsmBinding) -> Option<hir::Expr> {
        if !binding.direction.output() {
            return self.lower_value(&binding.value, None);
        }
        if !matches!(&binding.value, ast::Expr::Name(path) if path.segments.len() == 1 && path.segments[0].name != "_")
        {
            // 型非依存のshape違反はvalidate_sourceがすでに診断している。
            return None;
        }
        if self.read_only_target(&binding.value) {
            return self.fail(binding.value.span(), "asm output cannot write a constant");
        }
        self.lower_target(&binding.value)
    }
}
