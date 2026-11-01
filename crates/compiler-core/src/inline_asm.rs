//! Kagura v1の構造化asm命令と、frontend/MIRが共有する静的検査。
use crate::{
    diagnostic::Diagnostic,
    semantic::{TypeId, TypeKind, TypeTable},
    source::Span,
};
use std::collections::{BTreeMap, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Input,
    Output,
    InOut,
}
impl Direction {
    pub fn input(self) -> bool {
        self != Self::Output
    }
    pub fn output(self) -> bool {
        self != Self::Input
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Register {
    pub number: u8,
    pub span: Span,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    Machine {
        opcode: u8,
        rd: Register,
        rs1: Register,
        rs2: Register,
        immediate: i16,
        immediate_span: Span,
    },
    LocalBranch {
        condition: Register,
        target: String,
        target_span: Span,
    },
    ExternalBranch {
        target: Register,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    pub operation: Operation,
    pub span: Span,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    pub name: String,
    pub position: usize,
    pub span: Span,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub instructions: Vec<Instruction>,
    pub labels: Vec<Label>,
    pub end: Span,
}

pub fn scalar(types: &TypeTable, ty: TypeId) -> bool {
    matches!(
        types.underlying_kind(ty),
        Some(
            TypeKind::U8
                | TypeKind::I8
                | TypeKind::U16
                | TypeKind::I16
                | TypeKind::U32
                | TypeKind::I32
                | TypeKind::Bool
                | TypeKind::F32
                | TypeKind::Pointer(_)
                | TypeKind::Function(_)
        )
    )
}

/// bindingsにはinput/outputを別々に渡す。同じinout registerは各列に1回現れる。
pub fn validate(
    program: &Program,
    noreturn: bool,
    inputs: &[Register],
    outputs: &[Register],
) -> Vec<Diagnostic> {
    let mut errors = Vec::new();
    let mut report = |span, message: &str| errors.push(Diagnostic::error(span, message));
    let limit = if noreturn { 15 } else { 13 };
    for list in [inputs, outputs] {
        let mut used = 0u16;
        for reg in list {
            if reg.number == 0 || reg.number > limit {
                report(reg.span, "asm binding register is reserved or out of range");
            } else if used & (1 << reg.number) != 0 {
                report(reg.span, "duplicate asm register binding");
            } else {
                used |= 1 << reg.number;
            }
        }
    }
    if noreturn {
        for reg in outputs {
            report(reg.span, "asm noreturn cannot have output bindings");
        }
    }
    let n = program.instructions.len();
    let mut labels = BTreeMap::new();
    for label in &program.labels {
        if label.position > n {
            report(label.span, "asm label position is outside the block");
        }
        if labels.insert(label.name.as_str(), label.position).is_some() {
            report(label.span, "duplicate asm label");
        }
    }
    let mut successors = vec![Vec::new(); n];
    let mut reads = vec![Vec::new(); n];
    let mut writes = vec![0u16; n];
    for (index, instruction) in program.instructions.iter().enumerate() {
        let mut registers = Vec::new();
        match &instruction.operation {
            Operation::Machine {
                opcode,
                rd,
                rs1,
                rs2,
                immediate,
                immediate_span,
            } => {
                registers.extend([rd, rs1, rs2]);
                if !matches!(opcode, 0..=5 | 7..=9 | 11 | 13) {
                    report(instruction.span, "invalid asm opcode");
                }
                if *opcode == 13 && !matches!(immediate, 0 | 1 | 2 | 4 | 5 | 6) {
                    report(
                        *immediate_span,
                        "CMP immediate has reserved comparison encoding",
                    );
                }
                if *opcode == 2 && !matches!(immediate, 0..=31 | -32768..=-32737 | -16384..=-16353)
                {
                    report(*immediate_span, "SHIFT immediate has reserved encoding");
                }
                reads[index].extend([rs1.clone(), rs2.clone()]);
                if matches!(opcode, 8 | 9 | 11) {
                    reads[index].push(rd.clone());
                } else if rd.number < 16 {
                    writes[index] = 1 << rd.number;
                }
                successors[index].push(index + 1);
            }
            Operation::LocalBranch {
                condition,
                target,
                target_span,
            } => {
                registers.push(condition);
                reads[index].push(condition.clone());
                if condition.number != 0 {
                    successors[index].push(index + 1);
                }
                if let Some(&position) = labels.get(target.as_str()) {
                    if i16::try_from(position as i128 - index as i128 - 1).is_err() {
                        report(
                            *target_span,
                            "asm local branch offset exceeds signed 16-bit range",
                        );
                    }
                    if position <= n {
                        successors[index].push(position);
                    }
                } else {
                    report(*target_span, "undefined asm label");
                }
            }
            Operation::ExternalBranch { target } => {
                registers.push(target);
                reads[index].push(target.clone());
                if !noreturn {
                    report(
                        instruction.span,
                        "external indirect jump requires asm noreturn",
                    );
                }
                if target.number == 0 {
                    report(target.span, "external branch requires r1 through r15");
                }
            }
        }
        for register in registers {
            if register.number > limit {
                report(
                    register.span,
                    "normal asm cannot use r14/r15 or out-of-range registers",
                );
            }
        }
    }
    // 各経路の定義集合の積集合。entryはloop backedgeから定義を借りない。
    let mut initial = 1u16;
    if noreturn {
        initial |= 1 << 14;
    }
    for reg in inputs {
        if reg.number < 16 {
            initial |= 1 << reg.number;
        }
    }
    let mut incoming = vec![None; n + 1];
    incoming[0] = Some(initial);
    let mut queue = VecDeque::from([0]);
    while let Some(index) = queue.pop_front() {
        if index == n {
            continue;
        }
        let after = incoming[index].unwrap() | writes[index];
        for &next in &successors[index] {
            let joined = incoming[next].map_or(after, |prior| prior & after);
            if incoming[next] != Some(joined) {
                incoming[next] = Some(joined);
                queue.push_back(next);
            }
        }
    }
    for (index, required) in reads.iter().enumerate() {
        if let Some(defined) = incoming[index] {
            for reg in required {
                if reg.number < 16 && defined & (1 << reg.number) == 0 {
                    report(
                        reg.span,
                        "asm register is undefined on a path before this read",
                    );
                }
            }
        }
    }
    if let Some(defined) = incoming[n] {
        if noreturn {
            report(
                program.end,
                "asm noreturn has a fallthrough path to block end",
            );
        }
        for reg in outputs {
            if reg.number < 16 && defined & (1 << reg.number) == 0 {
                report(
                    reg.span,
                    "asm output register is not defined on every exit path",
                );
            }
        }
    }
    errors
}

/// Genericの未特殊化bodyにも適用できる、型引数に依存しないsource条件。
pub(crate) fn validate_source(source: &crate::ir::ast::InlineAssembly) -> Vec<Diagnostic> {
    use crate::ir::ast;
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();
    let mut registers = std::collections::BTreeSet::new();
    let mut destinations = std::collections::BTreeSet::new();
    let mut errors = Vec::new();
    for binding in &source.bindings {
        if !registers.insert(binding.register.number) {
            errors.push(Diagnostic::error(
                binding.register.span,
                "duplicate asm register binding",
            ));
        }
        if binding.direction.input() {
            inputs.push(binding.register.clone());
        }
        if binding.direction.output() {
            outputs.push(binding.register.clone());
            match &binding.value {
                ast::Expr::Name(path)
                    if path.segments.len() == 1 && path.segments[0].name != "_" =>
                {
                    if !destinations.insert(path.segments[0].name.as_str()) {
                        errors.push(Diagnostic::error(
                            binding.value.span(),
                            "duplicate asm output variable",
                        ));
                    }
                }
                _ => errors.push(Diagnostic::error(
                    binding.value.span(),
                    "asm output must be a writable scalar variable identifier",
                )),
            }
        }
    }
    errors.extend(validate(
        &source.program,
        source.noreturn,
        &inputs,
        &outputs,
    ));
    let mut seen = std::collections::BTreeSet::new();
    errors.retain(|error| {
        let span = error.primary.span;
        seen.insert((span.file, span.start, span.end, error.message.clone()))
    });
    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_memory_opcodes_validate_and_stores_read_their_data_register() {
        let span = Span::synthetic();
        let reg = |number| Register { number, span };
        for opcode in [4, 5, 7, 8, 9, 11] {
            let program = Program {
                instructions: vec![Instruction {
                    operation: Operation::Machine {
                        opcode,
                        rd: reg(1),
                        rs1: reg(0),
                        rs2: reg(0),
                        immediate: 0,
                        immediate_span: span,
                    },
                    span,
                }],
                labels: vec![],
                end: span,
            };
            assert!(
                validate(&program, false, &[reg(1)], &[]).is_empty(),
                "opcode {opcode}"
            );
            assert_eq!(
                validate(&program, false, &[], &[]).is_empty(),
                matches!(opcode, 4 | 5 | 7),
                "opcode {opcode} must classify data register read/write"
            );
        }
    }
    #[test]
    fn reserved_opcode_ten_is_rejected() {
        let span = Span::synthetic();
        let reg = Register { number: 0, span };
        let program = Program {
            instructions: vec![Instruction {
                operation: Operation::Machine {
                    opcode: 10,
                    rd: reg.clone(),
                    rs1: reg.clone(),
                    rs2: reg,
                    immediate: 0,
                    immediate_span: span,
                },
                span,
            }],
            labels: vec![],
            end: span,
        };
        assert!(!validate(&program, false, &[], &[]).is_empty());
    }
}
