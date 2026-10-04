//! Stack-resident O0 lowering. r7..r11 are never touched.
use super::{emit::Emitter, frame::Frame, memory, *};
use ond_protocol::debug::{DebugOperation, ReturnAddressLocation};

pub(super) struct PendingFunctionDebug {
    pub frame_size: u32,
    pub return_address_offset: u32,
    pub unwind: Vec<PendingUnwindRow>,
    pub ranges: Vec<PendingDebugRange>,
}

pub(super) struct PendingUnwindRow {
    pub offset: u32,
    pub cfa_sp_offset: u32,
    pub return_address: ReturnAddressLocation,
}

pub(super) struct PendingDebugRange {
    pub start: u32,
    pub end: u32,
    pub operation: DebugOperation,
    pub message: Option<String>,
    pub span: Span,
}

pub(super) fn lower(
    p: &mir::Project,
    f: &mir::Function,
    e: &mut Emitter,
) -> Result<PendingFunctionDebug, Diagnostic> {
    let signature = mir::FunctionType {
        parameters: f
            .parameters
            .iter()
            .map(|id| f.locals[id.0 as usize].ty)
            .collect(),
        returns: f.returns.clone(),
    };
    let abi = layout::abi(&p.types, &signature, f.span)?;
    let frame = Frame::new(f, &p.types, &abi)?;
    let function_start = e.offset(f.span)?;
    let mut unwind = vec![PendingUnwindRow {
        offset: 0,
        cfa_sp_offset: 0,
        return_address: ReturnAddressLocation::LinkRegister,
    }];
    let mut ranges = Vec::new();
    // Preserve all register parameters until captured, including hidden return pointer.
    let overflow_start = e.offset(f.span)?;
    e.immediate(12, frame.size);
    e.emit(13, 13, 14, 12, 2);
    e.trap_if_nonzero(13);
    e.emit(1, 12, 12, 12, 0);
    e.emit(0, 12, 12, 0, 1);
    e.emit(0, 12, 14, 12, 0);
    e.symbol(13, "__ond_stack_bottom", f.span)?;
    e.emit(13, 13, 12, 13, 2);
    e.trap_if_nonzero(13);
    let overflow_end = e.offset(f.span)?;
    ranges.push(PendingDebugRange {
        start: overflow_start - function_start,
        end: overflow_end - function_start,
        operation: DebugOperation::StackOverflow,
        message: None,
        span: f.span,
    });
    e.emit(0, 14, 12, 0, 0);
    unwind.push(PendingUnwindRow {
        offset: e.offset(f.span)? - function_start,
        cfa_sp_offset: frame.size,
        return_address: ReturnAddressLocation::LinkRegister,
    });
    e.store(15, frame.ra);
    unwind.push(PendingUnwindRow {
        offset: e.offset(f.span)? - function_start,
        cfa_sp_offset: frame.size,
        return_address: ReturnAddressLocation::StackOffset(frame.ra),
    });
    for (n, arg) in abi.hidden.iter().chain(&abi.arguments).enumerate() {
        let reg = match arg {
            layout::Argument::Register(r) => *r,
            layout::Argument::Stack(o) => {
                e.load(12, frame.size + o);
                12
            }
        };
        e.store(reg, frame.incoming[n]);
    }
    for (n, id) in f.parameters.iter().enumerate() {
        let ty = f.locals[id.0 as usize].ty;
        let input = frame.incoming[n + usize::from(abi.hidden.is_some())];
        if layout::aggregate(&p.types, ty) {
            e.load(4, input);
            memory::address(e, 5, frame.local(*id));
            memory::copy(e, &p.types, ty, f.span)?;
        } else {
            e.load(1, input);
            ops::normalize(e, 1, layout::scalar(&p.types, ty, f.span)?);
            e.store(1, frame.local(*id));
        }
    }
    let labels: Vec<_> = f.blocks.iter().map(|_| e.label()).collect();
    e.branch(0, labels[f.entry.0 as usize], f.span);
    let mut cx = Context {
        p,
        f,
        e,
        frame,
        labels,
        abi,
        function_start,
        unwind: &mut unwind,
        ranges: &mut ranges,
    };
    for b in &f.blocks {
        cx.e.bind(cx.labels[b.id.0 as usize]);
        for i in &b.instructions {
            let start = cx.e.offset(i.span)?;
            cx.instruction(i)?;
            let end = cx.e.offset(i.span)?;
            cx.record_range(start, end, instruction_operation(i), None, i.span);
        }
        let start = cx.e.offset(b.terminator.span)?;
        cx.terminator(&b.terminator)?;
        let end = cx.e.offset(b.terminator.span)?;
        let (operation, message) = terminator_operation(&b.terminator);
        cx.record_range(start, end, operation, message, b.terminator.span);
    }
    Ok(PendingFunctionDebug {
        frame_size: cx.frame.size,
        return_address_offset: cx.frame.ra,
        unwind,
        ranges,
    })
}
pub(super) struct Context<'a> {
    pub p: &'a mir::Project,
    pub f: &'a mir::Function,
    pub e: &'a mut Emitter,
    pub frame: Frame,
    pub labels: Vec<usize>,
    pub abi: layout::AbiPlan,
    pub function_start: u32,
    pub unwind: &'a mut Vec<PendingUnwindRow>,
    pub ranges: &'a mut Vec<PendingDebugRange>,
}
impl Context<'_> {
    fn record_range(
        &mut self,
        start: u32,
        end: u32,
        operation: DebugOperation,
        message: Option<String>,
        span: Span,
    ) {
        if start < end {
            self.ranges.push(PendingDebugRange {
                start: start - self.function_start,
                end: end - self.function_start,
                operation,
                message,
                span,
            });
        }
    }
    pub(super) fn load(&mut self, id: mir::ValueId, reg: u8) {
        self.e.load(reg, self.frame.value(id));
    }
    pub(super) fn ty(&self, id: mir::ValueId) -> mir::TypeId {
        self.f.values[id.0 as usize].ty
    }
    fn instruction(&mut self, i: &mir::Instruction) -> Result<(), Diagnostic> {
        use mir::InstructionKind as I;
        match &i.kind {
            I::Constant(c) => {
                if let mir::Constant::Zero(ty) = c {
                    if layout::aggregate(&self.p.types, *ty) {
                        memory::address(self.e, 5, self.frame.value(i.results[0]));
                        for (offset, s) in layout::leaves(&self.p.types, *ty, i.span)? {
                            memory::write(self.e, 0, 5, offset, s);
                        }
                        return Ok(());
                    }
                }
                match c {
                    mir::Constant::Zero(_) | mir::Constant::Nil(_) => self.e.immediate(1, 0),
                    mir::Constant::Integer { bits, .. } => self.e.immediate(1, *bits as u32),
                    mir::Constant::Float32 { bits, .. } => self.e.immediate(1, *bits),
                    mir::Constant::Bool { value, .. } => self.e.immediate(1, u32::from(*value)),
                    mir::Constant::Function { symbol, .. } => self.e.symbol(1, symbol, i.span)?,
                }
            }
            I::AddressOf(p) => {
                self.place(p, i.span)?;
                self.e.emit(0, 1, 4, 0, 0);
            }
            I::Load { place, .. } => {
                self.place(place, i.span)?;
                if layout::aggregate(&self.p.types, place.ty) {
                    memory::address(self.e, 5, self.frame.value(i.results[0]));
                    memory::copy(self.e, &self.p.types, place.ty, i.span)?;
                    return Ok(());
                }
                memory::read(
                    self.e,
                    1,
                    4,
                    0,
                    layout::scalar(&self.p.types, place.ty, i.span)?,
                );
            }
            I::Store { place, value, .. } => {
                self.place(place, i.span)?;
                self.e.emit(0, 5, 4, 0, 0);
                if layout::aggregate(&self.p.types, place.ty) {
                    memory::address(self.e, 4, self.frame.value(*value));
                    memory::copy(self.e, &self.p.types, place.ty, i.span)?;
                } else {
                    self.load(*value, 1);
                    memory::write(
                        self.e,
                        1,
                        5,
                        0,
                        layout::scalar(&self.p.types, place.ty, i.span)?,
                    );
                }
            }
            I::AggregateCopy {
                destination,
                source,
                ..
            } => {
                self.place(source, i.span)?;
                memory::address(self.e, 5, self.frame.scratch);
                memory::copy(self.e, &self.p.types, source.ty, i.span)?;
                self.place(destination, i.span)?;
                self.e.emit(0, 5, 4, 0, 0);
                memory::address(self.e, 4, self.frame.scratch);
                memory::copy(self.e, &self.p.types, source.ty, i.span)?;
            }
            I::Check(check) => self.numeric_check(check, i.span)?,
            I::Unary { op, operand } => {
                self.load(*operand, 1);
                if self.float(self.ty(*operand)) {
                    self.e.call(&runtime::Helper::F32Neg.symbol(), i.span)?;
                } else {
                    ops::unary(self.e, *op);
                }
            }
            I::Binary {
                op,
                lhs,
                rhs,
                overflow,
            } => {
                self.numeric_binary(*op, *lhs, *rhs, *overflow, i.span)?;
            }
            I::Cast {
                value,
                to,
                behavior,
            } => {
                self.numeric_cast(*value, *to, *behavior, i.span)?;
            }
            I::Call {
                callee,
                signature,
                arguments,
                ..
            } => return self.call(i, callee, signature, arguments),
        }
        if let Some(result) = i.results.first() {
            ops::normalize(
                self.e,
                1,
                layout::scalar(&self.p.types, self.ty(*result), i.span)?,
            );
            self.e.store(1, self.frame.value(*result));
        }
        Ok(())
    }
    fn edge(&mut self, edge: &mir::Edge, span: Span) -> Result<(), Diagnostic> {
        let params = self.f.blocks[edge.target.0 as usize].parameters.clone();
        for (src, dst) in edge.arguments.iter().zip(&params) {
            self.copy_slots(
                self.frame.value(*src),
                self.frame.edges[dst.0 as usize],
                self.ty(*src),
                span,
            )?;
        }
        for dst in params {
            self.copy_slots(
                self.frame.edges[dst.0 as usize],
                self.frame.value(dst),
                self.ty(dst),
                span,
            )?;
        }
        self.e.branch(0, self.labels[edge.target.0 as usize], span);
        Ok(())
    }
    fn terminator(&mut self, t: &mir::Terminator) -> Result<(), Diagnostic> {
        match &t.kind {
            mir::TerminatorKind::Jump(edge) => self.edge(edge, t.span)?,
            mir::TerminatorKind::Branch {
                condition,
                then_edge,
                else_edge,
            } => {
                let otherwise = self.e.label();
                self.load(*condition, 1);
                self.e.branch(1, otherwise, t.span);
                self.edge(then_edge, t.span)?;
                self.e.bind(otherwise);
                self.edge(else_edge, t.span)?;
            }
            mir::TerminatorKind::Return(values) => {
                self.return_values(values, t.span)?;
                self.e.load(15, self.frame.ra);
                self.unwind.push(PendingUnwindRow {
                    offset: self.e.offset(t.span)? - self.function_start,
                    cfa_sp_offset: self.frame.size,
                    return_address: ReturnAddressLocation::LinkRegister,
                });
                self.e.release_frame(self.frame.size);
                self.unwind.push(PendingUnwindRow {
                    offset: self.e.offset(t.span)? - self.function_start,
                    cfa_sp_offset: 0,
                    return_address: ReturnAddressLocation::LinkRegister,
                });
                self.e.emit(12, 0, 0, 15, 0);
                self.unwind.push(PendingUnwindRow {
                    offset: self.e.offset(t.span)? - self.function_start,
                    cfa_sp_offset: self.frame.size,
                    return_address: ReturnAddressLocation::StackOffset(self.frame.ra),
                });
            }
            mir::TerminatorKind::Trap(_) | mir::TerminatorKind::Unreachable => self.e.trap(),
        }
        Ok(())
    }
}

