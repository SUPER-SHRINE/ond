use ond_compiler_core::{
    mir,
    semantic::{TypeId, TypeKind, TypeTable},
};
use std::{collections::BTreeMap, fmt::Write};

pub fn package_inputs(project: &mir::Project, package: &mir::Package) -> String {
    let mut output = String::new();
    let mut roots = BTreeMap::<String, Vec<TypeId>>::new();
    let mut values = BTreeMap::new();
    for function in package.functions.iter().chain(package.initializer.iter()) {
        collect_function_types(function, &mut roots);
        let constants = function
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .filter_map(|instruction| match &instruction.kind {
                mir::InstructionKind::Constant(value) => Some(value),
                _ => None,
            })
            .collect::<Vec<_>>();
        let mut encoded = format!("{}:", constants.len());
        let mut references = BTreeMap::new();
        for constant in constants {
            encode_constant(&mut encoded, constant, project, &mut references);
        }
        values.insert(function.name.clone(), encoded);
    }
    for global in &package.globals {
        let types = roots.entry(global.symbol.clone()).or_default();
        types.push(global.ty);
        if let Some(initializer) = &global.initializer {
            collect_static_types(initializer, types);
            values.insert(global.symbol.clone(), static_value(initializer, project));
        }
    }
    for static_data in &package.statics {
        let types = roots.entry(static_data.symbol.clone()).or_default();
        collect_static_types(&static_data.value, types);
        values.insert(
            static_data.symbol.clone(),
            static_value(&static_data.value, project),
        );
    }
    for (symbol, types) in roots {
        text(&mut output, &symbol);
        text(
            &mut output,
            values.get(&symbol).map(String::as_str).unwrap_or(""),
        );
        let mut references = BTreeMap::new();
        for ty in types {
            encode_type(
                &mut output,
                ty,
                &project.types,
                &project.packages,
                &mut references,
            );
        }
    }
    output
}

fn encode_constant(
    output: &mut String,
    value: &mir::Constant,
    project: &mir::Project,
    references: &mut BTreeMap<TypeId, usize>,
) {
    match value {
        mir::Constant::Zero(_) => output.push('z'),
        mir::Constant::Integer { bits, .. } => write!(output, "i{bits};").unwrap(),
        mir::Constant::Float32 { bits, .. } => write!(output, "f{bits};").unwrap(),
        mir::Constant::Bool { value, .. } => output.push(if *value { 't' } else { 'b' }),
        mir::Constant::Nil(_) => output.push('n'),
        mir::Constant::Function {
            symbol, signature, ..
        } => {
            output.push('F');
            text(output, symbol);
            sequence(
                output,
                &signature.parameters,
                &project.types,
                &project.packages,
                references,
            );
            sequence(
                output,
                &signature.returns,
                &project.types,
                &project.packages,
                references,
            );
        }
    }
    encode_type(
        output,
        value.ty(),
        &project.types,
        &project.packages,
        references,
    );
}

fn static_value(value: &mir::StaticValue, project: &mir::Project) -> String {
    let mut output = String::new();
    encode_static_value(&mut output, value, project, &mut BTreeMap::new());
    output
}

fn encode_static_value(
    output: &mut String,
    value: &mir::StaticValue,
    project: &mir::Project,
    references: &mut BTreeMap<TypeId, usize>,
) {
    match &value.kind {
        mir::StaticValueKind::Zero => output.push('z'),
        mir::StaticValueKind::Integer(bits) => write!(output, "i{bits};").unwrap(),
        mir::StaticValueKind::Float32(bits) => write!(output, "f{bits};").unwrap(),
        mir::StaticValueKind::Bool(value) => output.push(if *value { 't' } else { 'b' }),
        mir::StaticValueKind::Nil => output.push('n'),
        mir::StaticValueKind::Function { symbol, signature } => {
            output.push('F');
            text(output, symbol);
            sequence(
                output,
                &signature.parameters,
                &project.types,
                &project.packages,
                references,
            );
            sequence(
                output,
                &signature.returns,
                &project.types,
                &project.packages,
                references,
            );
        }
        mir::StaticValueKind::Composite(elements) => {
            write!(output, "c{}:", elements.len()).unwrap();
            for (index, value) in elements {
                write!(output, "{index}:").unwrap();
                encode_static_value(output, value, project, references);
            }
        }
    }
    encode_type(
        output,
        value.ty,
        &project.types,
        &project.packages,
        references,
    );
}

