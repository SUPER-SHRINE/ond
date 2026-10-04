//! Package declaration index and target-neutral type resolution.
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::ir::{ast, hir};
use crate::project::{LoadedProject, PackageId};
use crate::source::{FileId, Span};
use std::collections::{BTreeMap, BTreeSet};

use super::{body::BodyLowerer, canonical_symbol_name};

pub(super) fn method_receiver_name(receiver: &ast::Field) -> Option<(&str, bool)> {
    match &receiver.ty {
        ast::Type::Named(path) if path.segments.len() == 1 => Some((&path.segments[0].name, false)),
        ast::Type::Apply { base, .. } if base.segments.len() == 1 => {
            Some((&base.segments[0].name, false))
        }
        ast::Type::Pointer(inner, _) => match inner.as_ref() {
            ast::Type::Named(path) if path.segments.len() == 1 => {
                Some((&path.segments[0].name, true))
            }
            ast::Type::Apply { base, .. } if base.segments.len() == 1 => {
                Some((&base.segments[0].name, true))
            }
            _ => None,
        },
        _ => None,
    }
}

#[derive(Clone)]
pub(super) struct GenericMethodReceiver {
    pub(super) name: String,
    pub(super) pointer: bool,
    pub(super) parameters: Vec<ast::Ident>,
}

