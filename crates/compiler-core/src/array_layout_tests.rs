//! Fixed arrays and structs must fit the 32-bit byte-size limit before MIR lowering.
use crate::test_support::compile;

#[test]
fn oversized_array_type_is_rejected_even_when_unused() {
    crate::test_support::reject(
        "package main\ntype TooLarge [1073741824]u32\nfunc main() {}\n",
        None,
        "array layout overflow",
        "[1073741824]u32",
    );
}

#[test]
fn array_layout_limit_accepts_the_boundary_and_zero_sizes() {
    for (ty, size) in [
        ("[4294967295]u8", u32::MAX),
        ("[2147483647]u16", u32::MAX - 1),
        ("[1073741823]u32", u32::MAX - 3),
        ("[1073741823]*u8", u32::MAX - 3),
        ("[1073741823]func()", u32::MAX - 3),
        ("[536870911]Element", u32::MAX - 7),
        ("[0]u32", 0),
        ("[4294967295]struct {}", 0),
        ("[4294967295][0]u32", 0),
    ] {
        let source = format!(
            "package main\ntype Element struct {{ First:u32; Second:u32 }}\ntype Accepted {ty}\nfunc main() {{}}\n"
        );
        let result = compile(&source, None).unwrap_or_else(|error| panic!("{source}: {error}"));
        let accepted = result.hir.types.find_named("main.Accepted").unwrap();
        assert_eq!(result.hir.types.size_align(accepted).unwrap().0, size);
    }
}

#[test]
fn array_layout_limit_applies_to_all_resolved_type_positions() {
    for declaration in [
        "type Unused [2147483648]u16",
        "type Unused [4294967295]u32",
        "type Unused *[1073741824]u32",
        "type Unused struct { items: [1073741824]u32 }",
        "type Unused func(items: [1073741824]u32)",
        "var unused: [1073741824]u32",
        "func unused(items: [1073741824]u32) {}",
        "func unused() -> [1073741824]u32 { return [1073741824]u32{} }",
        "func unused() { var items: [1073741824]u32 }",
        "const Unused = [1073741824]u32{}",
        "const Unused = len([1073741824]u32{})",
        "const Unused = sizeof([1073741824]u32)",
        "const Unused = alignof([1073741824]u32)",
        "func unused() { var items = new([1073741824]u32) }",
        "func unused() { var items = alloc[[1073741824]u32](1 as u32) }",
        "type Word u32\ntype Unused [1073741824]Word",
        "type Alias [1073741824]u32\ntype Unused Alias",
        "type Unused [2][2147483648]u8",
        "type Unused [0][1073741824]u32",
        "type Element struct { First:u32; Second:u32 }\ntype Unused [536870912]Element",
        "type Unused [1073741824]*u8",
        "type Unused [1073741824]func()",
    ] {
        let source = format!("package main\n{declaration}\nfunc main() {{}}\n");
        let error = compile(&source, None).expect_err(&source);
        assert!(
            error.to_string().contains("array layout overflow"),
            "{source}: {error}"
        );
        assert!(!error.diagnostics()[0].primary.span.is_synthetic());
    }
}

#[test]
fn array_element_size_includes_struct_padding() {
    let element = "type Padded struct { a: u8; b: u32; c: u8 }";
    // The fields occupy six bytes, but the element stride is twelve bytes.
    let accepted =
        format!("package main\n{element}\ntype Accepted [357913941]Padded\nfunc main() {{}}\n");
    let result = compile(&accepted, None).unwrap();
    let ty = result.hir.types.find_named("main.Accepted").unwrap();
    assert_eq!(result.hir.types.size_align(ty), Ok((u32::MAX - 3, 4)));
    let rejected = accepted.replace("357913941", "357913942");
    crate::test_support::reject(
        &rejected,
        None,
        "array layout overflow",
        "[357913942]Padded",
    );
}

#[test]
fn inferred_arrays_obey_the_same_layout_limit() {
    for expression in [
        "[...]u16{2147483647 as u32: 0}",
        "[...]u32{1073741823 as u32: 0}",
        "[...][2]u8{2147483647 as u32: [2]u8{}}",
    ] {
        let source = format!("package main\nconst Unused = {expression}\nfunc main() {{}}\n");
        crate::test_support::reject(&source, None, "array layout overflow", expression);
    }
}

