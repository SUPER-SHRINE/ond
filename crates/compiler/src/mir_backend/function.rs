//! Two-word and scalar lowering with short-lived result registers and stack backing.
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

#[derive(Default)]
pub(super) struct EmittedLayout {
    pub prologue_end: usize,
    pub blocks: Vec<EmittedBlock>,
}

pub(super) struct EmittedBlock {
    pub start: usize,
    pub terminator: usize,
    pub end: usize,
}

pub(super) fn lower(
    p: &mir::Project,
    f: &mir::Function,
    e: &mut Emitter,
) -> Result<PendingFunctionDebug, Diagnostic> {
    lower_body(p, f, e, None, None)
}

pub(super) fn lower_body(
    p: &mir::Project,
    f: &mir::Function,
    e: &mut Emitter,
    induction: Option<&raw_induction::Plan>,
    emitted: Option<&mut EmittedLayout>,
) -> Result<PendingFunctionDebug, Diagnostic> {
    lower_body_using(p, f, e, induction, emitted, &quotient_reuse::plan(f))
}

#[cfg(test)]
pub(super) fn lower_without_quotient_reuse(
    p: &mir::Project,
    f: &mir::Function,
    e: &mut Emitter,
) -> Result<PendingFunctionDebug, Diagnostic> {
    lower_body_using(p, f, e, None, None, &quotient_reuse::Plan::new())
}

