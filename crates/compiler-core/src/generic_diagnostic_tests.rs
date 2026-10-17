use crate::test_support::compile;

fn assert_related(source: &str, error: &crate::FrontendError, primary: &str, related: &[&str]) {
    let diagnostics = error
        .diagnostics()
        .iter()
        .filter(|diagnostic| {
            &source[diagnostic.primary.span.start..diagnostic.primary.span.end] == primary
        })
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    let spans = diagnostics[0]
        .related
        .iter()
        .map(|label| &source[label.span.start..label.span.end])
        .collect::<Vec<_>>();
    let unique_spans = diagnostics[0]
        .related
        .iter()
        .map(|label| label.span)
        .collect::<Vec<_>>();
    assert!(
        unique_spans
            .iter()
            .enumerate()
            .all(|(index, span)| !unique_spans[..index].contains(span)),
        "{spans:?}"
    );
    assert_eq!(spans.len(), related.len(), "{spans:?}");
    for expected in related {
        let expected_count = related
            .iter()
            .filter(|candidate| *candidate == expected)
            .count();
        assert_eq!(
            spans.iter().filter(|span| **span == *expected).count(),
            expected_count,
            "wrong number of {expected:?} labels: {spans:?}"
        );
    }
}

#[test]
fn failed_function_specialization_collects_late_nested_requests() {
    let source = r#"package main
func Invalid[T](value: T) -> T { return value + value }
func Bridge[T](value: T) { _ = Invalid[T](value) }
func Outer[T](value: T) { Bridge[T](value) }
func main() {
    _ = Invalid[bool](true)
    Bridge[bool](false)
    Outer[bool](true)
    Outer[bool](false)
}
"#;
    let error = compile(source, None).expect_err("bool addition in a generic body must fail");
    assert_eq!(error.diagnostics().len(), 1, "{:?}", error.diagnostics());
    assert_related(
        source,
        &error,
        "value + value",
        &[
            "Invalid[bool](true)",
            "Bridge[bool](false)",
            "Outer[bool](true)",
            "Outer[bool](false)",
            "Bridge[T](value)",
            "Invalid[T](value)",
        ],
    );
}

#[test]
fn failed_generic_method_collects_late_requests() {
    let source = r#"package main
type Box[T] struct { value: T }
func (box: Box[T]) Invalid() -> T { return box.value + box.value }
func (box: Box[T]) Later() { _ = box.Invalid() }
func main() {
    box := Box[bool]{value: true}
    _ = box.Invalid()
    box.Later()
}
"#;
    let error = compile(source, None).expect_err("bool addition in a generic method must fail");
    assert_eq!(error.diagnostics().len(), 1, "{:?}", error.diagnostics());
    assert_related(
        source,
        &error,
        "box.value + box.value",
        &["box.Invalid()", "box.Later()", "box.Invalid()"],
    );
}

#[test]
fn failed_generic_operator_collects_late_method_requests() {
    let source = r#"package main
type Box[T] struct { value: T }
type Wrap[T] struct { value: Box[T] }
operator (lhs: Box[T]) + (rhs: Box[T]) -> Box[T] {
    return Box[T]{value: lhs.value + rhs.value}
}

operator (wrap: Wrap[T]) len() -> u32 {
    _ = wrap.value + wrap.value
    return 0
}

func main() {
    box := Box[bool]{value: true}
    _ = box + box
    wrap := Wrap[bool]{value: box}
    _ = len(wrap)
}
"#;
    let error = compile(source, None).expect_err("bool addition in a generic operator must fail");
    assert_eq!(error.diagnostics().len(), 1, "{:?}", error.diagnostics());
    assert_related(
        source,
        &error,
        "lhs.value + rhs.value",
        &["box + box", "len(wrap)", "wrap.value + wrap.value"],
    );
}

#[test]
fn mutual_generic_recursion_keeps_failed_descendant_reachable() {
    let source = r#"package main
func First[T](value: T) { Second[T](value) }
func Second[T](value: T) {
    First[T](value)
    Invalid[T](value)
}

func Invalid[T](value: T) { _ = value + value }
func main() {
    First[bool](true)
    Second[bool](false)
}
"#;
    let error = compile(source, None).expect_err("bool addition in a reachable generic must fail");
    assert_eq!(error.diagnostics().len(), 1, "{:?}", error.diagnostics());
    assert_related(
        source,
        &error,
        "value + value",
        &[
            "First[bool](true)",
            "Second[bool](false)",
            "Second[T](value)",
            "First[T](value)",
            "Invalid[T](value)",
        ],
    );
}

#[test]
fn mutually_recursive_generic_instances_compile_and_terminate() {
    let source = r#"package main
func First[T](value: T) { Second[T](value) }
func Second[T](value: T) { First[T](value) }
func main() { First[u32](1) }
"#;
    compile(source, None).expect("a recursive generic call cycle should reach a fixed point");
}

#[test]
fn densely_mutually_recursive_generic_instances_compile() {
    let source = r#"package main
func First[T](value: T) {
    Second[T](value)
    Third[T](value)
    Fourth[T](value)
}
func Second[T](value: T) {
    First[T](value)
    Third[T](value)
    Fourth[T](value)
}
func Third[T](value: T) {
    First[T](value)
    Second[T](value)
    Fourth[T](value)
}
func Fourth[T](value: T) {
    First[T](value)
    Second[T](value)
    Third[T](value)
}
func main() { First[u32](1) }
"#;
    compile(source, None).expect("dense mutual recursion should converge over distinct spans");
}