#[test]
fn concrete_generic_arrays_obey_the_layout_limit() {
    for declaration in [
        "type Buffer[T] [1073741824]T\nvar unused: *Buffer[u32]",
        "func unused[T]() { var items: [1073741824]T }\nfunc use() { unused[u32]() }",
        "type Buffer[T] struct { items: [1073741824]T }\nvar unused: *Buffer[u32]",
    ] {
        let source = format!("package main\n{declaration}\nfunc main() {{}}\n");
        let error = compile(&source, None).expect_err(&source);
        assert!(
            error.to_string().contains("array layout overflow"),
            "{source}: {error}"
        );
        assert!(!error.diagnostics()[0].primary.span.is_synthetic());
    }
    compile(
        "package main\ntype Buffer[T] [4294967295]T\nvar unused: *Buffer[u8]\nfunc main() {}\n",
        None,
    )
    .unwrap();
}

#[test]
fn recursive_pointer_elements_are_checked_after_resolution() {
    compile(
        "package main\ntype Node struct { children: *[1073741823]Node }\nfunc main() {}\n",
        None,
    )
    .unwrap();
    crate::test_support::reject(
        "package main\ntype Node struct { children: *[1073741824]Node }\nfunc main() {}\n",
        None,
        "array layout overflow",
        "[1073741824]Node",
    );
    crate::test_support::reject(
        "package main\ntype Cycle [1]Cycle\nfunc main() {}\n",
        None,
        "direct recursive type",
        "Cycle [1]Cycle",
    );
}

#[test]
fn struct_layout_limit_accepts_u32_max_and_zero_length_wrappers() {
    let source = "package main
type Largest struct { first: [4294967294]u8; last: u8 }
type Empty [0]Largest
type NestedEmpty [1][0]Largest
type RepeatedEmpty [4294967295][0]Largest
func main() {}
";
    let result = compile(source, None).unwrap();
    for (name, size) in [
        ("main.Largest", u32::MAX),
        ("main.Empty", 0),
        ("main.NestedEmpty", 0),
        ("main.RepeatedEmpty", 0),
    ] {
        let ty = result.hir.types.find_named(name).unwrap();
        assert_eq!(result.hir.types.size_align(ty), Ok((size, 1)));
    }
}

#[test]
fn struct_layout_limit_checks_field_sizes_and_padding_without_values() {
    for (type_prefix, structure) in [
        ("", "struct { first: [4294967295]u8; last: u8 }"),
        // A zero-length field adds no bytes but still aligns its start.
        ("", "struct { first: [4294967295]u8; last: [0]u16 }"),
        // The last field ends at u32::MAX before final alignment.
        (
            "",
            "struct { first: [4294967292]u8; second: u16; last: u8 }",
        ),
        ("*", "struct { first: [4294967295]u8; last: u8 }"),
        ("[0]", "struct { first: [4294967295]u8; last: u8 }"),
    ] {
        let source =
            format!("package main\ntype Unused {type_prefix}{structure}\nfunc main() {{}}\n");
        crate::test_support::reject(&source, None, "struct layout overflow", structure);
    }
}

#[test]
fn struct_layout_limit_checks_concrete_generic_structs() {
    let structure = "struct { first: [4294967295]u8; last: T }";
    for declaration in ["", "type Pointer *Buffer[struct {}]"] {
        let source =
            format!("package main\ntype Buffer[T] {structure}\n{declaration}\nfunc main() {{}}\n");
        compile(&source, None).unwrap();
    }
    let source = format!(
        "package main\ntype Buffer[T] {structure}\ntype Pointer *Buffer[u8]\nfunc main() {{}}\n"
    );
    let error = compile(&source, None).expect_err(&source);
    let [diagnostic] = error.diagnostics() else {
        panic!("one concrete struct layout error expected: {error}");
    };
    assert_eq!(diagnostic.message, "struct layout overflow");
    let primary = diagnostic.primary.span;
    assert_eq!(&source[primary.start..primary.end], "Buffer[u8]");
    let [related] = diagnostic.related.as_slice() else {
        panic!("one generic definition position expected: {error}");
    };
    assert_eq!(&source[related.span.start..related.span.end], structure);
}

