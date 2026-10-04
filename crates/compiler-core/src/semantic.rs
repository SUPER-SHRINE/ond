use std::collections::BTreeMap;
use std::fmt;
use std::hash::{Hash, Hasher};

use crate::project::PackageId;
use crate::source::Span;

/// Target-neutral runtime operations; allocation layout is resolved downstream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intrinsic {
    New(TypeId),
    Alloc(TypeId),
    Free,
    Load32,
    Store32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeId(pub u32);

impl TypeId {
    pub const BOOL: Self = Self(0);
    pub const U8: Self = Self(1);
    pub const I8: Self = Self(2);
    pub const U16: Self = Self(3);
    pub const I16: Self = Self(4);
    pub const U32: Self = Self(5);
    pub const I32: Self = Self(6);
    pub const F32: Self = Self(7);
}

impl fmt::Display for TypeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "type#{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeTable {
    definitions: Vec<TypeDefinition>,
    generic_instances: BTreeMap<TypeId, GenericInstance>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericInstance {
    pub declaration: String,
    pub arguments: Vec<TypeId>,
}

impl Default for TypeTable {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeTable {
    pub fn new() -> Self {
        let mut table = Self {
            definitions: Vec::new(),
            generic_instances: BTreeMap::new(),
        };
        for (name, kind) in [
            ("bool", TypeKind::Bool),
            ("u8", TypeKind::U8),
            ("i8", TypeKind::I8),
            ("u16", TypeKind::U16),
            ("i16", TypeKind::I16),
            ("u32", TypeKind::U32),
            ("i32", TypeKind::I32),
            ("f32", TypeKind::F32),
        ] {
            table.push(Some(name.to_string()), kind, Span::synthetic());
        }
        table
    }

    pub fn definitions(&self) -> &[TypeDefinition] {
        &self.definitions
    }

    pub fn get(&self, id: TypeId) -> Option<&TypeDefinition> {
        self.definitions.get(id.0 as usize)
    }

    pub fn find_named(&self, name: &str) -> Option<TypeId> {
        self.definitions
            .iter()
            .find(|definition| definition.name.as_deref() == Some(name))
            .map(|definition| definition.id)
    }

    pub fn kind(&self, id: TypeId) -> Option<&TypeKind> {
        self.get(id).map(|definition| &definition.kind)
    }

    pub fn reserve_defined(&mut self, name: String, span: Span) -> TypeId {
        self.push(Some(name), TypeKind::Pending, span)
    }

    pub fn reserve_generic_defined(
        &mut self,
        name: String,
        declaration: String,
        arguments: Vec<TypeId>,
        span: Span,
    ) -> TypeId {
        let id = self.reserve_defined(name, span);
        self.generic_instances.insert(
            id,
            GenericInstance {
                declaration,
                arguments,
            },
        );
        id
    }

    pub fn generic_instance(&self, id: TypeId) -> Option<&GenericInstance> {
        self.generic_instances.get(&id)
    }

    pub fn define(&mut self, id: TypeId, kind: TypeKind) {
        let definition = self
            .definitions
            .get_mut(id.0 as usize)
            .expect("reserved type id must exist");
        definition.kind = kind;
    }

    pub fn intern(&mut self, kind: TypeKind, span: Span) -> TypeId {
        if let Some(definition) = self
            .definitions
            .iter()
            .find(|definition| definition.name.is_none() && definition.kind == kind)
        {
            return definition.id;
        }
        self.push(None, kind, span)
    }

    pub fn pointer(&mut self, pointee: TypeId, span: Span) -> TypeId {
        self.intern(TypeKind::Pointer(pointee), span)
    }

    pub fn function(&mut self, signature: FunctionType, span: Span) -> TypeId {
        self.intern(TypeKind::Function(signature), span)
    }

    pub fn underlying_id(&self, mut id: TypeId) -> Option<TypeId> {
        let mut remaining = self.definitions.len() + 1;
        while remaining > 0 {
            match self.kind(id)? {
                TypeKind::Defined { underlying } => id = *underlying,
                TypeKind::Pending => return None,
                _ => return Some(id),
            }
            remaining -= 1;
        }
        None
    }

    pub fn underlying_kind(&self, id: TypeId) -> Option<&TypeKind> {
        self.kind(self.underlying_id(id)?)
    }

    pub fn is_integer(&self, id: TypeId) -> bool {
        matches!(
            self.underlying_kind(id),
            Some(
                TypeKind::U8
                    | TypeKind::I8
                    | TypeKind::U16
                    | TypeKind::I16
                    | TypeKind::U32
                    | TypeKind::I32
            )
        )
    }

    pub fn is_signed_integer(&self, id: TypeId) -> bool {
        matches!(
            self.underlying_kind(id),
            Some(TypeKind::I8 | TypeKind::I16 | TypeKind::I32)
        )
    }

    pub fn display(&self, id: TypeId) -> TypeDisplay<'_> {
        TypeDisplay { table: self, id }
    }

