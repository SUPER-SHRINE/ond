//! ABI marshaling. Argument copies, snapshots and return areas never overlap.
use super::{function::Context, memory, *};
impl Context<'_> {
    pub(super) fn call(
        &mut self,
        i: &mir::Instruction,
        callee: &mir::Callee,
        signature: &mir::FunctionType,
        arguments: &[mir::ValueId],
    ) -> Result<(), Diagnostic> {
        use ond_compiler_core::semantic::Intrinsic;
        match callee {
            mir::Callee::Intrinsic(Intrinsic::Store32) => {
                self.load(arguments[0], 1);
                self.load(arguments[1], 2);
                self.e.emit(11, 2, 1, 0, 0);
                return Ok(());
            }
            mir::Callee::Intrinsic(Intrinsic::Load32) => {
                self.load(arguments[0], 1);
                self.e.emit(7, 1, 1, 0, 0);
                self.e.store(1, self.frame.value(i.results[0]));
                return Ok(());
            }
            mir::Callee::Intrinsic(
                op @ (Intrinsic::Alloc(_) | Intrinsic::Free | Intrinsic::New(_)),
            ) => {
                use runtime::Helper;
                let helper = match op {
                    Intrinsic::New(ty) => {
                        self.e
                            .immediate(1, layout::data(&self.p.types, *ty, i.span)?.size);
                        Helper::New
                    }
                    Intrinsic::Alloc(ty) => {
                        self.load(arguments[0], 1);
                        let size = layout::data(&self.p.types, *ty, i.span)?.size.max(1);
                        self.e.immediate(2, size);
                        self.e.emit(3, 1, 1, 2, 0);
                        Helper::Alloc
                    }
                    _ => {
                        self.load(arguments[0], 1);
                        Helper::Free
                    }
                };
                self.e.symbol(2, "__ond_heap_base", i.span)?;
                if helper == Helper::Free {
                    self.e.symbol(3, "__ond_heap_cursor", i.span)?;
                } else {
                    self.e.symbol(3, "__ond_stack_bottom", i.span)?;
                    self.e.symbol(4, "__ond_heap_cursor", i.span)?;
                }
                self.e.call(&helper.symbol(), i.span)?;
                if let Some(value) = i.results.first() {
                    self.e.store(1, self.frame.value(*value));
                }
                return Ok(());
            }
            mir::Callee::Direct(_) | mir::Callee::Indirect(_) => {}
        }
        let abi = layout::abi(&self.p.types, signature, i.span)?;
        let copies = self.frame.calls[&i.id].arguments.clone();
        let returns = self.frame.calls[&i.id].returns;
        for (value, copy) in arguments.iter().zip(&copies) {
            if let Some(slot) = copy {
                self.copy_slots(self.frame.value(*value), *slot, self.ty(*value), i.span)?;
            }
        }
        // Copying clobbers r1..r5; populate argument registers only afterwards.
        if let Some(location) = abi.hidden {
            self.argument_address(location, returns);
        }
        for ((value, copy), location) in arguments.iter().zip(copies).zip(abi.arguments) {
            if let Some(slot) = copy {
                self.argument_address(location, slot);
            } else {
                match location {
                    layout::Argument::Register(r) => self.load(*value, r),
                    layout::Argument::Stack(o) => {
                        self.load(*value, 12);
                        self.e.store(12, o);
                    }
                }
            }
        }
        match callee {
            mir::Callee::Direct(name) => self.e.call(name, i.span)?,
            mir::Callee::Indirect(v) => {
                self.load(*v, 12);
                self.e.emit(12, 15, 0, 12, 0);
            }
            _ => unreachable!(),
        }
        if abi.hidden.is_some() {
            for (value, offset) in i.results.iter().zip(abi.return_offsets) {
                if layout::aggregate(&self.p.types, self.ty(*value)) {
                    self.copy_slots(
                        returns + offset,
                        self.frame.value(*value),
                        self.ty(*value),
                        i.span,
                    )?;
                } else {
                    memory::address(self.e, 4, returns + offset);
                    memory::read(
                        self.e,
                        1,
                        4,
                        0,
                        layout::scalar(&self.p.types, self.ty(*value), i.span)?,
                    );
                    self.e.store(1, self.frame.value(*value));
                }
            }
        } else {
            for (n, value) in i.results.iter().enumerate() {
                let r = n as u8 + 1;
                ops::normalize(
                    self.e,
                    r,
                    layout::scalar(&self.p.types, self.ty(*value), i.span)?,
                );
                self.e.store(r, self.frame.value(*value));
            }
        }
        Ok(())
    }
    fn argument_address(&mut self, arg: layout::Argument, offset: u32) {
        match arg {
            layout::Argument::Register(r) => memory::address(self.e, r, offset),
            layout::Argument::Stack(o) => {
                memory::address(self.e, 12, offset);
                self.e.store(12, o);
            }
        }
    }
    pub(super) fn return_values(
        &mut self,
        values: &[mir::ValueId],
        span: Span,
    ) -> Result<(), Diagnostic> {
        if self.abi.hidden.is_some() {
            for (n, value) in values.iter().enumerate() {
                self.e.load(5, self.frame.incoming[0]);
                self.e.immediate(2, self.abi.return_offsets[n]);
                self.e.emit(0, 5, 5, 2, 0);
                if layout::aggregate(&self.p.types, self.ty(*value)) {
                    memory::address(self.e, 4, self.frame.value(*value));
                    memory::copy(self.e, &self.p.types, self.ty(*value), span)?;
                } else {
                    self.load(*value, 1);
                    let ty = layout::scalar(&self.p.types, self.ty(*value), span)?;
                    memory::write(self.e, 1, 5, 0, ty);
                    // Narrow Direct-1 slots can start at a non-word-aligned record offset.
                    // Preserve the natural alignment and canonical zero upper ABI bits.
                    let byte = layout::scalar(&self.p.types, mir::TypeId::U8, span)?;
                    for offset in ty.size..4 {
                        memory::write(self.e, 0, 5, offset, byte);
                    }
                }
            }
        } else {
            for (n, value) in values.iter().enumerate() {
                self.load(*value, n as u8 + 1);
            }
        }
        Ok(())
    }
}
