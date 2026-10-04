use std::fmt::Write;

use super::*;

pub fn format_project(project: &Project) -> String {
    let mut out = String::new();
    if !project.initialization_order.is_empty() {
        let _ = writeln!(
            out,
            "startup: zero_all_globals; initialize {:?}; main.main",
            project.initialization_order
        );
    }
    for package in &project.packages {
        let _ = writeln!(out, "package {} {{", package.logical_path);
        for global in &package.globals {
            let _ = writeln!(
                out,
                "  global {}: {} = zero",
                global.symbol,
                project.types.display(global.ty)
            );
        }
        for item in &package.statics {
            let _ = writeln!(
                out,
                "  static {}: {} = {:?}",
                item.symbol,
                project.types.display(item.value.ty),
                item.value.kind
            );
        }
        if let Some(initializer) = &package.initializer {
            format_function(&mut out, &project.types, initializer);
        }
        for function in &package.functions {
            format_function(&mut out, &project.types, function);
        }
        let _ = writeln!(out, "}}");
    }
    out
}

fn format_function(out: &mut String, types: &TypeTable, function: &Function) {
    let returns = function
        .returns
        .iter()
        .map(|ty| types.display(*ty).to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let _ = writeln!(out, "  func {} -> ({returns}) {{", function.name);
    for block in &function.blocks {
        let parameters = block
            .parameters
            .iter()
            .map(|value| format_value(types, function, *value))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(out, "    bb{}({parameters}):", block.id.0);
        for instruction in &block.instructions {
            let results = instruction
                .results
                .iter()
                .map(|value| format_value(types, function, *value))
                .collect::<Vec<_>>()
                .join(", ");
            if !results.is_empty() {
                let _ = write!(out, "      {results} = ");
            } else {
                let _ = write!(out, "      ");
            }
            let _ = writeln!(out, "{}", format_instruction(&instruction.kind));
        }
        let _ = writeln!(out, "      {}", format_terminator(&block.terminator.kind));
    }
    let _ = writeln!(out, "  }}");
}

fn format_value(types: &TypeTable, function: &Function, value: ValueId) -> String {
    match function.values.get(value.0 as usize) {
        Some(data) => format!("%{}: {}", value.0, types.display(data.ty)),
        None => format!("%{}: <invalid>", value.0),
    }
}

fn format_instruction(kind: &InstructionKind) -> String {
    match kind {
        InstructionKind::Constant(value) => format!("const {value:?}"),
        InstructionKind::Unary { op, operand } => format!("{op:?} %{}", operand.0),
        InstructionKind::Binary {
            op,
            lhs,
            rhs,
            overflow,
        } => {
            format!("{op:?}.{overflow:?} %{}, %{}", lhs.0, rhs.0)
        }
        InstructionKind::Cast {
            value,
            to,
            behavior,
        } => format!("cast.{behavior:?} %{} to {to}", value.0),
        InstructionKind::AddressOf(place) => format!("address_of {place:?}"),
        InstructionKind::Load { place, access } => format!("load.{access:?} {place:?}"),
        InstructionKind::Store {
            place,
            value,
            access,
        } => {
            format!("store.{access:?} {place:?}, %{}", value.0)
        }
        InstructionKind::AggregateCopy {
            destination,
            source,
            access,
        } => {
            format!("aggregate_copy.{access:?} {destination:?}, {source:?}")
        }
        InstructionKind::Check(check) => format!("check {check:?}"),
        InstructionKind::Call {
            callee,
            signature,
            arguments,
            effects,
        } => {
            let arguments = arguments
                .iter()
                .map(|value| format!("%{}", value.0))
                .collect::<Vec<_>>()
                .join(", ");
            format!("call.{effects:?} {callee:?}({arguments}) : {signature:?}")
        }
    }
}

fn format_terminator(kind: &TerminatorKind) -> String {
    match kind {
        TerminatorKind::Jump(edge) => format_edge("jump", edge),
        TerminatorKind::Branch {
            condition,
            then_edge,
            else_edge,
        } => format!(
            "branch %{}, {}, {}",
            condition.0,
            format_edge("then", then_edge),
            format_edge("else", else_edge)
        ),
        TerminatorKind::Return(values) => {
            let values = values
                .iter()
                .map(|value| format!("%{}", value.0))
                .collect::<Vec<_>>()
                .join(", ");
            format!("return {values}")
        }
        TerminatorKind::Trap(kind) => format!("trap {kind:?}"),
        TerminatorKind::Unreachable => "unreachable".to_string(),
    }
}

fn format_edge(prefix: &str, edge: &Edge) -> String {
    let arguments = edge
        .arguments
        .iter()
        .map(|value| format!("%{}", value.0))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{prefix} bb{}({arguments})", edge.target.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_empty_function_deterministically() {
        let project = Project {
            initialization_order: Vec::new(),
            types: TypeTable::new(),
            packages: vec![Package {
                id: PackageId(0),
                imports: Vec::new(),
                logical_path: ".".to_string(),
                globals: Vec::new(),
                statics: Vec::new(),
                initializer: None,
                functions: vec![Function {
                    name: "main.main".to_string(),
                    parameters: Vec::new(),
                    returns: Vec::new(),
                    locals: Vec::new(),
                    values: Vec::new(),
                    blocks: vec![BasicBlock {
                        id: BlockId(0),
                        parameters: Vec::new(),
                        instructions: Vec::new(),
                        terminator: Terminator {
                            kind: TerminatorKind::Return(Vec::new()),
                            span: Span::synthetic(),
                        },
                    }],
                    entry: BlockId(0),
                    span: Span::synthetic(),
                }],
            }],
        };
        assert_eq!(
            format_project(&project),
            "package . {\n  func main.main -> () {\n    bb0():\n      return \n  }\n}\n"
        );
    }
}