pub(super) fn generic_method_receiver(receiver: &ast::Field) -> Option<GenericMethodReceiver> {
    let (ty, pointer) = match &receiver.ty {
        ast::Type::Pointer(inner, _) => (inner.as_ref(), true),
        ty => (ty, false),
    };
    let ast::Type::Apply {
        base, arguments, ..
    } = ty
    else {
        return None;
    };
    let [base] = base.segments.as_slice() else {
        return None;
    };
    let parameters = arguments
        .iter()
        .map(|argument| match argument {
            ast::Type::Named(path) if matches!(path.segments.as_slice(), [_]) => {
                Some(path.segments[0].clone())
            }
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    Some(GenericMethodReceiver {
        name: base.name.clone(),
        pointer,
        parameters,
    })
}

#[derive(Clone)]
pub(super) struct TypeDeclaration {
    pub(super) ty: ast::Type,
    pub(super) type_params: Vec<ast::Ident>,
    pub(super) span: Span,
    pub(super) type_id: Option<crate::semantic::TypeId>,
}

pub(super) struct TypePackage {
    pub(super) id: PackageId,
    pub(super) logical_path: String,
    pub(super) imports: BTreeMap<FileId, BTreeMap<String, PackageId>>,
    pub(super) declarations: BTreeMap<String, TypeDeclaration>,
    pub(super) constants: BTreeMap<String, (Option<ast::Type>, ast::Expr)>,
    pub(super) values: BTreeSet<String>,
    pub(super) functions: BTreeMap<String, ast::Signature>,
    pub(super) generic_functions: BTreeMap<String, ast::FuncDecl>,
    pub(super) methods: Vec<ast::FuncDecl>,
    pub(super) operators: Vec<ast::OperatorDecl>,
    pub(super) globals: BTreeMap<String, ast::VarSpec>,
}

pub(super) fn collect_type_universe(
    loaded: &LoadedProject,
    ast: &ast::Project,
    logical_to_package: &BTreeMap<String, PackageId>,
    types: &mut crate::semantic::TypeTable,
) -> Vec<TypePackage> {
    ast.packages
        .iter()
        .filter_map(|package| {
            let source = loaded
                .packages
                .iter()
                .find(|candidate| candidate.logical_path == package.logical_path)?;
            let imports = package
                .files
                .iter()
                .map(|file| {
                    let names = file
                        .imports
                        .iter()
                        .filter_map(|import| {
                            let path = crate::string_literal::import_path(&import.path).ok()?;
                            let id = logical_to_package.get(&path).copied()?;
                            let name = import
                                .alias
                                .as_ref()
                                .map(|alias| alias.name.clone())
                                .unwrap_or_else(|| {
                                    path.rsplit('/').next().unwrap_or_default().to_string()
                                });
                            Some((name, id))
                        })
                        .collect();
                    (file.file_id, names)
                })
                .collect();
            let mut declarations = BTreeMap::new();
            let mut constants = BTreeMap::new();
            let mut values = BTreeSet::new();
            let mut functions = BTreeMap::new();
            let mut generic_functions = BTreeMap::new();
            let mut methods = Vec::new();
            let mut operators = Vec::new();
            let mut globals = BTreeMap::new();
            for file in &package.files {
                for decl in &file.decls {
                    match decl {
                        ast::TopLevelDecl::Const(decl) => {
                            for spec in &decl.specs {
                                for (name, value) in spec.names.iter().zip(&spec.values) {
                                    if name.name == "_" {
                                        continue;
                                    }
                                    constants.insert(
                                        name.name.clone(),
                                        (spec.ty.clone(), value.clone()),
                                    );
                                }
                            }
                        }
                        ast::TopLevelDecl::Var(decl) => {
                            for spec in &decl.specs {
                                values.extend(
                                    spec.names
                                        .iter()
                                        .filter(|n| n.name != "_")
                                        .map(|name| name.name.clone()),
                                );
                                for name in &spec.names {
                                    if name.name == "_" {
                                        continue;
                                    }
                                    globals.insert(name.name.clone(), spec.clone());
                                }
                            }
                        }
                        ast::TopLevelDecl::Func(decl) => {
                            if decl.receiver.is_some() {
                                methods.push(decl.clone());
                            } else if !decl.type_params.is_empty() {
                                values.insert(decl.name.name.clone());
                                generic_functions.insert(decl.name.name.clone(), decl.clone());
                            } else {
                                values.insert(decl.name.name.clone());
                                functions.insert(decl.name.name.clone(), decl.signature.clone());
                            }
                        }
                        ast::TopLevelDecl::Operator(decl) => operators.push(decl.clone()),
                        _ => {}
                    }
                    let ast::TopLevelDecl::Type(decl) = decl else {
                        continue;
                    };
                    for spec in &decl.specs {
                        if spec.name.name == "_" {
                            continue;
                        }
                        let canonical_name =
                            canonical_symbol_name(&package.logical_path, &spec.name.name);
                        let type_id = spec
                            .type_params
                            .is_empty()
                            .then(|| types.reserve_defined(canonical_name, spec.span));
                        declarations
                            .entry(spec.name.name.clone())
                            .or_insert(TypeDeclaration {
                                ty: spec.ty.clone(),
                                type_params: spec.type_params.clone(),
                                span: spec.span,
                                type_id,
                            });
                    }
                }
            }
            Some(TypePackage {
                id: source.id,
                logical_path: package.logical_path.clone(),
                imports,
                declarations,
                constants,
                values,
                functions,
                generic_functions,
                methods,
                operators,
                globals,
            })
        })
        .collect()
}

pub(super) fn resolve_signature(
    package_id: PackageId,
    signature: &ast::Signature,
    universe: &[TypePackage],
    types: &mut crate::semantic::TypeTable,
    diagnostics: &mut Diagnostics,
) -> Option<hir::Signature> {
    let mut valid = true;
    let parameters = signature
        .params
        .iter()
        .filter_map(|parameter| {
            let ty = resolve_type(
                package_id,
                &parameter.ty,
                universe,
                types,
                false,
                &mut BTreeSet::new(),
                diagnostics,
            );
            valid &= ty.is_some();
            ty.map(|ty| hir::Parameter {
                name: parameter.name.name.clone(),
                ty,
                span: parameter.span,
            })
        })
        .collect();
    let returns = signature
        .results
        .iter()
        .filter_map(|result| {
            let ty = resolve_type(
                package_id,
                result,
                universe,
                types,
                false,
                &mut BTreeSet::new(),
                diagnostics,
            );
            valid &= ty.is_some();
            ty
        })
        .collect();
    valid.then_some(hir::Signature {
        parameters,
        returns,
        span: signature.span,
    })
}

pub(super) fn resolve_method_signature(
    package_id: PackageId,
    receiver: &ast::Field,
    signature: &ast::Signature,
    universe: &[TypePackage],
    types: &mut crate::semantic::TypeTable,
    diagnostics: &mut Diagnostics,
) -> Option<hir::Signature> {
    let mut resolved = resolve_signature(package_id, signature, universe, types, diagnostics)?;
    let receiver_ty = resolve_type(
        package_id,
        &receiver.ty,
        universe,
        types,
        false,
        &mut BTreeSet::new(),
        diagnostics,
    )?;
    resolved.parameters.insert(
        0,
        hir::Parameter {
            name: receiver.name.name.clone(),
            ty: receiver_ty,
            span: receiver.span,
        },
    );
    Some(resolved)
}

pub(super) fn resolve_defined_type(
    package_id: PackageId,
    name: &str,
    universe: &[TypePackage],
    types: &mut crate::semantic::TypeTable,
    indirected: bool,
    visiting: &mut BTreeSet<crate::semantic::TypeId>,
    diagnostics: &mut Diagnostics,
) -> Option<crate::semantic::TypeId> {
    use crate::semantic::TypeKind;

    let package = universe.iter().find(|package| package.id == package_id)?;
    let declaration = package.declarations.get(name)?;
    let type_id = declaration.type_id?;
    if !matches!(types.kind(type_id), Some(TypeKind::Pending)) {
        return Some(type_id);
    }
    if visiting.contains(&type_id) {
        if indirected {
            return Some(type_id);
        }
        diagnostics.push(Diagnostic::error(
            declaration.span,
            format!("direct recursive type `{name}` is not allowed"),
        ));
        return None;
    }
    visiting.insert(type_id);
    let underlying = match &declaration.ty {
        ast::Type::Interface { methods, span } => resolve_interface_type(
            package_id,
            methods,
            *span,
            universe,
            types,
            visiting,
            diagnostics,
        ),
        ty => resolve_type(
            package_id,
            ty,
            universe,
            types,
            false,
            visiting,
            diagnostics,
        ),
    };
    visiting.remove(&type_id);
    if let Some(underlying) = underlying {
        types.define(type_id, TypeKind::Defined { underlying });
        Some(type_id)
    } else {
        None
    }
}

fn resolve_interface_type(
    package_id: PackageId,
    methods: &[ast::InterfaceMethod],
    span: Span,
    universe: &[TypePackage],
    types: &mut crate::semantic::TypeTable,
    visiting: &mut BTreeSet<crate::semantic::TypeId>,
    diagnostics: &mut Diagnostics,
) -> Option<crate::semantic::TypeId> {
    use crate::semantic::{
        FunctionType, InterfaceMethod, InterfaceType, StructField, StructType, TypeKind,
    };

    let data_pointer = types.pointer(crate::semantic::TypeId::U8, span);
    let mut names = BTreeSet::new();
    let mut resolved = Vec::new();
    let mut fields = Vec::new();
    let mut valid = true;
    for method in methods {
        if !names.insert(method.name.name.clone()) {
            diagnostics.push(Diagnostic::error(
                method.name.span,
                format!("duplicate interface method `{}`", method.name.name),
            ));
            valid = false;
            continue;
        }
        let mut parameters = vec![data_pointer];
        for parameter in &method.signature.params {
            let ty = resolve_type(
                package_id,
                &parameter.ty,
                universe,
                types,
                true,
                visiting,
                diagnostics,
            );
            valid &= ty.is_some();
            if let Some(ty) = ty {
                parameters.push(ty);
            }
        }
        let mut returns = Vec::new();
        for result in &method.signature.results {
            let ty = resolve_type(
                package_id,
                result,
                universe,
                types,
                true,
                visiting,
                diagnostics,
            );
            valid &= ty.is_some();
            if let Some(ty) = ty {
                returns.push(ty);
            }
        }
        let signature = FunctionType {
            parameters,
            returns,
        };
        let function = types.function(signature.clone(), method.span);
        fields.push(StructField {
            name: method.name.name.clone(),
            ty: function,
            private_owner: None,
            span: method.span,
        });
        resolved.push(InterfaceMethod {
            name: method.name.name.clone(),
            signature,
            private_owner: None,
        });
    }
    if !valid {
        return None;
    }
    let vtable = types.intern(TypeKind::Struct(StructType { fields }), span);
    let vtable = types.pointer(vtable, span);
    Some(types.intern(
        TypeKind::Interface(InterfaceType {
            methods: resolved,
            data_pointer,
            vtable,
        }),
        span,
    ))
}

#[allow(clippy::too_many_arguments)]
fn resolve_applied_type(
    caller_package: PackageId,
    base: &ast::Path,
    arguments: &[ast::Type],
    span: Span,
    universe: &[TypePackage],
    types: &mut crate::semantic::TypeTable,
    indirected: bool,
    visiting: &mut BTreeSet<crate::semantic::TypeId>,
    diagnostics: &mut Diagnostics,
) -> Option<crate::semantic::TypeId> {
    let caller = universe
        .iter()
        .find(|package| package.id == caller_package)?;
    let (owner, name) = match base.segments.as_slice() {
        [name] => (caller, name.name.as_str()),
        [qualifier, name] => {
            if !name
                .name
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_uppercase)
            {
                diagnostics.push(Diagnostic::error(
                    base.span,
                    format!("`{}.{}` is not exported", qualifier.name, name.name),
                ));
                return None;
            }
            let owner_id = caller
                .imports
                .get(&base.span.file)
                .and_then(|imports| imports.get(&qualifier.name))?;
            let owner = universe.iter().find(|package| package.id == *owner_id)?;
            (owner, name.name.as_str())
        }
        _ => {
            diagnostics.push(Diagnostic::error(base.span, "invalid generic type name"));
            return None;
        }
    };
    let Some(declaration) = owner.declarations.get(name) else {
        diagnostics.push(Diagnostic::error(
            base.span,
            format!("unknown generic type `{name}`"),
        ));
        return None;
    };
    if declaration.type_params.is_empty() {
        diagnostics.push(Diagnostic::error(
            span,
            format!("type `{name}` is not generic"),
        ));
        return None;
    }
    if declaration.type_params.len() != arguments.len() {
        diagnostics.push(Diagnostic::error(
            span,
            format!(
                "generic type `{name}` expects {} type arguments, got {}",
                declaration.type_params.len(),
                arguments.len()
            ),
        ));
        return None;
    }
    let resolved_arguments = arguments
        .iter()
        .map(|argument| {
            resolve_type(
                caller_package,
                argument,
                universe,
                types,
                false,
                visiting,
                diagnostics,
            )
        })
        .collect::<Option<Vec<_>>>()?;
    let instance_name = format!(
        "{}.{}[{}]",
        owner.logical_path,
        name,
        resolved_arguments
            .iter()
            .map(|ty| ty.0.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
    let existing = types.find_named(&instance_name);
    if let Some(id) = existing {
        if !matches!(types.kind(id), Some(crate::semantic::TypeKind::Pending)) {
            return Some(id);
        }
        if visiting.contains(&id) {
            if indirected {
                return Some(id);
            }
            diagnostics.push(Diagnostic::error(
                span,
                format!("direct recursive type `{name}` is not allowed"),
            ));
            return None;
        }
    }
    let declaration_name = format!("{}.{}", owner.logical_path, name);
    let type_id = existing.unwrap_or_else(|| {
        types.reserve_generic_defined(
            instance_name,
            declaration_name,
            resolved_arguments.clone(),
            span,
        )
    });
    visiting.insert(type_id);
    let substitutions = declaration
        .type_params
        .iter()
        .map(|parameter| parameter.name.as_str())
        .zip(resolved_arguments.iter().copied())
        .collect::<BTreeMap<_, _>>();
    let concrete = substitute_type(&declaration.ty, &substitutions);
    let mut instance_diagnostics = Diagnostics::new();
    let underlying = match &concrete {
        ast::Type::Interface { methods, span } => resolve_interface_type(
            owner.id,
            methods,
            *span,
            universe,
            types,
            visiting,
            &mut instance_diagnostics,
        ),
        ty => resolve_type(
            owner.id,
            ty,
            universe,
            types,
            false,
            visiting,
            &mut instance_diagnostics,
        ),
    };
    visiting.remove(&type_id);
    for diagnostic in instance_diagnostics {
        diagnostics.push(diagnostic.reanchor(
            span,
            format!("while instantiating generic type `{name}` declared here"),
        ));
    }
    if let Some(underlying) = underlying {
        types.define(type_id, crate::semantic::TypeKind::Defined { underlying });
        Some(type_id)
    } else {
        None
    }
}

pub(super) fn substitute_type(
    ty: &ast::Type,
    substitutions: &BTreeMap<&str, crate::semantic::TypeId>,
) -> ast::Type {
    match ty {
        ast::Type::Named(path) if path.segments.len() == 1 => substitutions
            .get(path.segments[0].name.as_str())
            .map_or_else(|| ty.clone(), |id| ast::Type::Resolved(*id, path.span)),
        ast::Type::Named(_) | ast::Type::Resolved(_, _) => ty.clone(),
        ast::Type::Apply {
            base,
            arguments,
            span,
        } => ast::Type::Apply {
            base: base.clone(),
            arguments: arguments
                .iter()
                .map(|argument| substitute_type(argument, substitutions))
                .collect(),
            span: *span,
        },
        ast::Type::Pointer(inner, span) => {
            ast::Type::Pointer(Box::new(substitute_type(inner, substitutions)), *span)
        }
        ast::Type::Array { len, element, span } => ast::Type::Array {
            len: len.clone(),
            element: Box::new(substitute_type(element, substitutions)),
            span: *span,
        },
        ast::Type::Struct { fields, span } => ast::Type::Struct {
            fields: fields
                .iter()
                .map(|field| ast::Field {
                    name: field.name.clone(),
                    ty: substitute_type(&field.ty, substitutions),
                    span: field.span,
                })
                .collect(),
            span: *span,
        },
        ast::Type::Interface { methods, span } => ast::Type::Interface {
            methods: methods
                .iter()
                .map(|method| ast::InterfaceMethod {
                    name: method.name.clone(),
                    signature: substitute_type_signature(&method.signature, substitutions),
                    span: method.span,
                })
                .collect(),
            span: *span,
        },
        ast::Type::Func { signature, span } => ast::Type::Func {
            signature: substitute_type_signature(signature, substitutions),
            span: *span,
        },
    }
}

pub(super) fn substitute_signature(
    signature: &ast::Signature,
    parameters: &[ast::Ident],
    arguments: &[crate::semantic::TypeId],
) -> ast::Signature {
    let substitutions = parameters
        .iter()
        .map(|parameter| parameter.name.as_str())
        .zip(arguments.iter().copied())
        .collect::<BTreeMap<_, _>>();
    ast::Signature {
        params: signature
            .params
            .iter()
            .map(|parameter| ast::Field {
                name: parameter.name.clone(),
                ty: substitute_type(&parameter.ty, &substitutions),
                span: parameter.span,
            })
            .collect(),
        results: signature
            .results
            .iter()
            .map(|result| substitute_type(result, &substitutions))
            .collect(),
        span: signature.span,
    }
}

fn substitute_type_signature(
    signature: &ast::TypeSignature,
    substitutions: &BTreeMap<&str, crate::semantic::TypeId>,
) -> ast::TypeSignature {
    ast::TypeSignature {
        params: signature
            .params
            .iter()
            .map(|parameter| ast::TypeParameter {
                name: parameter.name.clone(),
                ty: substitute_type(&parameter.ty, substitutions),
                span: parameter.span,
            })
            .collect(),
        results: signature
            .results
            .iter()
            .map(|result| substitute_type(result, substitutions))
            .collect(),
        span: signature.span,
    }
}

pub(super) fn resolve_type(
    package_id: PackageId,
    ty: &ast::Type,
    universe: &[TypePackage],
    types: &mut crate::semantic::TypeTable,
    indirected: bool,
    visiting: &mut BTreeSet<crate::semantic::TypeId>,
    diagnostics: &mut Diagnostics,
) -> Option<crate::semantic::TypeId> {
    use crate::semantic::{FunctionType, StructField, StructType, TypeId, TypeKind};

    match ty {
        ast::Type::Resolved(id, _) => Some(*id),
        ast::Type::Apply {
            base,
            arguments,
            span,
        } => resolve_applied_type(
            package_id,
            base,
            arguments,
            *span,
            universe,
            types,
            indirected,
            visiting,
            diagnostics,
        ),
        ast::Type::Named(path) if path.segments.len() == 1 => {
            let name = &path.segments[0].name;
            let builtin = match name.as_str() {
                "bool" => Some(TypeId::BOOL),
                "u8" => Some(TypeId::U8),
                "i8" => Some(TypeId::I8),
                "u16" => Some(TypeId::U16),
                "i16" => Some(TypeId::I16),
                "u32" => Some(TypeId::U32),
                "i32" => Some(TypeId::I32),
                "f32" => Some(TypeId::F32),
                _ => None,
            };
            if builtin.is_some() {
                builtin
            } else if universe
                .iter()
                .find(|package| package.id == package_id)
                .is_some_and(|package| package.declarations.contains_key(name))
            {
                let declaration = universe
                    .iter()
                    .find(|package| package.id == package_id)
                    .and_then(|package| package.declarations.get(name))?;
                if !declaration.type_params.is_empty() {
                    diagnostics.push(Diagnostic::error(
                        path.span,
                        format!("generic type `{name}` requires type arguments"),
                    ));
                    return None;
                }
                resolve_defined_type(
                    package_id,
                    name,
                    universe,
                    types,
                    indirected,
                    visiting,
                    diagnostics,
                )
            } else {
                diagnostics.push(Diagnostic::error(
                    path.span,
                    format!("unknown type `{name}`"),
                ));
                None
            }
        }
        ast::Type::Named(path) if path.segments.len() == 2 => {
            let qualifier = &path.segments[0].name;
            let name = &path.segments[1].name;
            if !name.as_bytes().first().is_some_and(u8::is_ascii_uppercase) {
                diagnostics.push(Diagnostic::error(
                    path.span,
                    format!("`{qualifier}.{name}` is not exported"),
                ));
                return None;
            }
            let current = universe.iter().find(|package| package.id == package_id)?;
            let target_id = current
                .imports
                .get(&path.span.file)
                .and_then(|imports| imports.get(qualifier))
                .copied();
            let target = universe
                .iter()
                .find(|candidate| Some(candidate.id) == target_id);
            match target {
                Some(target) if target.declarations.contains_key(name) => {
                    if target
                        .declarations
                        .get(name)
                        .is_some_and(|declaration| !declaration.type_params.is_empty())
                    {
                        diagnostics.push(Diagnostic::error(
                            path.span,
                            format!("generic type `{qualifier}.{name}` requires type arguments"),
                        ));
                        None
                    } else {
                        resolve_defined_type(
                            target.id,
                            name,
                            universe,
                            types,
                            indirected,
                            visiting,
                            diagnostics,
                        )
                    }
                }
                _ => {
                    diagnostics.push(Diagnostic::error(
                        path.span,
                        format!("unknown imported type `{qualifier}.{name}`"),
                    ));
                    None
                }
            }
        }
        ast::Type::Named(path) => {
            diagnostics.push(Diagnostic::error(path.span, "invalid qualified type name"));
            None
        }
        ast::Type::Pointer(inner, span) => {
            let inner = resolve_type(
                package_id,
                inner,
                universe,
                types,
                true,
                visiting,
                diagnostics,
            )?;
            Some(types.pointer(inner, *span))
        }
        ast::Type::Array { len, element, span } => {
            let Some(len) = len else {
                diagnostics.push(Diagnostic::error(
                    *span,
                    "inferred array length requires a composite literal",
                ));
                return None;
            };
            let length = if let Some(length) = resolve_array_length(len) {
                Some(length)
            } else {
                let mut lowerer = BodyLowerer::new(package_id, universe, types);
                let evaluated = lowerer.constant_expr(len, None);
                match evaluated {
                    Some(hir::Expr {
                        kind: hir::ExprKind::Integer(value),
                        ..
                    }) => match u32::try_from(value) {
                        Ok(length) => Some(length),
                        Err(_) => {
                            diagnostics.push(Diagnostic::error(
                                *span,
                                "array length must be a non-negative integer constant within u32 range",
                            ));
                            None
                        }
                    },
                    _ => {
                        diagnostics.push(lowerer.failure.unwrap_or_else(|| {
                            Diagnostic::error(
                                *span,
                                "array length must be a non-negative integer constant",
                            )
                        }));
                        None
                    }
                }
            };
            let element = resolve_type(
                package_id,
                element,
                universe,
                types,
                indirected,
                visiting,
                diagnostics,
            );
            match (length, element) {
                (Some(length), Some(element)) => {
                    Some(types.intern(TypeKind::Array { length, element }, *span))
                }
                _ => None,
            }
        }
        ast::Type::Struct { fields, span } => {
            let mut resolved = Vec::new();
            let mut names = BTreeSet::new();
            for field in fields {
                if !names.insert(&field.name.name) {
                    diagnostics.push(Diagnostic::error(
                        field.span,
                        format!("duplicate struct field `{}`", field.name.name),
                    ));
                    return None;
                }
                let ty = resolve_type(
                    package_id,
                    &field.ty,
                    universe,
                    types,
                    indirected,
                    visiting,
                    diagnostics,
                )?;
                resolved.push(StructField {
                    name: field.name.name.clone(),
                    ty,
                    private_owner: (!field
                        .name
                        .name
                        .as_bytes()
                        .first()
                        .is_some_and(u8::is_ascii_uppercase))
                    .then_some(package_id),
                    span: field.span,
                });
            }
            Some(types.intern(TypeKind::Struct(StructType { fields: resolved }), *span))
        }
        ast::Type::Interface { span, .. } => {
            diagnostics.push(Diagnostic::error(
                *span,
                "interface types must be declared with `type Name interface { ... }`",
            ));
            None
        }
        ast::Type::Func { signature, span } => {
            let mut parameters = Vec::new();
            for parameter in &signature.params {
                parameters.push(resolve_type(
                    package_id,
                    &parameter.ty,
                    universe,
                    types,
                    true,
                    visiting,
                    diagnostics,
                )?);
            }
            let mut returns = Vec::new();
            for result in &signature.results {
                returns.push(resolve_type(
                    package_id,
                    result,
                    universe,
                    types,
                    true,
                    visiting,
                    diagnostics,
                )?);
            }
            Some(types.function(
                FunctionType {
                    parameters,
                    returns,
                },
                *span,
            ))
        }
    }
}

pub(super) fn resolve_all_defined_types(
    universe: &[TypePackage],
    types: &mut crate::semantic::TypeTable,
    diagnostics: &mut Diagnostics,
) {
    let names = universe
        .iter()
        .flat_map(|package| {
            package
                .declarations
                .keys()
                .cloned()
                .map(|name| (package.id, name))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    for (package, name) in names {
        let _ = resolve_defined_type(
            package,
            &name,
            universe,
            types,
            false,
            &mut BTreeSet::new(),
            diagnostics,
        );
    }
}

fn resolve_array_length(expr: &ast::Expr) -> Option<u32> {
    let ast::Expr::Literal(ast::Literal::Int(value, _)) = expr else {
        return None;
    };
    u32::try_from(crate::numeric_literal::integer(value)?).ok()
}
