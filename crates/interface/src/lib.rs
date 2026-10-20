//! Target-neutral, package-local export data. No AST/HIR/MIR or machine layout.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
mod enum_wire;

pub type TypeRef = u32;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Interface {
    pub package: String,
    pub types: Vec<Type>,
    #[serde(deserialize_with = "unique_map")]
    pub exports: BTreeMap<String, Export>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Type {
    /// Fully qualified nominal identity; None for structural types.
    #[serde(deserialize_with = "required_nullable")]
    pub name: Option<String>,
    pub kind: Kind,
    /// Public concrete methods whose receiver is this nominal type.
    pub methods: Vec<ConcreteMethod>,
}
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub enum Kind {
    Bool,
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    F32,
    Pointer(TypeRef),
    Array { length: u32, element: TypeRef },
    Function(Signature),
    Struct(Vec<Field>),
    Defined(TypeRef),
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Signature {
    pub parameters: Vec<TypeRef>,
    pub returns: Vec<TypeRef>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Field {
    pub name: String,
    pub ty: TypeRef,
    /// Logical package path, not an ephemeral PackageId.
    #[serde(deserialize_with = "required_nullable")]
    pub private_owner: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConcreteMethod {
    pub name: String,
    /// Source-visible signature; the receiver is not included.
    pub signature: Signature,
    pub pointer_receiver: bool,
    pub symbol: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Export {
    Type(TypeRef),
    Constant(Value),
    Global(TypeRef),
    Function(Signature),
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Value {
    pub ty: TypeRef,
    pub kind: ValueKind,
}
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum ValueKind {
    Zero,
    Integer(u64),
    Float32(u32),
    Bool(bool),
    Nil,
    /// Sparse field/index list; absent elements are zero.
    Composite(Vec<(u32, Value)>),
}

fn required_nullable<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    Option::deserialize(d)
}

fn unique_map<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<BTreeMap<String, Export>, D::Error> {
    struct Unique;
    impl<'de> serde::de::Visitor<'de> for Unique {
        type Value = BTreeMap<String, Export>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("an export map with unique keys")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut map: A,
        ) -> Result<Self::Value, A::Error> {
            let mut result = BTreeMap::new();
            while let Some((key, value)) = map.next_entry()? {
                if result.insert(key, value).is_some() {
                    return Err(serde::de::Error::custom("duplicate export key"));
                }
            }
            Ok(result)
        }
    }
    d.deserialize_map(Unique)
}
impl Interface {
    /// Validate all references before exposing untrusted artifact data to a consumer.
    pub fn validate(&self) -> Result<(), String> {
        let check = |id: TypeRef| {
            if (id as usize) < self.types.len() {
                Ok(())
            } else {
                Err(format!("export data references missing type {id}"))
            }
        };
        let signature = |s: &Signature| {
            for id in s.parameters.iter().chain(&s.returns) {
                check(*id)?;
            }
            Ok::<_, String>(())
        };
        let mut names = std::collections::BTreeSet::new();
        for ty in &self.types {
            if ty.name.is_some() != matches!(ty.kind, Kind::Defined(_)) {
                return Err("nominal names require a defined type".into());
            }
            if let Some(name) = &ty.name
                && !names.insert(name)
            {
                return Err(format!("duplicate nominal type {name}"));
            }
            match &ty.kind {
                Kind::Pointer(id) | Kind::Defined(id) => check(*id)?,
                Kind::Array { element, .. } => check(*element)?,
                Kind::Function(s) => signature(s)?,
                Kind::Struct(fields) => {
                    for field in fields {
                        check(field.ty)?;
                    }
                }
                _ => {}
            }
            let mut method_names = std::collections::BTreeSet::new();
            if ty.name.is_none() && !ty.methods.is_empty() {
                return Err("concrete methods require a nominal type".into());
            }
            for method in &ty.methods {
                if !method
                    .name
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_uppercase)
                {
                    return Err(format!("non-public concrete method {}", method.name));
                }
                if !method_names.insert(&method.name) {
                    return Err(format!("duplicate concrete method {}", method.name));
                }
                if method.symbol.is_empty() {
                    return Err(format!("concrete method {} has no symbol", method.name));
                }
                signature(&method.signature)?;
            }
        }
        for id in 0..self.types.len() {
            self.value_kind(id as TypeRef)?;
        }
        let mut complete = std::collections::BTreeSet::new();
        for root in 0..self.types.len() {
            let mut active = std::collections::BTreeSet::new();
            let mut pending = vec![(root as TypeRef, false)];
            while let Some((id, leaving)) = pending.pop() {
                if leaving {
                    active.remove(&id);
                    complete.insert(id);
                    continue;
                }
                if complete.contains(&id) {
                    continue;
                }
                if !active.insert(id) {
                    return Err("direct recursive value type".into());
                }
                pending.push((id, true));
                match &self.types[id as usize].kind {
                    Kind::Defined(next) | Kind::Array { element: next, .. } => {
                        pending.push((*next, false))
                    }
                    Kind::Struct(fields) => {
                        pending.extend(fields.iter().map(|field| (field.ty, false)))
                    }
                    _ => {}
                }
            }
        }
        for (name, export) in &self.exports {
            if !name.as_bytes().first().is_some_and(u8::is_ascii_uppercase) {
                return Err(format!("non-public export name {name}"));
            }
            match export {
                Export::Type(id) | Export::Global(id) => check(*id)?,
                Export::Function(s) => signature(s)?,
                Export::Constant(v) => self.validate_value(v)?,
            }
        }
        Ok(())
    }

    fn value_kind(&self, mut id: TypeRef) -> Result<&Kind, String> {
        let mut seen = std::collections::BTreeSet::new();
        loop {
            if !seen.insert(id) {
                return Err("cyclic defined type".into());
            }
            let ty = self
                .types
                .get(id as usize)
                .ok_or_else(|| format!("export data references missing type {id}"))?;
            match &ty.kind {
                Kind::Defined(next) => id = *next,
                kind => return Ok(kind),
            }
        }
    }

    fn same_type(
        &self,
        left: TypeRef,
        right: TypeRef,
        seen: &mut std::collections::BTreeSet<(TypeRef, TypeRef)>,
    ) -> bool {
        if left == right || !seen.insert((left, right)) {
            return true;
        }
        let (Some(left), Some(right)) = (
            self.types.get(left as usize),
            self.types.get(right as usize),
        ) else {
            return false;
        };
        if left.name.is_some() || right.name.is_some() {
            return left.name == right.name;
        }
        let mut same = |a, b| self.same_type(a, b, seen);
        match (&left.kind, &right.kind) {
            (Kind::Pointer(a), Kind::Pointer(b)) => same(*a, *b),
            (
                Kind::Array {
                    length: a,
                    element: x,
                },
                Kind::Array {
                    length: b,
                    element: y,
                },
            ) => a == b && same(*x, *y),
            (Kind::Struct(a), Kind::Struct(b)) => {
                a.len() == b.len()
                    && a.iter().zip(b).all(|(a, b)| {
                        a.name == b.name && a.private_owner == b.private_owner && same(a.ty, b.ty)
                    })
            }
            (Kind::Function(a), Kind::Function(b)) => {
                a.parameters.len() == b.parameters.len()
                    && a.returns.len() == b.returns.len()
                    && a.parameters
                        .iter()
                        .chain(&a.returns)
                        .zip(b.parameters.iter().chain(&b.returns))
                        .all(|(a, b)| same(*a, *b))
            }
            (a, b) => a == b,
        }
    }

    fn validate_value(&self, value: &Value) -> Result<(), String> {
        let kind = self.value_kind(value.ty)?;
        let valid = match (&value.kind, kind) {
            (ValueKind::Zero, _) => true,
            (ValueKind::Integer(value), Kind::U8) => *value <= u8::MAX as u64,
            (ValueKind::Integer(value), Kind::U16) => *value <= u16::MAX as u64,
            (ValueKind::Integer(value), Kind::U32) => *value <= u32::MAX as u64,
            (ValueKind::Integer(value), Kind::I8) => *value == *value as i8 as i64 as u64,
            (ValueKind::Integer(value), Kind::I16) => *value == *value as i16 as i64 as u64,
            (ValueKind::Integer(value), Kind::I32) => *value == *value as i32 as i64 as u64,
            (ValueKind::Float32(_), Kind::F32)
            | (ValueKind::Bool(_), Kind::Bool)
            | (ValueKind::Nil, Kind::Pointer(_) | Kind::Function(_)) => true,
            (ValueKind::Composite(elements), Kind::Array { .. } | Kind::Struct(_)) => {
                let mut seen = std::collections::BTreeSet::new();
                for (index, child) in elements {
                    if !seen.insert(index) {
                        return Err("duplicate aggregate constant index".into());
                    }
                    let expected = match kind {
                        Kind::Array { length, element } if index < length => *element,
                        Kind::Struct(fields) => {
                            fields
                                .get(*index as usize)
                                .ok_or("aggregate constant index out of range")?
                                .ty
                        }
                        _ => return Err("aggregate constant index out of range".into()),
                    };
                    if !self.same_type(child.ty, expected, &mut Default::default()) {
                        return Err("aggregate constant child type mismatch".into());
                    }
                    self.validate_value(child)?;
                }
                true
            }
            _ => false,
        };
        if valid {
            Ok(())
        } else {
            Err("constant payload does not match its type".into())
        }
    }
}
