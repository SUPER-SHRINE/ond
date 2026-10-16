//! Stable source type identities used by concrete generic names and link symbols.

use super::types::TypePackage;
use crate::semantic::{TypeId, TypeKind, TypeTable};
use std::fmt::Write;

// Lengths count UTF-8 bytes. Every node has a tag and every sequence a count,
// so delimiters in package paths, names, or nested encodings cannot collide.
fn text(output: &mut String, value: &str) {
    write!(output, "{}:{value}", value.len()).unwrap();
}

fn sequence(output: &mut String, items: &[TypeId], types: &TypeTable, universe: &[TypePackage]) {
    write!(output, "{}:", items.len()).unwrap();
    for item in items {
        encode(output, *item, types, universe);
    }
}

pub(super) fn arguments(items: &[TypeId], types: &TypeTable, universe: &[TypePackage]) -> String {
    let mut output = String::new();
    sequence(&mut output, items, types, universe);
    output
}

pub(super) fn generic_name(
    declaration: &str,
    items: &[TypeId],
    types: &TypeTable,
    universe: &[TypePackage],
) -> String {
    format!(
        "{}:{declaration}[{}]",
        declaration.len(),
        arguments(items, types, universe)
    )
}

fn encode(output: &mut String, id: TypeId, types: &TypeTable, universe: &[TypePackage]) {
    let definition = types.get(id).expect("resolved type must exist");
    if let Some(instance) = types.generic_instance(id) {
        output.push('g');
        text(output, &instance.declaration);
        sequence(output, &instance.arguments, types, universe);
        return;
    }
    // Named recursion stops here, including a declaration still being resolved.
    if let Some(name) = &definition.name
        && matches!(
            definition.kind,
            TypeKind::Pending | TypeKind::Defined { .. } | TypeKind::Interface(_)
        )
    {
        output.push('n');
        text(output, name);
        return;
    }
    match &definition.kind {
        TypeKind::Pointer(pointee) => {
            output.push('p');
            encode(output, *pointee, types, universe);
        }
        TypeKind::Array { length, element } => {
            write!(output, "a{length}:").unwrap();
            encode(output, *element, types, universe);
        }
        TypeKind::Function(signature) => {
            output.push('f');
            sequence(output, &signature.parameters, types, universe);
            sequence(output, &signature.returns, types, universe);
        }
        TypeKind::Struct(structure) => {
            write!(output, "s{}:", structure.fields.len()).unwrap();
            for field in &structure.fields {
                text(output, &field.name);
                match field.private_owner {
                    Some(owner) => {
                        output.push('o');
                        text(
                            output,
                            &universe
                                .iter()
                                .find(|p| p.id == owner)
                                .expect("private field owner must exist")
                                .logical_path,
                        );
                    }
                    None => output.push('e'),
                }
                encode(output, field.ty, types, universe);
            }
        }
        TypeKind::Bool => output.push('b'),
        TypeKind::U8 => output.push('c'),
        TypeKind::I8 => output.push('d'),
        TypeKind::U16 => output.push('h'),
        TypeKind::I16 => output.push('i'),
        TypeKind::U32 => output.push('u'),
        TypeKind::I32 => output.push('j'),
        TypeKind::F32 => output.push('r'),
        TypeKind::Pending | TypeKind::Defined { .. } | TypeKind::Interface(_) => {
            unreachable!("pending, defined and interface types must have a nominal name")
        }
    }
}
