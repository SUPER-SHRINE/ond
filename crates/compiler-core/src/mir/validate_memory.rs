//! Check each place projection, not just its final declared type.
use super::*;

impl Validator<'_> {
    pub(super) fn validate_place(&mut self, block: BlockId, place: &Place) {
        self.validate_type_id(Some(block), place.ty, "place");
        let mut current = match &place.base {
            PlaceBase::Local(id) => match self.local(*id) {
                Some(local) => Some(local.ty),
                None => {
                    self.error(Some(block), format!("missing local {id:?}"));
                    None
                }
            },
            PlaceBase::Static { ty, .. } => {
                self.validate_type_id(Some(block), *ty, "static place");
                Some(*ty)
            }
            PlaceBase::Pointer(value) => {
                match self
                    .value_type(block, *value)
                    .and_then(|ty| self.types.underlying_kind(ty))
                {
                    Some(TypeKind::Pointer(pointee)) => Some(*pointee),
                    _ => {
                        self.error(Some(block), "place pointer has non-pointer type".into());
                        None
                    }
                }
            }
        };
        for (position, projection) in place.projections.iter().enumerate() {
            let Some(ty) = current else {
                break;
            };
            current = match projection {
                Projection::Dereference => match self.types.underlying_kind(ty) {
                    Some(TypeKind::Pointer(pointee)) => Some(*pointee),
                    _ => {
                        self.error(
                            Some(block),
                            "dereference projection requires pointer".into(),
                        );
                        None
                    }
                },
                Projection::Field { index, ty: result } => {
                    let actual = match self.types.underlying_kind(ty) {
                        Some(TypeKind::Struct(structure)) => {
                            structure.fields.get(*index as usize).map(|f| f.ty)
                        }
                        Some(TypeKind::Interface(interface)) => match *index {
                            0 => Some(interface.data_pointer),
                            1 => Some(interface.vtable),
                            _ => None,
                        },
                        _ => None,
                    };
                    if actual != Some(*result) {
                        self.error(Some(block), "invalid field projection type or index".into());
                    }
                    actual
                }
                Projection::Index { index, element } => {
                    self.expect_integer(block, *index, "array index");
                    let actual = match self.types.underlying_kind(ty) {
                        Some(TypeKind::Array { element, .. }) => Some(*element),
                        _ => None,
                    };
                    if actual != Some(*element) {
                        self.error(Some(block), "invalid array projection type".into());
                    }
                    actual
                }
                Projection::Offset { index } => {
                    if position != 0 || !matches!(place.base, PlaceBase::Pointer(_)) {
                        self.error(Some(block), "raw offset requires a pointer base".into());
                    }
                    if self
                        .value_type(block, *index)
                        .is_some_and(|ty| ty != TypeId::U32)
                    {
                        self.error(Some(block), "raw pointer index requires u32".into());
                    }
                    Some(ty)
                }
            };
        }
        if current.is_some_and(|ty| ty != place.ty) {
            self.error(
                Some(block),
                "place result type differs from its projections".into(),
            );
        }
    }
}
