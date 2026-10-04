//! AST reachability checks used before typed lowering.
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::ir::ast;

pub(super) fn return_diagnostics(project: &ast::Project) -> Diagnostics {
    let mut diagnostics = Diagnostics::new();
    for package in &project.packages {
        for file in &package.files {
            for declaration in &file.decls {
                let ast::TopLevelDecl::Func(function) = declaration else {
                    continue;
                };
                let expected = function.signature.results.len();
                if expected != 0 && block_can_complete_normally(&function.body) {
                    diagnostics.push(Diagnostic::error(
                        function.body.span,
                        format!("missing return in function `{}`", function.name.name),
                    ));
                }
                return_count_diagnostics(&function.body, expected, &mut diagnostics);
            }
        }
    }
    diagnostics
}

fn return_count_diagnostics(block: &ast::Block, expected: usize, diagnostics: &mut Diagnostics) {
    for statement in &block.statements {
        return_count_diagnostics_in_statement(statement, expected, diagnostics);
    }
}

fn return_count_diagnostics_in_statement(
    statement: &ast::Stmt,
    expected: usize,
    diagnostics: &mut Diagnostics,
) {
    match statement {
        ast::Stmt::Return(statement) => {
            let sole_call_can_expand =
                matches!(statement.values.as_slice(), [ast::Expr::Call { .. }]);
            if statement.values.len() != expected && !sole_call_can_expand {
                diagnostics.push(Diagnostic::error(
                    statement.span,
                    format!(
                        "return value count mismatch: expected {expected}, found {}",
                        statement.values.len()
                    ),
                ));
            }
        }
        ast::Stmt::Block(block) => return_count_diagnostics(block, expected, diagnostics),
        ast::Stmt::If(statement) => {
            if let Some(init) = statement.init.as_deref() {
                return_count_diagnostics_in_statement(init, expected, diagnostics);
            }
            return_count_diagnostics(&statement.then_block, expected, diagnostics);
            if let Some(branch) = statement.else_branch.as_deref() {
                return_count_diagnostics_in_statement(branch, expected, diagnostics);
            }
        }
        ast::Stmt::For(statement) => {
            if let ast::ForKind::ThreeClause { init, post, .. } = &statement.kind {
                if let Some(init) = init.as_deref() {
                    return_count_diagnostics_in_statement(init, expected, diagnostics);
                }
                if let Some(post) = post.as_deref() {
                    return_count_diagnostics_in_statement(post, expected, diagnostics);
                }
            }
            return_count_diagnostics(&statement.body, expected, diagnostics);
        }
        ast::Stmt::Defer(_) | ast::Stmt::Trap { .. } => {}
        ast::Stmt::Const(_)
        | ast::Stmt::Var(_)
        | ast::Stmt::ShortVar(_)
        | ast::Stmt::Assign(_)
        | ast::Stmt::Expr(_)
        | ast::Stmt::Break(_)
        | ast::Stmt::Continue(_) => {}
    }
}

pub(super) fn block_can_complete_normally(block: &ast::Block) -> bool {
    let mut reachable = true;
    for stmt in &block.statements {
        if !reachable {
            return false;
        }
        reachable = stmt_can_complete_normally(stmt);
    }
    reachable
}

fn stmt_can_complete_normally(stmt: &ast::Stmt) -> bool {
    match stmt {
        ast::Stmt::Var(_) | ast::Stmt::Assign(_) | ast::Stmt::Const(_) | ast::Stmt::ShortVar(_) => {
            true
        }
        ast::Stmt::Expr(_) | ast::Stmt::Defer(_) => true,
        ast::Stmt::Trap { .. } => false,
        ast::Stmt::Return(_) | ast::Stmt::Break(_) | ast::Stmt::Continue(_) => false,
        ast::Stmt::Block(block) => block_can_complete_normally(block),
        ast::Stmt::If(stmt) => {
            let then_normal = block_can_complete_normally(&stmt.then_block);
            let else_normal = stmt
                .else_branch
                .as_deref()
                .map(stmt_can_complete_normally)
                .unwrap_or(true);
            then_normal || else_normal
        }
        ast::Stmt::For(stmt) => match &stmt.kind {
            ast::ForKind::Infinite => loop_body_can_break(&stmt.body),
            ast::ForKind::While(_) => true,
            ast::ForKind::ThreeClause { condition, .. } => {
                if condition.is_some() {
                    true
                } else {
                    loop_body_can_break(&stmt.body)
                }
            }
        },
    }
}

fn loop_body_can_break(block: &ast::Block) -> bool {
    let mut reachable = true;
    for stmt in &block.statements {
        if !reachable {
            return false;
        }
        if stmt_can_break_current_loop(stmt) {
            return true;
        }
        reachable = stmt_can_complete_normally(stmt);
    }
    false
}

fn stmt_can_break_current_loop(stmt: &ast::Stmt) -> bool {
    match stmt {
        ast::Stmt::Break(_) => true,
        ast::Stmt::Block(block) => loop_body_can_break(block),
        ast::Stmt::If(stmt) => {
            loop_body_can_break(&stmt.then_block)
                || stmt
                    .else_branch
                    .as_deref()
                    .is_some_and(stmt_can_break_current_loop)
        }
        ast::Stmt::Var(_)
        | ast::Stmt::Assign(_)
        | ast::Stmt::Expr(_)
        | ast::Stmt::Return(_)
        | ast::Stmt::Continue(_)
        | ast::Stmt::Const(_)
        | ast::Stmt::ShortVar(_)
        | ast::Stmt::Defer(_)
        | ast::Stmt::Trap { .. }
        | ast::Stmt::For(_) => false,
    }
}