#[test]
fn generic_self_recursion_merges_distinct_call_site_spans() {
    let source = r#"package main
func Recursive[T](value: T) {
    Recursive[T](value)
    Recursive[T](value)
    Recursive[T](value)
    Recursive[T](value)
    Recursive[T](value)
    Recursive[T](value)
    Recursive[T](value)
}
func main() { Recursive[u32](1) }
"#;
    compile(source, None).expect("self-recursive call sites should merge by root use");
}

#[test]
fn fifteen_mutually_recursive_generic_instances_compile() {
    let functions = (0..15)
        .map(|index| format!("Node{index:02}"))
        .collect::<Vec<_>>();
    let mut source = String::from("package main\n");
    for (index, function) in functions.iter().enumerate() {
        source.push_str(&format!("func {function}[T](value: T) {{\n"));
        for (other, target) in functions.iter().enumerate() {
            if index != other {
                source.push_str(&format!("    {target}[T](value)\n"));
            }
        }
        source.push_str("}\n");
    }
    source.push_str("func main() { Node00[u32](1) }\n");

    compile(&source, None).expect("dense 15-node mutual recursion should converge");
}

#[test]
fn two_hundred_forty_mutually_recursive_generic_instances_compile() {
    let source = generic_ring_source(false);
    compile(&source, None).expect("a 240-node mutual recursion ring should converge");
}

fn generic_ring_source(failing: bool) -> String {
    let mut source = String::from("package main\n");
    for index in 0..240 {
        source.push_str(&format!(
            "func Node{index:03}[T](value: T) {{ Node{:03}[T](value)\n{}\n}}\n",
            (index + 1) % 240,
            if failing && index == 239 {
                "Invalid[T](value)"
            } else {
                ""
            },
        ));
    }
    if failing {
        source.push_str("func Invalid[T](value: T) { _ = value + value }\n");
        source.push_str("func main() { Node000[bool](true) }\n");
    } else {
        source.push_str("func main() { Node000[u32](1) }\n");
    }
    source
}

#[test]
fn failed_descendant_collects_all_requests_through_a_large_generic_ring() {
    let source = generic_ring_source(true);
    let error = compile(&source, None).expect_err("the reachable bool addition must fail");
    assert_eq!(error.diagnostics().len(), 1, "{:?}", error.diagnostics());
    let mut related = (0..240)
        .map(|index| format!("Node{index:03}[T](value)"))
        .collect::<Vec<_>>();
    related.push("Node000[bool](true)".into());
    related.push("Invalid[T](value)".into());
    assert_related(
        &source,
        &error,
        "value + value",
        &related.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    let labels = &error.diagnostics()[0].related;
    assert_eq!(
        labels
            .iter()
            .filter(|label| label.message == "generic function instantiated here")
            .count(),
        1,
    );
    assert_eq!(
        labels
            .iter()
            .filter(|label| label.message == "nested generic function instantiation requested here")
            .count(),
        241,
    );
}

#[test]
fn failed_generic_interface_method_tracks_single_conversion_request() {
    let source = r#"package main
type Reader interface { Read() -> bool; }
type Box[T] struct { value: T }
func (box: *Box[T]) Read() -> T { return box.value + box.value }
func Convert[T](box: *Box[T]) -> Reader { return box as Reader }
func main() {
    box := new(Box[bool])
    _ = box as Reader
}
"#;
    let error = compile(source, None).expect_err("bool addition in generic method must fail");
    assert_eq!(error.diagnostics().len(), 1, "{:?}", error.diagnostics());
    assert!(
        error.diagnostics()[0]
            .message
            .contains("invalid binary operand type")
    );
    assert_related(
        source,
        &error,
        "box.value + box.value",
        &[
            "box as Reader",
            "func (box: *Box[T]) Read() -> T { return box.value + box.value }",
        ],
    );
}

#[test]
fn failed_generic_interface_method_tracks_repeated_shared_vtable_requests() {
    let source = r#"package main
type Reader interface { Read() -> bool; }
type Box[T] struct { value: T }
func (box: *Box[T]) Read() -> T { return box.value + box.value }
func Convert[T](box: *Box[T]) -> Reader { return box as Reader }
func main() {
    box := new(Box[bool])
    _ = box as Reader
    _ = box as Reader
}
"#;
    let error = compile(source, None).expect_err("bool addition in generic method must fail");
    assert_eq!(error.diagnostics().len(), 1, "{:?}", error.diagnostics());
    assert!(
        error.diagnostics()[0]
            .message
            .contains("invalid binary operand type")
    );
    assert_related(
        source,
        &error,
        "box.value + box.value",
        &[
            "box as Reader",
            "box as Reader",
            "func (box: *Box[T]) Read() -> T { return box.value + box.value }",
        ],
    );
}

#[test]
fn failed_generic_interface_method_tracks_nested_requests() {
    let source = r#"package main
type Reader interface { Read() -> bool; }
type Box[T] struct { value: T }
func (box: *Box[T]) Read() -> T { return box.value + box.value }
func Convert[T](box: *Box[T]) -> Reader { return box as Reader }
func Outer[T](box: *Box[T]) -> Reader { return Convert[T](box) }
func main() {
    box := new(Box[bool])
    _ = box.Read()
    _ = Convert[bool](box)
    _ = Outer[bool](box)
}
"#;
    let error = compile(source, None).expect_err("bool addition in generic method must fail");
    assert_eq!(error.diagnostics().len(), 1, "{:?}", error.diagnostics());
    assert!(
        error.diagnostics()[0]
            .message
            .contains("invalid binary operand type")
    );
    assert_related(
        source,
        &error,
        "box.value + box.value",
        &[
            "box.Read()",
            "Convert[bool](box)",
            "Outer[bool](box)",
            "Convert[T](box)",
            "box as Reader",
            "func (box: *Box[T]) Read() -> T { return box.value + box.value }",
        ],
    );
}
