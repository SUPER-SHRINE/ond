//! Source-derived generation inputs, independent of lowered instructions and storage.
use super::types::{GenericSpecialization, TypePackage, method_receiver_name};
use crate::{
    export::identity::{nominal, quote, sequence, signature, type_expr},
    ir::{ast, hir},
    project::PackageId,
    semantic::{FunctionType, TypeId, TypeKind},
    source::Span,
};
use std::collections::{BTreeMap, BTreeSet};

/// Canonical JSON Generation objects keyed by logical package path.
#[derive(Debug, Clone, Default)]
pub struct GenerationInputs {
    pub packages: BTreeMap<String, String>,
}

#[derive(Default)]
pub(super) struct Collector {
    pub(super) enabled: bool,
    pub(super) nodes: Vec<Node>,
    pub(super) values: Vec<(Span, Option<String>, hir::Expr)>,
    pub(super) types: BTreeSet<TypeId>,
}
pub(super) struct Node {
    pub(super) span: Span,
    pub(super) caller: Option<String>,
    pub(super) ty: TypeId,
    pub(super) skipped: bool,
    pub(super) static_value: bool,
    pub(super) value: Option<hir::Expr>,
}

#[derive(Clone)]
enum Declaration {
    Function(String),
    Method {
        receiver: String,
        name: String,
        pointer: bool,
    },
    Operator {
        receiver: String,
        token: &'static str,
        count: usize,
        pointer: bool,
    },
    Global(String),
    Constant(String),
}
#[derive(Clone)]
struct Owner {
    package: PackageId,
    declaration: Declaration,
    receiver_arguments: Vec<TypeId>,
    type_arguments: Vec<TypeId>,
}
struct Function {
    owner: Owner,
    symbol: String,
    signature: FunctionType,
    receiver: Option<TypeId>,
    span: Span,
    generic: bool,
}
struct Value {
    owner: Owner,
    span: Span,
    static_value: bool,
    value: hir::Expr,
}
#[derive(Default)]
pub(crate) struct Collected {
    functions: Vec<Function>,
    types: BTreeMap<PackageId, BTreeSet<TypeId>>,
    values: Vec<Value>,
}

fn contains(outer: Span, inner: Span) -> bool {
    outer.file == inner.file && outer.start <= inner.start && inner.end <= outer.end
}

fn function_declaration(decl: &ast::FuncDecl) -> Declaration {
    if let Some(receiver) = &decl.receiver {
        let (name, pointer) = method_receiver_name(receiver).expect("resolved receiver");
        Declaration::Method {
            receiver: name.into(),
            name: decl.name.name.clone(),
            pointer,
        }
    } else {
        Declaration::Function(decl.name.name.clone())
    }
}
fn operator_declaration(decl: &ast::OperatorDecl) -> Declaration {
    let receiver = &decl.signature.params[0];
    let (name, pointer) = method_receiver_name(receiver).expect("resolved operator receiver");
    let token = match decl.name {
        ast::OperatorName::Add => "+",
        ast::OperatorName::Sub => "-",
        ast::OperatorName::Mul => "*",
        ast::OperatorName::Div => "/",
        ast::OperatorName::Rem => "%",
        ast::OperatorName::Index => "[]",
        ast::OperatorName::IndexSet => "[]=",
        ast::OperatorName::Len => "len",
    };
    Declaration::Operator {
        receiver: name.into(),
        token,
        count: decl.signature.params.len(),
        pointer,
    }
}

