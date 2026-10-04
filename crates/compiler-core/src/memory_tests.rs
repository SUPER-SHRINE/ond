use crate::{memory_test_vm::execute, mir::*, test_support::compile};

fn checked(source: &str) -> crate::Compilation {
    let compilation = compile(source, None).unwrap();
    validate_project(&compilation.mir).unwrap();
    compilation
}

#[test]
fn nested_aggregates_copy_by_value_and_pointers_alias_storage() {
    let c = checked(
        r#"package main
type Point struct { x: i32; y: i32; }
type Box struct { p: Point; values: [3]i32; }
func main() {}
func run() -> (i32, i32, i32, i32, i32, i32) {
    var a = [3]i32{1: 7, 8}
    var s = Box{p: Point{x: 1}, values: a}
    var copied = s
    var p = &s.p.x
    *p = 9
    s.values[1] = 3
    a[0], a[1] = a[1], a[0]
    return copied.p.x, copied.values[1], s.p.x, a[0], a[1], s.p.y
}
"#,
    );
    assert_eq!(
        execute(&c.mir, "main.run", &[]).unwrap(),
        [1, 7, 9, 7, 0, 0]
    );
    let f = c.mir.packages[0]
        .functions
        .iter()
        .find(|f| f.name == "main.run")
        .unwrap();
    assert!(f.locals.iter().any(|local| local.address_taken));
    assert!(
        f.blocks
            .iter()
            .flat_map(|b| &b.instructions)
            .any(|i| matches!(
                i.kind,
                InstructionKind::Store {
                    access: AccessKind::Ordered,
                    ..
                }
            ))
    );
}

#[test]
fn aggregate_arguments_returns_and_parallel_assignment_preserve_snapshots() {
    let c = checked(
        r#"package main
func main() {}
func change(a: [2]i32) -> [2]i32 { a[0] = 99; return a; }
func tick(p: *i32) -> i32 { *p = *p + 1; return *p; }
func run() -> (i32, i32, i32, i32) {
    var counter: i32
    var a = [2]i32{1: tick(&counter), 0: tick(&counter)}
    var b = change(a)
    a, b = b, a
    return a[0], b[0], b[1], counter
}
"#,
    );
    assert_eq!(execute(&c.mir, "main.run", &[]).unwrap(), [99, 2, 1, 2]);
}

#[test]
fn string_literals_are_mutable_byte_array_values_and_copy_independently() {
    let c = checked(
        r#"package main
const Text = "音\xFF\000"
const Size = len(Text)
const First = Text[0]
func main() {}
func run() -> (u32, u8, u8, u8, u32) {
    var text = Text
    var copy = text
    text[3] = 42
    text[1] = 0
    var empty: [0]u8 = ""
    return Size, First, text[3], copy[1], len(empty)
}
"#,
    );
    assert_eq!(
        execute(&c.mir, "main.run", &[]).unwrap(),
        [5, 0xe9, 42, 0x9f, 0]
    );
}

#[test]
fn string_literal_array_types_flow_through_globals_arguments_and_returns() {
    let c = checked(
        r#"package main
var Global: [3]u8 = "abc"
const Japanese = "日本語"
const Emoji = "😀"
const Bytes = "\xFF\000"
const Raw = `a
b`
func main() {}
func pass(value: [3]u8) -> [3]u8 { return value; }
func run() -> (u32, u32, u32, u32, u8, u8) {
    var local: [3]u8 = pass("xyz")
    local = "123"
    return len(Japanese), len(Emoji), len(Bytes), len(Raw), Global[2], local[0]
}
"#,
    );
    assert_eq!(
        execute(&c.mir, "main.run", &[]).unwrap(),
        [9, 4, 2, 3, 99, 49]
    );
}

#[test]
fn string_literal_lengths_and_pointer_types_require_exact_matches() {
    for (body, message) in [
        ("var x: [4]u8 = \"abc\"", "exactly match"),
        ("var x: [2]u8 = \"abc\"", "exactly match"),
        ("var x: [3]i8 = \"abc\"", "exactly match"),
        ("var x = \"abc\"\nvar p: *u8 = x", "type mismatch"),
        ("var values: [1]S\nvar p: *u8 = &values[0]", "type mismatch"),
    ] {
        let source =
            format!("package main\ntype S struct {{ byte: u8; }}\nfunc main() {{\n{body}\n}}\n");
        let error = compile(&source, None).unwrap_err();
        assert!(error.to_string().contains(message), "{body}: {error}");
    }
    checked(
        "package main\ntype S struct { byte: u8; }\nfunc main() { var text = \"abc\"; var head: *u8 = &text[0]; var values: [1]S; var first: *S = &values[0]; _ = head; _ = first; }\n",
    );
}