fn collect_function_types(function: &mir::Function, roots: &mut BTreeMap<String, Vec<TypeId>>) {
    let types = roots.entry(function.name.clone()).or_default();
    types.extend(function.returns.iter().copied());
    types.extend(function.locals.iter().map(|local| local.ty));
    types.extend(function.values.iter().map(|value| value.ty));
}

fn collect_static_types(value: &mir::StaticValue, types: &mut Vec<TypeId>) {
    types.push(value.ty);
    match &value.kind {
        mir::StaticValueKind::Function { signature, .. } => {
            types.extend(signature.parameters.iter().copied());
            types.extend(signature.returns.iter().copied());
        }
        mir::StaticValueKind::Composite(elements) => {
            for (_, value) in elements {
                collect_static_types(value, types);
            }
        }
        mir::StaticValueKind::Zero
        | mir::StaticValueKind::Integer(_)
        | mir::StaticValueKind::Float32(_)
        | mir::StaticValueKind::Bool(_)
        | mir::StaticValueKind::Nil => {}
    }
}

fn text(output: &mut String, value: &str) {
    write!(output, "{}:{value}", value.len()).unwrap();
}

fn sequence(
    output: &mut String,
    types: &[TypeId],
    table: &TypeTable,
    packages: &[mir::Package],
    references: &mut BTreeMap<TypeId, usize>,
) {
    write!(output, "{}:", types.len()).unwrap();
    for ty in types {
        encode_type(output, *ty, table, packages, references);
    }
}

fn encode_type(
    output: &mut String,
    id: TypeId,
    table: &TypeTable,
    packages: &[mir::Package],
    references: &mut BTreeMap<TypeId, usize>,
) {
    if let Some(index) = references.get(&id) {
        write!(output, "r{index};").unwrap();
        return;
    }
    let index = references.len();
    references.insert(id, index);
    let definition = table.get(id).expect("MIR type must exist");
    if let Some(instance) = table.generic_instance(id) {
        output.push('g');
        text(output, &instance.declaration);
        sequence(output, &instance.arguments, table, packages, references);
        encode_kind(output, &definition.kind, table, packages, references);
    } else if let Some(name) = &definition.name {
        output.push('n');
        text(output, name);
        encode_kind(output, &definition.kind, table, packages, references);
    } else {
        encode_kind(output, &definition.kind, table, packages, references);
    }
    output.push(';');
}

fn encode_kind(
    output: &mut String,
    kind: &TypeKind,
    table: &TypeTable,
    packages: &[mir::Package],
    references: &mut BTreeMap<TypeId, usize>,
) {
    match kind {
        TypeKind::Pending => output.push('?'),
        TypeKind::Bool => output.push('b'),
        TypeKind::U8 => output.push('c'),
        TypeKind::I8 => output.push('d'),
        TypeKind::U16 => output.push('h'),
        TypeKind::I16 => output.push('i'),
        TypeKind::U32 => output.push('u'),
        TypeKind::I32 => output.push('j'),
        TypeKind::F32 => output.push('f'),
        TypeKind::Pointer(inner) => {
            output.push('p');
            encode_type(output, *inner, table, packages, references);
        }
        TypeKind::Array { length, element } => {
            write!(output, "a{length}:").unwrap();
            encode_type(output, *element, table, packages, references);
        }
        TypeKind::Function(signature) => {
            output.push('F');
            sequence(output, &signature.parameters, table, packages, references);
            sequence(output, &signature.returns, table, packages, references);
        }
        TypeKind::Struct(structure) => {
            encode_struct(output, structure, table, packages, references)
        }
        TypeKind::Defined { underlying } => {
            output.push('D');
            encode_type(output, *underlying, table, packages, references);
        }
    }
}

fn encode_struct(
    output: &mut String,
    structure: &ond_compiler_core::semantic::StructType,
    table: &TypeTable,
    packages: &[mir::Package],
    references: &mut BTreeMap<TypeId, usize>,
) {
    output.push('s');
    write!(output, "{}:", structure.fields.len()).unwrap();
    for field in &structure.fields {
        text(output, &field.name);
        owner(output, field.private_owner, packages);
        encode_type(output, field.ty, table, packages, references);
    }
}

fn owner(output: &mut String, id: Option<ond_compiler_core::PackageId>, packages: &[mir::Package]) {
    match id.and_then(|id| packages.iter().find(|package| package.id == id)) {
        Some(package) => text(output, &package.logical_path),
        None => output.push('-'),
    }
}
