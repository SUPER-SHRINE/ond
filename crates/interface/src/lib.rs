//! Target-neutral, package-local export data. No AST/HIR/MIR or machine layout.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type TypeRef = u32;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Interface {
    pub package: String,
    pub types: Vec<Type>,
    pub exports: BTreeMap<String, Export>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Type {
    /// Fully qualified nominal identity; None for structural types.
    pub name: Option<String>,
    pub kind: Kind,
    /// Public concrete methods whose receiver is this nominal type.
    pub methods: Vec<ConcreteMethod>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
    Interface(Vec<Method>),
    Defined(TypeRef),
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Signature {
    pub parameters: Vec<TypeRef>,
    pub returns: Vec<TypeRef>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub ty: TypeRef,
    /// Logical package path, not an ephemeral PackageId.
    pub private_owner: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Method {
    pub name: String,
    pub signature: Signature,
    pub private_owner: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
pub struct Value {
    pub ty: TypeRef,
    pub kind: ValueKind,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ValueKind {
    Zero,
    Integer(u64),
    Float32(u32),
    Bool(bool),
    Nil,
    /// Sparse field/index list; absent elements are zero.
    Composite(Vec<(u32, Value)>),
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
                Kind::Interface(methods) => {
                    let mut names = std::collections::BTreeSet::new();
                    for method in methods {
                        if !names.insert(&method.name) {
                            return Err(format!("duplicate interface method {}", method.name));
                        }
                        signature(&method.signature)?;
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
        fn value(v: &Value, check: &impl Fn(TypeRef) -> Result<(), String>) -> Result<(), String> {
            check(v.ty)?;
            if let ValueKind::Composite(elements) = &v.kind {
                let mut seen = std::collections::BTreeSet::new();
                for (index, v) in elements {
                    if !seen.insert(index) {
                        return Err("duplicate aggregate constant index".into());
                    }
                    value(v, check)?;
                }
            }
            Ok(())
        }
        for (name, export) in &self.exports {
            if !name.as_bytes().first().is_some_and(u8::is_ascii_uppercase) {
                return Err(format!("non-public export name {name}"));
            }
            match export {
                Export::Type(id) | Export::Global(id) => check(*id)?,
                Export::Function(s) => signature(s)?,
                Export::Constant(v) => value(v, &check)?,
            }
        }
        Ok(())
    }
}
