//! Canonical TypeExpr spelling for externally visible nominal identities.
use crate::{
    ir::hir,
    semantic::{TypeId, TypeKind},
};

pub(crate) fn quote(value: &str) -> String {
    let mut result = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\u{8}' => result.push_str("\\b"),
            '\t' => result.push_str("\\t"),
            '\n' => result.push_str("\\n"),
            '\u{c}' => result.push_str("\\f"),
            '\r' => result.push_str("\\r"),
            ch if ch < '\u{20}' => {
                use std::fmt::Write;
                write!(&mut result, "\\u{:04x}", ch as u32).unwrap();
            }
            ch => result.push(ch),
        }
    }
    result.push('"');
    result
}

pub(crate) fn sequence(project: &hir::Project, types: &[TypeId]) -> Result<String, String> {
    Ok(format!(
        "[{}]",
        types
            .iter()
            .map(|ty| type_expr(project, *ty))
            .collect::<Result<Vec<_>, _>>()?
            .join(",")
    ))
}

pub(crate) fn nominal(project: &hir::Project, id: TypeId) -> Result<(String, String), String> {
    let definition = project.types.get(id).ok_or("missing nominal type")?;
    let (declaration, arguments) = if let Some(instance) = project.types.generic_instance(id) {
        (
            instance.declaration.as_str(),
            sequence(project, &instance.arguments)?,
        )
    } else {
        (
            definition
                .name
                .as_deref()
                .ok_or("nominal type has no name")?,
            "[]".into(),
        )
    };
    let (package, name) = declaration
        .rsplit_once('.')
        .ok_or("nominal type has no package")?;
    Ok((
        declaration.into(),
        format!(
            "{{\"arguments\":{arguments},\"name\":{},\"package\":{}}}",
            quote(name),
            quote(package)
        ),
    ))
}

pub(super) fn nominal_name(project: &hir::Project, id: TypeId) -> Result<String, String> {
    let (declaration, _) = nominal(project, id)?;
    if let Some(instance) = project.types.generic_instance(id) {
        Ok(format!(
            "{declaration}{}",
            sequence(project, &instance.arguments)?
        ))
    } else {
        Ok(declaration)
    }
}

pub(crate) fn signature(
    project: &hir::Project,
    parameters: &[TypeId],
    returns: &[TypeId],
) -> Result<String, String> {
    Ok(format!(
        "{{\"parameters\":{},\"returns\":{}}}",
        sequence(project, parameters)?,
        sequence(project, returns)?
    ))
}

pub(crate) fn type_expr(project: &hir::Project, id: TypeId) -> Result<String, String> {
    let kind = project.types.kind(id).ok_or("missing type")?;
    Ok(match kind {
        TypeKind::Bool => quote("Bool"),
        TypeKind::U8 => quote("U8"),
        TypeKind::I8 => quote("I8"),
        TypeKind::U16 => quote("U16"),
        TypeKind::I16 => quote("I16"),
        TypeKind::U32 => quote("U32"),
        TypeKind::I32 => quote("I32"),
        TypeKind::F32 => quote("F32"),
        TypeKind::Defined { .. } => format!("{{\"Defined\":{}}}", nominal(project, id)?.1),
        TypeKind::Pointer(inner) => format!("{{\"Pointer\":{}}}", type_expr(project, *inner)?),
        TypeKind::Array { length, element } => format!(
            "{{\"Array\":{{\"element\":{},\"length\":{length}}}}}",
            type_expr(project, *element)?
        ),
        TypeKind::Function(function) => format!(
            "{{\"Function\":{}}}",
            signature(project, &function.parameters, &function.returns)?
        ),
        TypeKind::Struct(structure) => {
            let fields = structure
                .fields
                .iter()
                .map(|field| {
                    let owner = field
                        .private_owner
                        .map(|owner| {
                            project
                                .packages
                                .iter()
                                .find(|package| package.id == owner)
                                .map(|package| quote(&package.logical_path))
                                .ok_or("missing field owner")
                        })
                        .transpose()?
                        .unwrap_or_else(|| "null".into());
                    Ok(format!(
                        "{{\"name\":{},\"private_owner\":{owner},\"ty\":{}}}",
                        quote(&field.name),
                        type_expr(project, field.ty)?
                    ))
                })
                .collect::<Result<Vec<_>, String>>()?;
            format!("{{\"Struct\":[{}]}}", fields.join(","))
        }
        TypeKind::Pending => return Err("cannot export pending type identity".into()),
    })
}
