//! asm本文は1命令=1wordで配置。staging accessはすべてSP近傍に限定する。
use super::{function::Context, *};
use ond_compiler_core::inline_asm::{Operation, Program, Register};
use ond_protocol::debug::DebugOperation;

impl Context<'_> {
    pub(super) fn inline_asm(
        &mut self,
        instruction: &mir::Instruction,
        program: &Program,
        noreturn: bool,
        inputs: &[(Register, mir::ValueId)],
        outputs: &[(Register, mir::TypeId)],
    ) -> Result<(), Diagnostic> {
        if !noreturn {
            for register in 7..=11 {
                self.e.store(register, 64 + u32::from(register - 7) * 4);
            }
        }
        // 大きなvalue slotへのloadはr12/r13を使い得る。配置前に全値を近傍へ移す。
        for (register, value) in inputs {
            self.e.load(1, self.frame.value(*value));
            ops::normalize(
                self.e,
                1,
                layout::scalar(
                    &self.p.types,
                    self.f.values[value.0 as usize].ty,
                    instruction.span,
                )?,
            );
            self.e.store(1, u32::from(register.number) * 4);
        }
        for (register, _) in inputs.iter().filter(|(register, _)| register.number != 14) {
            self.e.load(register.number, u32::from(register.number) * 4);
        }
        // r14は最後に確定し、以後本文が終わるまでcompilerのstack accessを行わない。
        if inputs.iter().any(|(register, _)| register.number == 14) {
            self.e.load(14, 56);
        }
        let labels = program
            .labels
            .iter()
            .map(|label| (label.name.as_str(), label.position))
            .collect::<std::collections::BTreeMap<_, _>>();
        for (index, source) in program.instructions.iter().enumerate() {
            let start = self.e.offset(source.span)?;
            let operation = match &source.operation {
                Operation::Machine {
                    opcode,
                    rd,
                    rs1,
                    rs2,
                    immediate,
                    ..
                } => {
                    self.e.emit(
                        u32::from(*opcode),
                        rd.number,
                        rs1.number,
                        rs2.number,
                        *immediate,
                    );
                    match opcode {
                        4 | 5 | 7 => DebugOperation::MemoryRead,
                        8 | 9 | 11 => DebugOperation::MemoryWrite,
                        _ => DebugOperation::Source,
                    }
                }
                Operation::LocalBranch {
                    condition,
                    target,
                    target_span,
                } => {
                    let position = labels
                        .get(target.as_str())
                        .ok_or_else(|| error(*target_span, "undefined asm label"))?;
                    let immediate =
                        i16::try_from(*position as i128 - index as i128 - 1).map_err(|_| {
                            error(
                                *target_span,
                                "asm branch offset exceeds signed 16-bit range",
                            )
                        })?;
                    self.e.emit(12, 0, condition.number, 0, immediate);
                    DebugOperation::Source
                }
                Operation::ExternalBranch { target } => {
                    self.e.emit(12, 0, 0, target.number, 0);
                    DebugOperation::Source
                }
            };
            let end = self.e.offset(source.span)?;
            self.record_range(start, end, operation, None, source.span);
        }
        if !noreturn {
            // 全outputを先に取り出す。normalizeや大きなslotへのstoreが他outputを壊さない。
            for (register, _) in outputs {
                self.e
                    .store(register.number, u32::from(register.number) * 4);
            }
            for register in 7..=11 {
                self.e.load(register, 64 + u32::from(register - 7) * 4);
            }
            for ((register, ty), result) in outputs.iter().zip(&instruction.results) {
                self.e.load(1, u32::from(register.number) * 4);
                ops::normalize(
                    self.e,
                    1,
                    layout::scalar(&self.p.types, *ty, instruction.span)?,
                );
                self.e.store(1, self.frame.value(*result));
            }
        }
        Ok(())
    }
}
