//! Keep the private snapshot of a small direct-call result in registers.
//! Both ABI records and their frame reservations remain distinct. Only the
//! private value slot accesses between the complete read and write disappear.
use super::{Span, emit::Emitter, layout, memory, mir};
use mir::{InstructionKind as I, Place, Projection, TypeKind};

pub(super) struct Plan {
    pub call: mir::InstructionId,
    fields: Vec<Field>,
}

struct Field {
    offset: u32,
    first: u8,
    scalar: layout::ScalarLayout,
    elements: u32,
    direct: bool,
}

impl Plan {
    pub fn new(p: &mir::Project, f: &mir::Function, abi: &layout::AbiPlan) -> Option<Self> {
        abi.hidden?;
        let [block] = f.blocks.as_slice() else {
            return None;
        };
        let (call, before) = block.instructions.split_last()?;
        let I::Call {
            callee: mir::Callee::Direct(name),
            signature,
            ..
        } = &call.kind
        else {
            return None;
        };
        let mir::TerminatorKind::Return(values) = &block.terminator.kind else {
            return None;
        };
        // In validated single-block SSA, the last definition has no other use.
        if *values != call.results || signature.returns != f.returns {
            return None;
        }

        let mut fields = Vec::new();
        // r1/r2 carry payload, including paired words. r3 forms the return
        // offset, r4/r5 hold ABI addresses, and stack accesses can use r13.
        let mut first = 1;
        for (&ty, &offset) in f.returns.iter().zip(&abi.return_offsets) {
            let (element, elements, direct) = match p.types.underlying_kind(ty)? {
                TypeKind::Array { length, element } if *length != 0 => (*element, *length, false),
                _ => (ty, 1, true),
            };
            let scalar = relay_scalar(&p.types, element, call.span)?;
            let registers = elements.checked_mul(scalar.size.div_ceil(4))?;
            if registers > u32::from(3 - first) {
                return None;
            }
            fields.push(Field {
                offset,
                first,
                scalar,
                elements,
                direct,
            });
            first += registers as u8;
        }

        // A selected package always emits both bodies into one Object. An
        // independently linked package need not contain the body in this MIR.
        let package = p
            .packages
            .iter()
            .find(|package| package.functions.iter().any(|owner| std::ptr::eq(owner, f)))?;
        let child = package.functions.iter().find(|child| child.name == *name)?;
        // Only local typed accesses are possible outside ABI marshaling. In
        // particular neither function can observe stack storage through raw
        // aliases, and the child cannot reach an opaque observer.
        if child.returns != signature.returns
            || f.locals
                .iter()
                .chain(&child.locals)
                .any(|l| l.address_taken)
            || !before.iter().all(private_instruction)
            || !child
                .blocks
                .iter()
                .flat_map(|block| &block.instructions)
                .all(private_instruction)
        {
            return None;
        }
        Some(Self {
            call: call.id,
            fields,
        })
    }

    pub fn read_child(&self, e: &mut Emitter, record: u32) {
        for field in &self.fields {
            memory::address(e, 4, record + field.offset);
            for n in 0..field.elements {
                memory::read(e, field.register(n), 4, n * field.scalar.size, field.scalar);
            }
        }
    }

    pub fn write_parent(&self, e: &mut Emitter, incoming: u32) {
        for field in &self.fields {
            e.load(5, incoming);
            e.immediate(3, field.offset);
            e.emit(0, 5, 5, 3, 0);
            for n in 0..field.elements {
                memory::write(e, field.register(n), 5, n * field.scalar.size, field.scalar);
            }
            if field.direct {
                memory::zero_upper_slot_bytes(e, field.scalar);
            }
        }
    }
}

impl Field {
    fn register(&self, element: u32) -> u8 {
        self.first + (element * self.scalar.size.div_ceil(4)) as u8
    }
}

fn relay_scalar(
    types: &mir::TypeTable,
    ty: mir::TypeId,
    span: Span,
) -> Option<layout::ScalarLayout> {
    use TypeKind::*;
    match types.underlying_kind(ty)? {
        Bool | U8 | I8 | U16 | I16 | U32 | I32 | Q16 | F32 | U64 | I64 | Q32 | F64 => {
            layout::scalar(types, ty, span).ok()
        }
        _ => None,
    }
}

fn private_instruction(instruction: &mir::Instruction) -> bool {
    let private = |place: &Place| {
        matches!(place.base, mir::PlaceBase::Local(_))
            && place
                .projections
                .iter()
                .all(|p| matches!(p, Projection::Field { .. } | Projection::Index { .. }))
    };
    match &instruction.kind {
        I::Load { place, access } | I::Store { place, access, .. } => {
            *access == mir::AccessKind::Local && private(place)
        }
        I::AggregateCopy {
            destination,
            source,
            access,
        } => *access == mir::AccessKind::Local && private(destination) && private(source),
        I::Constant(_) | I::Check(_) => true,
        _ => false,
    }
}