pub(super) fn collect(
    project: &hir::Project,
    ast: &ast::Project,
    universe: &[TypePackage],
) -> Collected {
    let mut collected = Collected::default();
    let mut regions = Vec::<(Span, Owner)>::new();
    for package in &project.packages {
        let source = universe
            .iter()
            .find(|source| source.id == package.id)
            .expect("source package");
        let source_ast = ast
            .packages
            .iter()
            .find(|source| source.logical_path == package.logical_path)
            .expect("source AST");
        let roots = collected.types.entry(package.id).or_default();
        roots.extend(source.generation.borrow().types.iter());
        for item in &package.items {
            match item {
                hir::Item::Type(item) => {
                    roots.insert(item.ty);
                }
                hir::Item::Const(item) => roots.extend(item.values.iter().map(|value| value.ty)),
                hir::Item::Var(item) => {
                    roots.extend(item.globals.iter().flatten().map(|global| global.ty))
                }
                hir::Item::Func(function) => {
                    let specialization = source
                        .specializations
                        .borrow()
                        .get(&function.canonical_name)
                        .cloned();
                    let mut owner = Owner {
                        package: package.id,
                        declaration: Declaration::Function(function.name.clone()),
                        receiver_arguments: Vec::new(),
                        type_arguments: Vec::new(),
                    };
                    let generic = specialization.is_some();
                    match specialization {
                        Some(GenericSpecialization::Function { name, arguments }) => {
                            owner.declaration = Declaration::Function(name);
                            owner.type_arguments = arguments;
                        }
                        Some(GenericSpecialization::Method {
                            declaration,
                            arguments,
                        }) => {
                            owner.declaration = function_declaration(&source.methods[declaration]);
                            owner.receiver_arguments = arguments;
                        }
                        Some(GenericSpecialization::Operator {
                            declaration,
                            arguments,
                        }) => {
                            owner.declaration =
                                operator_declaration(&source.operators[declaration]);
                            owner.receiver_arguments = arguments;
                        }
                        None => {
                            for decl in source_ast.files.iter().flat_map(|file| &file.decls) {
                                match decl {
                                    ast::TopLevelDecl::Func(decl) if decl.span == function.span => {
                                        owner.declaration = function_declaration(decl)
                                    }
                                    ast::TopLevelDecl::Operator(decl)
                                        if decl.span == function.span =>
                                    {
                                        owner.declaration = operator_declaration(decl)
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    let receiver = matches!(
                        owner.declaration,
                        Declaration::Method { .. } | Declaration::Operator { .. }
                    )
                    .then(|| {
                        function
                            .signature
                            .parameters
                            .first()
                            .map(|parameter| parameter.ty)
                    })
                    .flatten();
                    let signature = FunctionType {
                        parameters: function
                            .signature
                            .parameters
                            .iter()
                            .skip(usize::from(receiver.is_some()))
                            .map(|parameter| parameter.ty)
                            .collect(),
                        returns: function.signature.returns.clone(),
                    };
                    roots.extend(signature.parameters.iter().chain(&signature.returns));
                    roots.extend(receiver);
                    let hir::Body::Typed(body) = &function.body;
                    roots.extend(
                        body.locals
                            .iter()
                            .filter(|local| !local.name.starts_with('\u{1}'))
                            .map(|local| local.ty),
                    );
                    collected.functions.push(Function {
                        owner,
                        symbol: function.canonical_name.clone(),
                        signature,
                        receiver,
                        span: function.span,
                        generic,
                    });
                }
            }
        }
        for decl in source_ast.files.iter().flat_map(|file| &file.decls) {
            match decl {
                ast::TopLevelDecl::Const(decl) => {
                    for spec in &decl.specs {
                        for (name, value) in spec.names.iter().zip(&spec.values) {
                            regions.push((
                                value.span(),
                                Owner {
                                    package: package.id,
                                    declaration: Declaration::Constant(name.name.clone()),
                                    receiver_arguments: Vec::new(),
                                    type_arguments: Vec::new(),
                                },
                            ));
                        }
                    }
                }
                ast::TopLevelDecl::Var(decl) => {
                    for spec in &decl.specs {
                        for (name, value) in spec.names.iter().zip(&spec.values) {
                            regions.push((
                                value.span(),
                                Owner {
                                    package: package.id,
                                    declaration: Declaration::Global(name.name.clone()),
                                    receiver_arguments: Vec::new(),
                                    type_arguments: Vec::new(),
                                },
                            ));
                        }
                    }
                }
                _ => {}
            }
        }
    }
    for source in universe {
        let capture = source.generation.borrow();
        for node in &capture.nodes {
            collected
                .types
                .entry(source.id)
                .or_default()
                .insert(node.ty);
            if node.skipped {
                continue;
            }
            let owner = collected
                .functions
                .iter()
                .find(|function| {
                    contains(function.span, node.span)
                        && (node.caller.as_deref() == Some(function.symbol.as_str())
                            || node.caller.is_none() && !function.generic)
                })
                .map(|function| function.owner.clone())
                .or_else(|| {
                    regions
                        .iter()
                        .find(|(span, _)| contains(*span, node.span))
                        .map(|(_, owner)| owner.clone())
                });
            let value = capture
                .values
                .iter()
                .rev()
                .find(|(span, caller, value)| {
                    *span == node.span && *caller == node.caller && value.ty == node.ty
                })
                .map(|(_, _, value)| value.clone())
                .or_else(|| node.value.clone());
            if let (Some(owner), Some(value)) = (owner, value) {
                collected.values.push(Value {
                    owner,
                    value,
                    span: node.span,
                    static_value: node.static_value,
                });
            }
        }
    }
    collected
}

impl Owner {
    fn json(&self, project: &hir::Project) -> Result<String, String> {
        let package = quote(
            &project
                .packages
                .iter()
                .find(|package| package.id == self.package)
                .ok_or("missing owner package")?
                .logical_path,
        );
        let declaration = match &self.declaration {
            Declaration::Function(name)
            | Declaration::Global(name)
            | Declaration::Constant(name) => {
                let tag = match self.declaration {
                    Declaration::Function(_) => "Function",
                    Declaration::Global(_) => "Global",
                    _ => "Constant",
                };
                format!(
                    "{{{}:{{\"name\":{},\"package\":{package}}}}}",
                    quote(tag),
                    quote(name)
                )
            }
            Declaration::Method {
                receiver,
                name,
                pointer,
            } => format!(
                "{{\"Method\":{{\"name\":{},\"package\":{package},\"pointer_receiver\":{pointer},\"receiver\":{}}}}}",
                quote(name),
                quote(receiver)
            ),
            Declaration::Operator {
                receiver,
                token,
                count,
                pointer,
            } => format!(
                "{{\"Operator\":{{\"operand_count\":{count},\"package\":{package},\"pointer_receiver\":{pointer},\"receiver\":{},\"token\":{}}}}}",
                quote(receiver),
                quote(token)
            ),
        };
        Ok(format!(
            "{{\"declaration\":{declaration},\"receiver_arguments\":{},\"type_arguments\":{}}}",
            sequence(project, &self.receiver_arguments)?,
            sequence(project, &self.type_arguments)?
        ))
    }
}

fn normalized_value(project: &hir::Project, value: &hir::Expr) -> Result<String, String> {
    let kind = match &value.kind {
        hir::ExprKind::Zero
        | hir::ExprKind::Nil
        | hir::ExprKind::Integer(0)
        | hir::ExprKind::Float32(0)
        | hir::ExprKind::Bool(false) => quote("Zero"),
        hir::ExprKind::Integer(bits) => format!("{{\"Integer\":{bits}}}"),
        hir::ExprKind::Float32(bits) => format!("{{\"Float32\":{bits}}}"),
        hir::ExprKind::Bool(value) => format!("{{\"Bool\":{value}}}"),
        hir::ExprKind::Composite(elements) => {
            let mut children = Vec::new();
            for (index, child) in elements {
                let value = normalized_value(project, child)?;
                if !value.starts_with("{\"kind\":\"Zero\",") {
                    children.push((*index, value));
                }
            }
            children.sort_by_key(|(index, _)| *index);
            if children.is_empty() {
                quote("Zero")
            } else {
                format!(
                    "{{\"Composite\":[{}]}}",
                    children
                        .iter()
                        .map(|(index, value)| format!("[{index},{value}]"))
                        .collect::<Vec<_>>()
                        .join(",")
                )
            }
        }
        _ => return Err("generation value was not evaluated at a source boundary".into()),
    };
    Ok(format!(
        "{{\"kind\":{kind},\"ty\":{}}}",
        type_expr(project, value.ty)?
    ))
}

fn json_set(values: impl IntoIterator<Item = String>) -> String {
    format!(
        "[{}]",
        values
            .into_iter()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
            .join(",")
    )
}

impl Collected {
    pub(crate) fn normalize(self, project: &hir::Project) -> Result<GenerationInputs, String> {
        let mut packages = BTreeMap::new();
        for package in &project.packages {
            let functions = self
                .functions
                .iter()
                .filter(|function| function.owner.package == package.id)
                .collect::<Vec<_>>();
            let mut types = BTreeSet::<String>::new();
            let mut visited = BTreeSet::new();
            let mut pending = self
                .types
                .get(&package.id)
                .into_iter()
                .flatten()
                .copied()
                .collect::<Vec<_>>();
            for function in &functions {
                types.insert(format!(
                    "{{\"Function\":{}}}",
                    signature(
                        project,
                        &function.signature.parameters,
                        &function.signature.returns
                    )?
                ));
            }
            let (interface, ids) =
                crate::export::package_and_type_ids(project, &package.logical_path)?;
            pending.extend(&ids);
            let mut nominals = BTreeMap::new();
            while let Some(id) = pending.pop() {
                if !visited.insert(id) {
                    continue;
                }
                types.insert(type_expr(project, id)?);
                match project.types.kind(id).ok_or("missing generation type")? {
                    TypeKind::Pointer(inner) => pending.push(*inner),
                    TypeKind::Array { element, .. } => pending.push(*element),
                    TypeKind::Function(function) => {
                        pending.extend(function.parameters.iter().chain(&function.returns))
                    }
                    TypeKind::Struct(structure) => {
                        pending.extend(structure.fields.iter().map(|field| field.ty))
                    }
                    TypeKind::Defined { underlying } => {
                        pending.push(*underlying);
                        if let Some(instance) = project.types.generic_instance(id) {
                            pending.extend(&instance.arguments);
                        }
                        let identity = nominal(project, id)?.1;
                        let mut methods = Vec::new();
                        for function in &self.functions {
                            let Some(receiver) = function.receiver else {
                                continue;
                            };
                            let base = match project.types.kind(receiver) {
                                Some(TypeKind::Pointer(base)) => *base,
                                _ => receiver,
                            };
                            let Declaration::Method { name, pointer, .. } =
                                &function.owner.declaration
                            else {
                                continue;
                            };
                            if base != id
                                || !name.as_bytes().first().is_some_and(u8::is_ascii_uppercase)
                            {
                                continue;
                            }
                            pending.extend(
                                function
                                    .signature
                                    .parameters
                                    .iter()
                                    .chain(&function.signature.returns),
                            );
                            pending.extend(
                                function
                                    .owner
                                    .receiver_arguments
                                    .iter()
                                    .chain(&function.owner.type_arguments),
                            );
                            methods.push((name.clone(),function.owner.json(project)?,format!("{{\"identity\":{},\"name\":{},\"pointer_receiver\":{pointer},\"signature\":{}}}", function.owner.json(project)?,quote(name),signature(project,&function.signature.parameters,&function.signature.returns)?)));
                        }
                        methods.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
                        nominals.insert(
                            identity.clone(),
                            format!(
                                "{{\"identity\":{identity},\"methods\":[{}],\"underlying\":{}}}",
                                methods
                                    .into_iter()
                                    .map(|(_, _, value)| value)
                                    .collect::<Vec<_>>()
                                    .join(","),
                                type_expr(project, *underlying)?
                            ),
                        );
                    }
                    _ => {}
                }
            }
            let candidates = self
                .values
                .iter()
                .filter(|value| value.owner.package == package.id)
                .map(|value| Ok((value, value.owner.json(project)?)))
                .collect::<Result<Vec<_>, String>>()?;
            let mut constants = Vec::new();
            for (value, owner) in &candidates {
                let covered = candidates.iter().any(|(outer, outer_owner)| {
                    owner == outer_owner
                        && outer.span != value.span
                        && contains(outer.span, value.span)
                });
                if !covered || value.static_value {
                    constants.push(format!(
                        "{{\"owner\":{owner},\"value\":{}}}",
                        normalized_value(project, &value.value)?
                    ));
                }
            }
            let instances = functions
                .iter()
                .filter(|function| function.generic)
                .map(|function| {
                    Ok(format!(
                        "{{\"identity\":{},\"signature\":{}}}",
                        function.owner.json(project)?,
                        signature(
                            project,
                            &function.signature.parameters,
                            &function.signature.returns
                        )?
                    ))
                })
                .collect::<Result<Vec<_>, String>>()?;
            let mut exports = Vec::new();
            for (name, export) in &interface.exports {
                use ond_interface::Export;
                let value = match export {
                    Export::Type(id) => {
                        format!("{{\"Type\":{}}}", type_expr(project, ids[*id as usize])?)
                    }
                    Export::Global(id) => {
                        format!("{{\"Global\":{}}}", type_expr(project, ids[*id as usize])?)
                    }
                    Export::Function(function) => format!(
                        "{{\"Function\":{}}}",
                        signature(
                            project,
                            &function
                                .parameters
                                .iter()
                                .map(|id| ids[*id as usize])
                                .collect::<Vec<_>>(),
                            &function
                                .returns
                                .iter()
                                .map(|id| ids[*id as usize])
                                .collect::<Vec<_>>()
                        )?
                    ),
                    Export::Constant(value) => {
                        format!("{{\"Constant\":{}}}", wire_value(project, value, &ids)?)
                    }
                };
                exports.push(format!("{}:{value}", quote(name)));
            }
            packages.insert(package.logical_path.clone(),format!("{{\"constants\":{},\"instances\":{},\"interface\":{{\"exports\":{{{}}},\"package\":{}}},\"nominals\":[{}],\"types\":{}}}",json_set(constants),json_set(instances),exports.join(","),quote(&package.logical_path),nominals.into_values().collect::<Vec<_>>().join(","),json_set(types)));
        }
        Ok(GenerationInputs { packages })
    }
}

fn wire_value(
    project: &hir::Project,
    value: &ond_interface::Value,
    ids: &[TypeId],
) -> Result<String, String> {
    use ond_interface::ValueKind;
    let kind = match &value.kind {
        ValueKind::Zero | ValueKind::Nil => hir::ExprKind::Zero,
        ValueKind::Integer(bits) => hir::ExprKind::Integer(*bits),
        ValueKind::Float32(bits) => hir::ExprKind::Float32(*bits),
        ValueKind::Bool(value) => hir::ExprKind::Bool(*value),
        ValueKind::Composite(elements) => {
            let children = elements
                .iter()
                .map(|(index, child)| Ok(format!("[{index},{}]", wire_value(project, child, ids)?)))
                .collect::<Result<Vec<_>, String>>()?;
            let kind = if children.is_empty() {
                quote("Zero")
            } else {
                format!("{{\"Composite\":[{}]}}", children.join(","))
            };
            return Ok(format!(
                "{{\"kind\":{kind},\"ty\":{}}}",
                type_expr(project, ids[value.ty as usize])?
            ));
        }
    };
    normalized_value(
        project,
        &hir::Expr {
            kind,
            ty: ids[value.ty as usize],
            span: Span::synthetic(),
        },
    )
}