#[test]
fn arrays_including_string_literals_are_bounds_checked_but_raw_pointers_are_not() {
    let c = checked(
        r#"package main
func main() {}
func array(i: i32) -> u8 { var a: [2]u8; return a[i]; }
func text(i: i32) -> u8 { return "ab"[i]; }
func raw(p: *u8, i: u32) -> u8 { return p[i]; }
"#,
    );
    assert_eq!(execute(&c.mir, "main.array", &[1]).unwrap(), [0]);
    for name in ["main.array", "main.text"] {
        assert_eq!(execute(&c.mir, name, &[2]), Err("bounds"));
        assert_eq!(execute(&c.mir, name, &[u64::MAX]), Err("bounds"));
    }
    let f = c.mir.packages[0]
        .functions
        .iter()
        .find(|f| f.name == "main.raw")
        .unwrap();
    assert!(
        !f.blocks
            .iter()
            .flat_map(|b| &b.instructions)
            .any(|i| matches!(i.kind, InstructionKind::Check(_)))
    );
    assert!(
        f.blocks
            .iter()
            .flat_map(|b| &b.instructions)
            .any(|i| matches!(
                i.kind,
                InstructionKind::Load {
                    access: AccessKind::Ordered,
                    ..
                }
            ))
    );
}

#[test]
fn memory_errors_have_source_locations() {
    for (body, message) in [
        ("var a: [2]u8\nvar x = a[-1]", "out of bounds"),
        ("var x = \"ab\"[2]", "out of bounds"),
        ("var a = [2]u8{2: 1}", "out of bounds"),
        ("var a = [2]u8{0: 1, 0: 2}", "duplicate"),
        ("var p = Point{x: 1, x: 2}", "duplicate"),
        ("var p = Point{1}", "struct literal requires named fields"),
        ("var p = Point{missing: 1}", "unknown struct field"),
        ("var p: Point\np.missing = 1", "unknown struct field"),
        ("var p = &Point{x: 1}", "not supported"),
        ("var x = *1", "requires a pointer"),
        ("var p: *u8\nvar i: i32\nvar x = p[i]", "type mismatch"),
        ("var x = len(1)", "requires an array"),
        ("var x = \"\\q\"", "invalid string escape"),
    ] {
        let source =
            format!("package main\ntype Point struct {{ x: i32; }}\nfunc main() {{\n{body}\n}}\n");
        let error = compile(&source, None).unwrap_err();
        assert!(error.to_string().contains(message), "{body}: {error}");
        assert!(!error.diagnostics()[0].primary.span.is_synthetic());
    }
}

#[test]
fn imported_private_fields_remain_private() {
    for body in [
        "var p: shared.Point\nvar x = p.hidden",
        "var p = shared.Point{hidden: 1}",
    ] {
        let error = compile(
            &format!("package main\nimport \"shared\"\nfunc main() {{\n{body}\n}}\n"),
            Some("package shared\ntype Point struct { hidden: i32; Public: i32; }\n"),
        )
        .unwrap_err();
        assert!(error.to_string().contains("not exported"), "{error}");
    }
}

#[test]
fn rejects_corrupted_place_projections_and_unordered_pointer_access() {
    let c = checked(
        "package main\ntype S struct { X: i32; }\nfunc main() {}\nfunc read(p: *S) -> i32 { return p.X; }\n",
    );
    let mut bad = c.mir.clone();
    let function = bad.packages[0]
        .functions
        .iter_mut()
        .find(|f| f.name == "main.read")
        .unwrap();
    for i in function.blocks.iter_mut().flat_map(|b| &mut b.instructions) {
        if let InstructionKind::Load { place, access } = &mut i.kind {
            if matches!(place.base, PlaceBase::Pointer(_)) {
                *access = AccessKind::Local;
                place.projections = vec![Projection::Field {
                    index: 99,
                    ty: TypeId::I32,
                }];
            }
        }
    }
    let errors = validate_project(&bad).unwrap_err();
    assert!(errors.iter().any(|error| error.message.contains("ordered")));
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("projection"))
    );
}

#[test]
fn decodes_raw_unicode_octal_and_invalid_string_escapes() {
    assert_eq!(crate::string_literal::decode("`a\r\nb`").unwrap(), b"a\nb");
    assert_eq!(
        crate::string_literal::decode(r#""\u97f3\101\xFF\000""#).unwrap(),
        [0xe9, 0x9f, 0xb3, 65, 255, 0]
    );
    for (source, message) in [
        (r#""\uD800""#, "invalid Unicode scalar in string escape"),
        (r#""\U00110000""#, "invalid Unicode scalar in string escape"),
        (r#""\400""#, "string byte escape out of range"),
        (r#""\x0""#, "invalid numeric string escape"),
    ] {
        assert_eq!(
            crate::string_literal::decode(source).unwrap_err(),
            message,
            "{source}"
        );
    }
}