fn lower_body_using(
    p: &mir::Project,
    f: &mir::Function,
    e: &mut Emitter,
    induction: Option<&raw_induction::Plan>,
    mut emitted: Option<&mut EmittedLayout>,
    quotients: &quotient_reuse::Plan,
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
    if induction.is_some() && !raw_induction::Plan::storage_available(&frame) {
        return Err(error(f.span, "no private word for raw address induction"));
    }
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
    let capture = |e: &mut Emitter, arg: layout::Argument| match arg {
        layout::Argument::Register(r) => r,
        layout::Argument::Stack(o) => {
            e.load(12, frame.size + o);
            12
        }
    };
    let aggregate_parameters = f
        .parameters
        .iter()
        .any(|id| layout::aggregate(&p.types, f.locals[id.0 as usize].ty));
    let hidden = usize::from(abi.hidden.is_some());
    if aggregate_parameters {
        for (n, arg) in abi.hidden.iter().chain(&abi.arguments).enumerate() {
            let reg = capture(e, *arg);
            e.store(reg, frame.incoming[n]);
        }
    } else if let Some(arg) = abi.hidden {
        let reg = capture(e, arg);
        e.store(reg, frame.incoming[0]);
    }
    let mut incoming = hidden;
    for id in &f.parameters {
        let ty = f.locals[id.0 as usize].ty;
        let input = frame.incoming[incoming];
        if layout::aggregate(&p.types, ty) {
            e.load(4, input);
            memory::address(e, 5, frame.local(*id));
            memory::copy(e, &p.types, ty, f.span)?;
        } else {
            let scalar = layout::scalar(&p.types, ty, f.span)?;
            for word in 0..layout::words(&p.types, ty) {
                let index = incoming + word as usize;
                let reg = if aggregate_parameters {
                    e.load(1, frame.incoming[index]);
                    1
                } else {
                    capture(e, abi.arguments[index - hidden])
                };
                if word == 0 {
                    ops::normalize(e, reg, scalar);
                }
                e.store(reg, frame.local(*id) + 4 * word);
            }
        }
        incoming += layout::words(&p.types, ty) as usize;
    }
    let labels: Vec<_> = f.blocks.iter().map(|_| e.label()).collect();
    e.branch(0, labels[f.entry.0 as usize], f.span);
    if let Some(code) = &mut emitted {
        code.prologue_end = e.words.len();
    }
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
        retained: None,
        previous: None,
        induction: induction.is_some(),
    };
    let proven_bounds = super::bounds::proven_checks(&p.types, f);
    let mut aggregates = super::local_aggregate::LocalAggregates::new(&p.types, f, &cx.abi);
    for b in &f.blocks {
        cx.e.bind(cx.labels[b.id.0 as usize]);
        let block_start = cx.e.words.len();
        cx.retained = None;
        for i in &b.instructions {
            if proven_bounds.contains(&i.id) || induction.is_some_and(|plan| plan.omit(&i.results))
            {
                continue;
            }
            cx.previous = cx.retained.take();
            let start = cx.e.offset(i.span)?;
            if !aggregates.as_mut().is_some_and(|a| a.emit(&mut cx, i)) {
                cx.instruction(i, quotients.get(&i.id).copied())?;
            }
            let end = cx.e.offset(i.span)?;
            let operation = instruction_operation(i, cx.e.has_call_since(start));
            if !matches!(i.kind, mir::InstructionKind::InlineAssembly { .. }) {
                cx.record_range(start, end, operation, None, i.span);
            }
        }
        if let Some(plan) = induction {
            plan.emit_update(cx.e, b.id, &cx.frame);
        }
        let term_start = cx.e.words.len();
        let start = cx.e.offset(b.terminator.span)?;
        cx.terminator(&b.terminator)?;
        let end = cx.e.offset(b.terminator.span)?;
        if let Some(code) = &mut emitted {
            code.blocks.push(EmittedBlock {
                start: block_start,
                terminator: term_start,
                end: cx.e.words.len(),
            });
        }
        let (operation, message) = terminator_operation(&b.terminator);
        cx.record_range(start, end, operation, message, b.terminator.span);
    }
    if f.blocks.iter().flat_map(|b| &b.instructions).any(|i| {
        matches!(
            i.kind,
            mir::InstructionKind::InlineAssembly { noreturn: true, .. }
        )
    }) {
        cx.unwind.clear();
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
    pub(super) retained: Option<mir::ValueId>,
    pub(super) induction: bool,
    previous: Option<mir::ValueId>,
}
impl Context<'_> {
    pub(super) fn record_range(
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
        self.load_word(id, 0, reg);
    }
    /// Only call before an instruction has written its operand registers.
    pub(super) fn load_first(&mut self, id: mir::ValueId, words: u32) {
        if self.previous.take() != Some(id) {
            self.load(id, 1);
            if words == 2 {
                self.load_word(id, 1, 2);
            }
        }
    }
    pub(super) fn load_word(&mut self, id: mir::ValueId, word: u32, reg: u8) {
        self.e.load(reg, self.frame.value(id) + 4 * word);
    }
    pub(super) fn store_result(&mut self, id: mir::ValueId, reg: u8) {
        self.e.store(reg, self.frame.value(id));
        if layout::words(&self.p.types, self.ty(id)) == 2 {
            self.e.store(reg + 1, self.frame.value(id) + 4);
        }
    }
    pub(super) fn ty(&self, id: mir::ValueId) -> mir::TypeId {
        self.f.values[id.0 as usize].ty
    }
    fn instruction(
        &mut self,
        i: &mir::Instruction,
        quotient: Option<mir::ValueId>,
    ) -> Result<(), Diagnostic> {
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
                    mir::Constant::Zero(_) | mir::Constant::Nil(_) => {
                        self.e.immediate(1, 0);
                        if layout::words(&self.p.types, c.ty()) == 2 {
                            self.e.immediate(2, 0);
                        }
                    }
                    mir::Constant::Integer { bits, .. }
                    | mir::Constant::NumericBits { bits, .. } => {
                        self.e.immediate(1, *bits as u32);
                        if layout::words(&self.p.types, c.ty()) == 2 {
                            self.e.immediate(2, (*bits >> 32) as u32);
                        }
                    }
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
                    if layout::words(&self.p.types, place.ty) == 2 {
                        self.load_word(*value, 1, 2);
                    }
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
            I::InlineAssembly {
                program,
                noreturn,
                inputs,
                outputs,
            } => {
                return self.inline_asm(i, program, *noreturn, inputs, outputs);
            }
            I::Check(check) => self.numeric_check(check, i.span)?,
            I::Unary { op, operand } => {
                let ty = self.ty(*operand);
                self.load_first(*operand, layout::words(&self.p.types, ty));
                if *op == mir::UnaryOp::FloatPlus {
                    self.float_positive(ty, i.span);
                } else if let Some(fraction) = self.p.types.fixed_fraction(ty) {
                    self.load_wide_operand(*operand, 3, i.span)?;
                    self.e.emit(0, 1, 0, 0, 0);
                    self.e.emit(0, 2, 0, 0, 0);
                    self.e.immediate(5, fraction);
                    self.e.immediate(6, 1);
                    self.e.call(&runtime::Helper::Fixed.symbol(), i.span)?;
                } else if self.p.types.underlying_id(ty) == Some(mir::TypeId::F64) {
                    self.e.call(&runtime::Helper::Neg64.symbol(), i.span)?;
                } else if self.float(ty) {
                    self.e.call(&runtime::Helper::F32Neg.symbol(), i.span)?;
                } else if layout::words(&self.p.types, ty) == 2 {
                    match op {
                        mir::UnaryOp::Negate => wide::negate(self.e),
                        mir::UnaryOp::BitNot => {
                            self.e.emit(1, 1, 1, 1, 0);
                            self.e.emit(1, 2, 2, 2, 0);
                        }
                        _ => unreachable!(),
                    }
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
                if let Some(quotient) = quotient {
                    self.remainder_from_quotient(*lhs, *rhs, quotient);
                } else {
                    self.numeric_binary(*op, *lhs, *rhs, *overflow, i.span)?;
                }
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
            self.store_result(*result, 1);
            self.retained = Some(*result);
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

fn instruction_operation(instruction: &mir::Instruction, emitted_call: bool) -> DebugOperation {
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
        _ if emitted_call => DebugOperation::Call,
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
        mir::TerminatorKind::Trap(mir::TrapKind::FixedPointOverflow) => {
            (DebugOperation::FixedPointOverflow, None)
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