fn instruction_operation(instruction: &mir::Instruction) -> DebugOperation {
    match &instruction.kind {
        mir::InstructionKind::Load { .. } => DebugOperation::MemoryRead,
        mir::InstructionKind::Store { .. } | mir::InstructionKind::AggregateCopy { .. } => {
            DebugOperation::MemoryWrite
        }
        mir::InstructionKind::Call { .. } => DebugOperation::Call,
        mir::InstructionKind::Check(mir::Check::Bounds { .. }) => DebugOperation::BoundsCheck,
        mir::InstructionKind::Check(mir::Check::NonZero { .. }) => DebugOperation::DivisionByZero,
        mir::InstructionKind::Check(mir::Check::SignedDivisionOverflow { .. }) => {
            DebugOperation::SignedDivisionOverflow
        }
        mir::InstructionKind::Cast {
            behavior: mir::CastBehavior::Checked,
            ..
        } => DebugOperation::InvalidConversion,
        _ => DebugOperation::Source,
    }
}

fn terminator_operation(terminator: &mir::Terminator) -> (DebugOperation, Option<String>) {
    match &terminator.kind {
        mir::TerminatorKind::Trap(mir::TrapKind::Explicit(message)) => {
            (DebugOperation::ExplicitTrap, Some(message.clone()))
        }
        mir::TerminatorKind::Trap(mir::TrapKind::Bounds) => (DebugOperation::BoundsCheck, None),
        mir::TerminatorKind::Trap(mir::TrapKind::DivisionByZero) => {
            (DebugOperation::DivisionByZero, None)
        }
        mir::TerminatorKind::Trap(mir::TrapKind::SignedDivisionOverflow) => {
            (DebugOperation::SignedDivisionOverflow, None)
        }
        mir::TerminatorKind::Trap(mir::TrapKind::InvalidConversion) => {
            (DebugOperation::InvalidConversion, None)
        }
        mir::TerminatorKind::Trap(mir::TrapKind::StackOverflow) => {
            (DebugOperation::StackOverflow, None)
        }
        _ => (DebugOperation::Source, None),
    }
}