    /// Ond v1 data layout. Kagura is a 32-bit target and every scalar has a
    /// natural alignment no greater than four bytes.
    pub fn size_align(&self, id: TypeId) -> Result<(u32, u32), &'static str> {
        fn align(value: u32, alignment: u32) -> Option<u32> {
            value
                .checked_add(alignment - 1)
                .map(|value| value & !(alignment - 1))
        }
        fn visit(table: &TypeTable, id: TypeId, depth: usize) -> Result<(u32, u32), &'static str> {
            if depth > table.definitions.len() {
                return Err("recursive value layout");
            }
            match table.underlying_kind(id) {
                Some(TypeKind::Bool | TypeKind::U8 | TypeKind::I8) => Ok((1, 1)),
                Some(TypeKind::U16 | TypeKind::I16) => Ok((2, 2)),
                Some(
                    TypeKind::U32
                    | TypeKind::I32
                    | TypeKind::F32
                    | TypeKind::Pointer(_)
                    | TypeKind::Function(_),
                ) => Ok((4, 4)),
                Some(TypeKind::Array { length, element }) => {
                    let (size, alignment) = visit(table, *element, depth + 1)?;
                    Ok((
                        size.checked_mul(*length).ok_or("array layout overflow")?,
                        alignment,
                    ))
                }
                Some(TypeKind::Struct(structure)) => {
                    let mut size = 0u32;
                    let mut structure_alignment = 1u32;
                    for field in &structure.fields {
                        let (field_size, field_alignment) = visit(table, field.ty, depth + 1)?;
                        size = align(size, field_alignment).ok_or("struct layout overflow")?;
                        size = size
                            .checked_add(field_size)
                            .ok_or("struct layout overflow")?;
                        structure_alignment = structure_alignment.max(field_alignment);
                    }
                    Ok((
                        align(size, structure_alignment).ok_or("struct layout overflow")?,
                        structure_alignment,
                    ))
                }
                Some(TypeKind::Interface(interface)) => {
                    let mut size = 0u32;
                    let mut interface_alignment = 1u32;
                    for field in [interface.data_pointer, interface.vtable] {
                        let (field_size, field_alignment) = visit(table, field, depth + 1)?;
                        size = align(size, field_alignment).ok_or("interface layout overflow")?;
                        size = size
                            .checked_add(field_size)
                            .ok_or("interface layout overflow")?;
                        interface_alignment = interface_alignment.max(field_alignment);
                    }
                    Ok((
                        align(size, interface_alignment).ok_or("interface layout overflow")?,
                        interface_alignment,
                    ))
                }
                Some(TypeKind::Pending | TypeKind::Defined { .. }) | None => {
                    Err("type layout is unavailable")
                }
            }
        }
        visit(self, id, 0)
    }

    fn push(&mut self, name: Option<String>, kind: TypeKind, span: Span) -> TypeId {
        let id = TypeId(self.definitions.len() as u32);
        self.definitions.push(TypeDefinition {
            id,
            name,
            kind,
            span,
        });
        id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeDefinition {
    pub id: TypeId,
    pub name: Option<String>,
    pub kind: TypeKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeKind {
    Pending,
    Bool,
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    F32,
    Pointer(TypeId),
    Function(FunctionType),
    Array { length: u32, element: TypeId },
    Struct(StructType),
    Interface(InterfaceType),
    Defined { underlying: TypeId },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionType {
    pub parameters: Vec<TypeId>,
    pub returns: Vec<TypeId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructType {
    pub fields: Vec<StructField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InterfaceType {
    pub methods: Vec<InterfaceMethod>,
    pub data_pointer: TypeId,
    pub vtable: TypeId,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InterfaceMethod {
    pub name: String,
    /// Erased dispatch signature. The first parameter is always `*u8` data.
    pub signature: FunctionType,
    pub private_owner: Option<PackageId>,
}

#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub ty: TypeId,
    /// Private names are package-qualified; exported names have no owner.
    pub private_owner: Option<PackageId>,
    /// Diagnostic origin only, never part of structural type identity.
    pub span: Span,
}

impl PartialEq for StructField {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.ty == other.ty && self.private_owner == other.private_owner
    }
}

impl Eq for StructField {}

impl Hash for StructField {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.ty.hash(state);
        self.private_owner.hash(state);
    }
}

pub struct TypeDisplay<'a> {
    table: &'a TypeTable,
    id: TypeId,
}

impl fmt::Display for TypeDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Some(definition) = self.table.get(self.id) else {
            return write!(f, "<invalid-type#{}>", self.id.0);
        };
        if let Some(instance) = self.table.generic_instance(self.id) {
            let declaration = instance
                .declaration
                .strip_prefix("..")
                .unwrap_or(&instance.declaration);
            let arguments = instance
                .arguments
                .iter()
                .map(|argument| self.table.display(*argument).to_string())
                .collect::<Vec<_>>()
                .join(", ");
            return write!(f, "{declaration}[{arguments}]");
        }
        if let Some(name) = &definition.name {
            return f.write_str(name);
        }
        match &definition.kind {
            TypeKind::Pending => write!(f, "<pending-type#{}>", self.id.0),
            TypeKind::Bool => f.write_str("bool"),
            TypeKind::U8 => f.write_str("u8"),
            TypeKind::I8 => f.write_str("i8"),
            TypeKind::U16 => f.write_str("u16"),
            TypeKind::I16 => f.write_str("i16"),
            TypeKind::U32 => f.write_str("u32"),
            TypeKind::I32 => f.write_str("i32"),
            TypeKind::F32 => f.write_str("f32"),
            TypeKind::Pointer(inner) => write!(f, "*{}", self.table.display(*inner)),
            TypeKind::Function(signature) => {
                let parameters = signature
                    .parameters
                    .iter()
                    .map(|ty| self.table.display(*ty).to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                let returns = signature
                    .returns
                    .iter()
                    .map(|ty| self.table.display(*ty).to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "func({parameters}) -> ({returns})")
            }
            TypeKind::Array { length, element } => {
                write!(f, "[{length}]{}", self.table.display(*element))
            }
            TypeKind::Struct(_) => write!(f, "struct#{}", self.id.0),
            TypeKind::Interface(_) => write!(f, "interface#{}", self.id.0),
            TypeKind::Defined { underlying } => self.table.display(*underlying).fmt(f),
        }
    }
}