#[test]
fn struct_layout_diagnostics_keep_sites_after_type_interning() {
    let plain = "struct { first: [4294967295]u8; last: u8 }";
    let generic = "struct { first: [4294967295]u8; last: T }";
    let source = format!(
        "package main\ntype Plain {plain}\ntype Buffer[T] [0]{generic}\n\
         type First *Buffer[u8]\ntype Second Buffer[u8]\nfunc main() {{}}\n"
    );
    let error = compile(&source, None).expect_err(&source);
    let mut expected = vec![source.find(plain).unwrap()];
    expected.extend(source.match_indices("Buffer[u8]").map(|(offset, _)| offset));
    let mut primary_offsets = Vec::new();
    for diagnostic in error.diagnostics() {
        assert_eq!(diagnostic.message, "struct layout overflow");
        let primary = diagnostic.primary.span;
        primary_offsets.push(primary.start);
        if &source[primary.start..primary.end] == "Buffer[u8]" {
            let [related] = diagnostic.related.as_slice() else {
                panic!("one generic definition position expected: {error}");
            };
            assert_eq!(&source[related.span.start..related.span.end], generic);
        } else {
            assert_eq!(&source[primary.start..primary.end], plain);
            assert!(diagnostic.related.is_empty());
        }
    }
    primary_offsets.sort_unstable();
    assert_eq!(primary_offsets, expected);
}

#[test]
fn struct_layout_diagnostics_follow_nested_generic_type_requests() {
    let structure = "struct { first: [4294967295]u8; last: T }";
    let source = format!(
        "package main\ntype Inner[T] {structure}\ntype Outer[T] [0]Inner[T]\n\
         type First *Outer[u8]\ntype Second Outer[u8]\nfunc main() {{}}\n"
    );
    let error = compile(&source, None).expect_err(&source);
    let mut primary_offsets = Vec::new();
    for diagnostic in error.diagnostics() {
        assert_eq!(diagnostic.message, "struct layout overflow");
        let primary = diagnostic.primary.span;
        assert_eq!(&source[primary.start..primary.end], "Outer[u8]");
        primary_offsets.push(primary.start);
        let related = diagnostic
            .related
            .iter()
            .map(|label| &source[label.span.start..label.span.end])
            .collect::<Vec<_>>();
        assert_eq!(related, [structure, "Inner[T]"]);
    }
    primary_offsets.sort_unstable();
    let expected = source.match_indices("Outer[u8]").map(|(offset, _)| offset);
    assert!(primary_offsets.into_iter().eq(expected));
}

#[test]
fn struct_layout_limit_only_checks_selected_generic_operator_signatures() {
    let source = "package main
type Buffer[T] [1]T
operator (self: Buffer[T]) [](index: *struct { first: [4294967295]u8; last: T }) -> T { return self[0 as u32] }
func probe(buffer: Buffer[u8], index: u32) -> u8 { return buffer[index] }
func main() {}
";
    compile(source, None).unwrap();

    let selected_source = "package main
type Buffer[T] [1]T
operator (self: Buffer[T]) - () -> *struct { first: [4294967295]u8; last: T } { return nil }
func probe(buffer: Buffer[u8]) { _ = -buffer }
func main() {}
";
    let error = compile(selected_source, None).expect_err("selected operator must be checked");
    let [diagnostic] = error.diagnostics() else {
        panic!("one concrete struct layout error expected: {error}");
    };
    assert_eq!(diagnostic.message, "struct layout overflow");
}

#[test]
fn struct_layout_limit_checks_package_constants_in_their_declared_scope() {
    let structure = "struct { first: [4294967295]u8; last: Element }";
    for main_body in ["", "Use[u8]()"] {
        let source = format!(
            "package main\ntype Element struct {{}}\nconst Word: u32 = sizeof(*{structure})\n\
             func Use[Element]() {{ _ = Word }}\nfunc main() {{ {main_body} }}\n"
        );
        compile(&source, None).unwrap();
        crate::test_support::reject(
            &source.replace("type Element struct {}", "type Element u8"),
            None,
            "struct layout overflow",
            structure,
        );
    }
}

#[test]
fn struct_layout_limit_checks_declarations_first_reached_during_constant_replay() {
    let structure = "struct { first: [4294967295]u8; last: u8 }";
    for name in ["A", "Z"] {
        let source = format!(
            "package main\ntype {name} [Word]u8\ntype TooBig {structure}\n\
             const Word: u32 = sizeof(*TooBig)\nfunc main() {{}}\n"
        );
        crate::test_support::reject(&source, None, "struct layout overflow", structure);
    }
}

#[test]
fn array_validation_uses_the_loaded_source_snapshot() {
    let mut loaded = compile("package main\nfunc main() {}\n", None)
        .unwrap()
        .loaded;
    let path = loaded.sources.files()[0].path().to_path_buf();
    let source = "package main\ntype TooLarge [1073741824]u32\nfunc main() {}\n";
    loaded.sources = crate::source::SourceDb::default();
    loaded.sources.add_file(path, source.into());
    let error =
        crate::compile_loaded_project(loaded).expect_err("current snapshot must be checked");
    assert_eq!(error.diagnostics()[0].message, "array layout overflow");
    let span = error.diagnostics()[0].primary.span;
    assert_eq!(&source[span.start..span.end], "[1073741824]u32");
}

