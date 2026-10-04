use super::*;
#[test]
fn no_requests_emit_nothing_and_all_registered_helpers_compile() {
    assert!(objects(&[]).unwrap().is_empty());
    let output = objects(&Helper::ALL).unwrap();
    let names = output
        .iter()
        .flat_map(|o| &o.symbols)
        .map(|s| s.name.as_str())
        .collect::<BTreeSet<_>>();
    for h in Helper::ALL {
        assert!(names.contains(h.symbol().as_str()));
    }
    for r in output.iter().flat_map(|o| &o.relocations) {
        assert!(
            names.contains(r.target.as_str()) || r.target == "__ond_stack_bottom",
            "{}",
            r.target
        );
    }
    assert!(
        output
            .iter()
            .all(|o| !o.symbols.iter().any(|s| s.name == "main.main"))
    );
}
#[test]
fn selection_keeps_transitive_dependencies_and_deduplicates() {
    let one = objects(&[Helper::F32Neg]).unwrap();
    let two = objects(&[Helper::F32Neg, Helper::F32Neg]).unwrap();
    assert_eq!(one[0].sections[0].data, two[0].sections[0].data);
    let names = one[0]
        .symbols
        .iter()
        .map(|s| s.name.as_str())
        .collect::<Vec<_>>();
    assert!(names.iter().any(|n| n.ends_with("__sf_is_nan")));
    assert!(!names.iter().any(|n| n.ends_with("__ond_f32_add")));
}
#[test]
fn unknown_roots_and_runtime_global_initialization_are_diagnosed() {
    let mut p = library::load("package runtime\nfunc entry(){}\n").unwrap();
    assert!(
        library::select(&mut p, &["runtime.missing".into()])
            .unwrap_err()
            .to_string()
            .contains("missing dependency")
    );
    assert!(
        library::load("package runtime\nvar x=1\nfunc entry(){}\n")
            .unwrap_err()
            .to_string()
            .contains("global storage")
    );
}

#[test]
fn address_dependencies_and_recursive_calls_have_a_finite_closure() {
    let mut p=library::load("package runtime\nfunc entry(x:u32)->u32{var f:func(u32)->u32=leaf;return f(x)}\nfunc leaf(x:u32)->u32{if x==0{return 1};return leaf(x-1)}\nfunc unused(x:f32)->f32{return x}\n").unwrap();
    library::select(&mut p, &["runtime.entry".into()]).unwrap();
    assert_eq!(p.packages[0].functions.len(), 2);
    let objects = codegen_raw(&p).unwrap().objects;
    assert!(
        objects[0]
            .relocations
            .iter()
            .any(|r| r.kind == RelocationKind::Abs32 && r.target.ends_with("leaf"))
    );
}

#[test]
fn malformed_requests_and_reserved_definitions_are_rejected() {
    let mut p = library::load("package runtime\nfunc entry(){}\n").unwrap();
    let mut input = codegen_raw(&p).unwrap().objects;
    input[0].relocations.push(Relocation {
        section: SectionKind::Text,
        offset: 0,
        kind: RelocationKind::Call16,
        target: format!("{PREFIX}unknown"),
        addend: 0,
    });
    assert!(
        supply(input)
            .unwrap_err()
            .to_string()
            .contains("unknown request")
    );
    library::select(&mut p, &["runtime.entry".into()]).unwrap();
    assert!(
        supply(codegen_raw(&p).unwrap().objects)
            .unwrap_err()
            .to_string()
            .contains("reserved runtime symbol")
    );
}
