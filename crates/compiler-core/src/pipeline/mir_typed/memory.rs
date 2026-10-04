//! Symbolic places and aggregate temporaries; no target offsets or byte sizes.
use super::FunctionBuilder;
use crate::{ir::hir, mir::*, source::Span};

impl FunctionBuilder<'_> {
    pub(super) fn temporary(&mut self, ty: TypeId, span: Span) -> Place {
        let id = LocalId(self.locals.len() as u32);
        self.locals.push(Local {
            id,
            name: None,
            ty,
            kind: LocalKind::Temporary,
            address_taken: false,
            span,
        });
        Place {
            base: PlaceBase::Local(id),
            projections: Vec::new(),
            ty,
        }
    }

    pub(super) fn access(place: &Place) -> AccessKind {
        if place.requires_ordered_access() {
            AccessKind::Ordered
        } else {
            AccessKind::Local
        }
    }

    pub(super) fn load_place_value(&mut self, place: Place, span: Span) -> ValueId {
        let ty = place.ty;
        let access = Self::access(&place);
        self.emit_value(ty, InstructionKind::Load { place, access }, span)
    }

    // Aggregate ValueIds are immutable by-value snapshots, not storage aliases.
    pub(super) fn store_value(&mut self, destination: Place, value: ValueId, span: Span) {
        let access = Self::access(&destination);
        if matches!(
            self.types.underlying_kind(destination.ty),
            Some(TypeKind::Array { .. } | TypeKind::Struct(_) | TypeKind::Interface(_))
        ) {
            let source = self.temporary(destination.ty, span);
            self.emit_effect(
                InstructionKind::Store {
                    place: source.clone(),
                    value,
                    access: AccessKind::Local,
                },
                span,
            );
            self.emit_effect(
                InstructionKind::AggregateCopy {
                    destination,
                    source,
                    access,
                },
                span,
            );
        } else {
            self.emit_effect(
                InstructionKind::Store {
                    place: destination,
                    value,
                    access,
                },
                span,
            );
        }
    }

    pub(super) fn lower_place(&mut self, expr: &hir::Expr) -> Option<Place> {
        match &expr.kind {
            hir::ExprKind::Global(symbol) => Some(Place {
                base: PlaceBase::Static {
                    symbol: symbol.clone(),
                    ty: expr.ty,
                },
                projections: Vec::new(),
                ty: expr.ty,
            }),
            hir::ExprKind::Local(id) => Some(Place {
                base: PlaceBase::Local(LocalId(id.0)),
                projections: Vec::new(),
                ty: expr.ty,
            }),
            hir::ExprKind::Dereference(pointer) => Some(Place {
                base: PlaceBase::Pointer(self.lower_expr(pointer)?),
                projections: Vec::new(),
                ty: expr.ty,
            }),
            hir::ExprKind::Field { base, index } => {
                let mut place = self.lower_place(base)?;
                place.projections.push(Projection::Field {
                    index: *index,
                    ty: expr.ty,
                });
                place.ty = expr.ty;
                Some(place)
            }
            hir::ExprKind::Index { base, index } => {
                let (mut place, length) = match self.types.underlying_kind(base.ty)? {
                    TypeKind::Pointer(_) => (
                        Place {
                            base: PlaceBase::Pointer(self.lower_expr(base)?),
                            projections: Vec::new(),
                            ty: expr.ty,
                        },
                        None,
                    ),
                    TypeKind::Array { length, .. } => {
                        let length = *length;
                        (self.lower_place(base)?, Some(length))
                    }
                    _ => return None,
                };
                let index_value = self.lower_expr(index)?;
                if let Some(length) = length {
                    let length = self.emit_value(
                        TypeId::U32,
                        InstructionKind::Constant(Constant::Integer {
                            bits: u64::from(length),
                            ty: TypeId::U32,
                        }),
                        expr.span,
                    );
                    self.emit_effect(
                        InstructionKind::Check(Check::Bounds {
                            index: index_value,
                            length,
                        }),
                        index.span,
                    );
                }
                place.projections.push(if length.is_none() {
                    Projection::Offset { index: index_value }
                } else {
                    Projection::Index {
                        index: index_value,
                        element: expr.ty,
                    }
                });
                place.ty = expr.ty;
                Some(place)
            }
            _ => {
                // HIR rejects writing/addressing these rvalue temporaries.
                let value = self.lower_expr(expr)?;
                let place = self.temporary(expr.ty, expr.span);
                self.store_value(place.clone(), value, expr.span);
                Some(place)
            }
        }
    }

    pub(super) fn lower_memory_expr(&mut self, expr: &hir::Expr) -> Option<ValueId> {
        let kind = match &expr.kind {
            hir::ExprKind::AddressOf(value) => {
                let place = self.lower_place(value)?;
                if let PlaceBase::Local(id) = place.base {
                    self.locals[id.0 as usize].address_taken = true;
                }
                InstructionKind::AddressOf(place)
            }
            hir::ExprKind::Global(_)
            | hir::ExprKind::Dereference(_)
            | hir::ExprKind::Field { .. }
            | hir::ExprKind::Index { .. } => {
                let place = self.lower_place(expr)?;
                InstructionKind::Load {
                    access: Self::access(&place),
                    place,
                }
            }
            hir::ExprKind::Composite(elements) => {
                let place = self.temporary(expr.ty, expr.span);
                let zero = self.emit_value(
                    expr.ty,
                    InstructionKind::Constant(Constant::Zero(expr.ty)),
                    expr.span,
                );
                self.store_value(place.clone(), zero, expr.span);
                for (index, value) in elements {
                    let evaluated = self.lower_expr(value)?;
                    let mut destination = place.clone();
                    destination
                        .projections
                        .push(match self.types.underlying_kind(expr.ty)? {
                            TypeKind::Array { .. } => Projection::Index {
                                index: self.emit_value(
                                    TypeId::U32,
                                    InstructionKind::Constant(Constant::Integer {
                                        bits: u64::from(*index),
                                        ty: TypeId::U32,
                                    }),
                                    value.span,
                                ),
                                element: value.ty,
                            },
                            TypeKind::Struct(_) | TypeKind::Interface(_) => Projection::Field {
                                index: *index,
                                ty: value.ty,
                            },
                            _ => return None,
                        });
                    destination.ty = value.ty;
                    self.store_value(destination, evaluated, value.span);
                }
                InstructionKind::Load {
                    place,
                    access: AccessKind::Local,
                }
            }
            _ => return None,
        };
        Some(self.emit_value(expr.ty, kind, expr.span))
    }
}