#[test]
fn array_validation_reuses_shared_struct_layouts() {
    fn source(base: &str, depth: usize, declaration: &str) -> String {
        let mut source = format!("package main\ntype S0 {base}\n");
        for index in 1..=depth {
            let previous = index - 1;
            source.push_str(&format!(
                "type S{index} struct {{ a: S{previous}; b: S{previous} }}\n"
            ));
        }
        source.push_str(declaration);
        source.push_str("\nfunc main() {}\n");
        source
    }

    // Rewalking both fields at every level would expand 2^64 paths. Compile
    // smoke checks avoid timing thresholds while making that regression visible.
    for declaration in [
        "type Unused [0]S64",
        "func unused() { var items: [0]S64 }",
        "type Unused [4294967295]S64",
    ] {
        compile(&source("struct {}", 64, declaration), None).unwrap();
    }
    compile(&source("u8", 31, "type Unused [1]S31"), None).unwrap();
    compile(&source("u8", 31, "type Unused [0]S31"), None).unwrap();
    let rejected = source("u8", 31, "type Unused [2]S31");
    crate::test_support::reject(&rejected, None, "array layout overflow", "[2]S31");
}

#[test]
fn generic_declaration_placeholders_do_not_create_concrete_arrays() {
    for (definition, member) in [
        (
            "type Buffer[T] [4294967295]struct { a: T; b: T }",
            "func (b: *Buffer[T]) Do() {}",
        ),
        (
            "type Buffer[T] struct {}",
            "func (b: *Buffer[T]) Do(arg: *[4294967295]struct { a: T; b: T }) {}",
        ),
        (
            "type Buffer[T] [4294967295]struct { a: T; b: T }",
            "operator (b: *Buffer[T]) len() -> u32 { return 0 }",
        ),
        (
            "type Buffer[T] struct {}",
            "operator (b: *Buffer[T]) + (arg: *[4294967295]struct { a: T; b: T }) -> u32 { return 0 }",
        ),
    ] {
        for concrete in ["", "var unused: *Buffer[struct {}]"] {
            let source =
                format!("package main\n{definition}\n{member}\n{concrete}\nfunc main() {{}}\n");
            compile(&source, None).unwrap_or_else(|error| panic!("{source}: {error}"));
        }
    }
}

#[test]
fn generic_member_concrete_arrays_still_obey_the_limit() {
    for member in [
        "func (b: *Buffer[T]) Do() {}",
        "operator (b: *Buffer[T]) len() -> u32 { return 0 }",
    ] {
        let source = format!(
            "package main\ntype Buffer[T] [4294967295]struct {{ a: T; b: T }}\n{member}\nvar unused: *Buffer[u8]\nfunc main() {{}}\n"
        );
        let error = compile(&source, None).expect_err(&source);
        assert!(
            error.to_string().contains("array layout overflow"),
            "{error}"
        );
    }
    for argument in ["struct {}", "u8"] {
        let source = format!(
            "package main\ntype Buffer[T] struct {{}}\nfunc (b: *Buffer[T]) Do(arg: *[4294967295]struct {{ a: T; b: T }}) {{}}\nfunc main() {{ var b: *Buffer[{argument}]; b.Do(nil) }}\n"
        );
        let result = compile(&source, None);
        if argument == "u8" {
            let error = result.expect_err("real method signature must obey the limit");
            assert!(
                error.to_string().contains("array layout overflow"),
                "{error}"
            );
        } else {
            result.expect("zero-size concrete method signature must remain valid");
        }
    }
}

#[test]
fn isolated_generic_signature_validation_keeps_declaration_errors() {
    for (declaration, message) in [
        (
            "type Buffer[T] struct { Do: T }\nfunc (b: *Buffer[T]) Do() {}",
            "conflicts with a field",
        ),
        (
            "type Buffer[T] struct {}\noperator (b: *Buffer[T]) len() -> u8 { return 0 }",
            "operator len must return u32",
        ),
        (
            "type Buffer[T] struct {}\nfunc (b: *Buffer[T]) Do(arg: Missing) {}",
            "unknown type",
        ),
    ] {
        let source = format!("package main\n{declaration}\nfunc main() {{}}\n");
        let error = compile(&source, None).expect_err(&source);
        assert!(error.to_string().contains(message), "{source}: {error}");
        assert!(!error.diagnostics()[0].primary.span.is_synthetic());
    }
}
