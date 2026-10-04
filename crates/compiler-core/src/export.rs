//! Export only public bindings and their transitive type/constant closure.
use crate::{
    ir::hir,
    semantic::{TypeId, TypeKind},
};
use ond_interface::{
    ConcreteMethod, Export, Field, Interface, Kind, Method, Signature, Type, Value, ValueKind,
};
use std::collections::{BTreeMap, BTreeSet};

pub fn package(project: &hir::Project, path: &str) -> Result<Interface, String> {
    let package = project
        .packages
        .iter()
        .find(|p| p.logical_path == path)
        .ok_or_else(|| format!("missing export package {path}"))?;
    let mut writer = Writer {
        project,
        ids: BTreeMap::new(),
        types: Vec::new(),
    };
    let mut exports = BTreeMap::new();
    let public = |s: &str| s.as_bytes().first().is_some_and(u8::is_ascii_uppercase);
    for item in &package.items {
        match item {
            hir::Item::Type(item) if public(&item.name) => {
                exports.insert(item.name.clone(), Export::Type(writer.ty(item.ty)?));
            }
            hir::Item::Func(item) if public(&item.name) => {
                let signature = writer.signature(
                    item.signature.parameters.iter().map(|p| p.ty),
                    item.signature.returns.iter().copied(),
                )?;
                exports.insert(item.name.clone(), Export::Function(signature));
            }
            hir::Item::Const(item) => {
                for (name, value) in item.names.iter().zip(&item.values) {
                    if public(name) {
                        exports.insert(name.clone(), Export::Constant(writer.value(value)?));
                    }
                }
            }
            hir::Item::Var(item) => {
                for (name, global) in item.names.iter().zip(&item.globals) {
                    if public(name)
                        && let Some(global) = global
                    {
                        exports.insert(name.clone(), Export::Global(writer.ty(global.ty)?));
                    }
                }
            }
            _ => {}
        }
    }
    let mut attached = BTreeSet::new();
    loop {
        let mut progress = false;
        for item in &package.items {
            let hir::Item::Func(item) = item else {
                continue;
            };
            let Some(receiver) = item.receiver else {
                continue;
            };
            if !public(&item.name) || attached.contains(&item.canonical_name) {
                continue;
            }
            let (base, pointer_receiver) = match project.types.get(receiver).map(|ty| &ty.kind) {
                Some(TypeKind::Pointer(base)) => (*base, true),
                Some(_) => (receiver, false),
                None => return Err(format!("missing method receiver type {receiver}")),
            };
            let Some(local) = writer.ids.get(&base).copied() else {
                continue;
            };
            let signature = writer.signature(
                item.signature
                    .parameters
                    .iter()
                    .skip(1)
                    .map(|parameter| parameter.ty),
                item.signature.returns.iter().copied(),
            )?;
            writer.types[local as usize].methods.push(ConcreteMethod {
                name: item.name.clone(),
                signature,
                pointer_receiver,
                symbol: item.canonical_name.clone(),
            });
            attached.insert(item.canonical_name.clone());
            progress = true;
        }
        if !progress {
            break;
        }
    }
    let interface = Interface {
        package: path.into(),
        types: writer.types,
        exports,
    };
    interface.validate()?;
    Ok(interface)
}
struct Writer<'a> {
    project: &'a hir::Project,
    ids: BTreeMap<TypeId, u32>,
    types: Vec<Type>,
}
impl Writer<'_> {
    fn signature(
        &mut self,
        parameters: impl Iterator<Item = TypeId>,
        returns: impl Iterator<Item = TypeId>,
    ) -> Result<Signature, String> {
        Ok(Signature {
            parameters: parameters.map(|ty| self.ty(ty)).collect::<Result<_, _>>()?,
            returns: returns.map(|ty| self.ty(ty)).collect::<Result<_, _>>()?,
        })
    }
    fn ty(&mut self, id: TypeId) -> Result<u32, String> {
        if let Some(id) = self.ids.get(&id) {
            return Ok(*id);
        }
        let definition = self
            .project
            .types
            .get(id)
            .ok_or_else(|| format!("missing type {id}"))?
            .clone();
        let local = u32::try_from(self.types.len()).map_err(|e| e.to_string())?;
        self.ids.insert(id, local);
        // Reserve before traversing pointer-recursive nominal definitions.
        self.types.push(Type {
            name: None,
            kind: Kind::Bool,
            methods: Vec::new(),
        });
        let kind = match definition.kind {
            TypeKind::Pending => return Err("cannot export pending type".into()),
            TypeKind::Bool => Kind::Bool,
            TypeKind::U8 => Kind::U8,
            TypeKind::I8 => Kind::I8,
            TypeKind::U16 => Kind::U16,
            TypeKind::I16 => Kind::I16,
            TypeKind::U32 => Kind::U32,
            TypeKind::I32 => Kind::I32,
            TypeKind::F32 => Kind::F32,
            TypeKind::Pointer(p) => Kind::Pointer(self.ty(p)?),
            TypeKind::Array { length, element } => Kind::Array {
                length,
                element: self.ty(element)?,
            },
            TypeKind::Defined { underlying } => Kind::Defined(self.ty(underlying)?),
            TypeKind::Function(s) => {
                Kind::Function(self.signature(s.parameters.into_iter(), s.returns.into_iter())?)
            }
            TypeKind::Struct(s) => {
                let mut fields = Vec::new();
                for field in s.fields {
                    let owner = field
                        .private_owner
                        .map(|id| {
                            self.project
                                .packages
                                .iter()
                                .find(|p| p.id == id)
                                .map(|p| p.logical_path.clone())
                                .ok_or_else(|| "missing field owner package".to_string())
                        })
                        .transpose()?;
                    fields.push(Field {
                        name: field.name,
                        ty: self.ty(field.ty)?,
                        private_owner: owner,
                    });
                }
                Kind::Struct(fields)
            }
            TypeKind::Interface(interface) => {
                let mut methods = Vec::new();
                for method in interface.methods {
                    let owner = method
                        .private_owner
                        .map(|id| {
                            self.project
                                .packages
                                .iter()
                                .find(|package| package.id == id)
                                .map(|package| package.logical_path.clone())
                                .ok_or_else(|| "missing method owner package".to_string())
                        })
                        .transpose()?;
                    methods.push(Method {
                        name: method.name,
                        signature: self.signature(
                            method.signature.parameters.into_iter().skip(1),
                            method.signature.returns.into_iter(),
                        )?,
                        private_owner: owner,
                    });
                }
                Kind::Interface(methods)
            }
        };
        self.types[local as usize] = Type {
            name: if matches!(kind, Kind::Defined(_)) {
                definition.name
            } else {
                None
            },
            kind,
            methods: Vec::new(),
        };
        Ok(local)
    }
    fn value(&mut self, expr: &hir::Expr) -> Result<Value, String> {
        let kind = match &expr.kind {
            hir::ExprKind::Zero => ValueKind::Zero,
            hir::ExprKind::Integer(v) => ValueKind::Integer(*v),
            hir::ExprKind::Float32(v) => ValueKind::Float32(*v),
            hir::ExprKind::Bool(v) => ValueKind::Bool(*v),
            hir::ExprKind::Nil => ValueKind::Nil,
            hir::ExprKind::Composite(items) => ValueKind::Composite(
                items
                    .iter()
                    .map(|(index, value)| Ok((*index, self.value(value)?)))
                    .collect::<Result<_, String>>()?,
            ),
            _ => return Err("export constant was not fully evaluated".into()),
        };
        Ok(Value {
            ty: self.ty(expr.ty)?,
            kind,
        })
    }
}
