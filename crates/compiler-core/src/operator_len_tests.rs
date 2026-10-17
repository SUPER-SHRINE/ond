use crate::{memory_test_vm, test_support};

fn assert_rejected(source: &str, expected: &str) {
    let error =
        test_support::compile(source, None).expect_err("invalid len use should be rejected");
    assert!(
        error
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.message.contains(expected)),
        "expected {expected:?}: {:?}\n{source}",
        error.diagnostics()
    );
}

#[test]
fn len_requires_one_explicit_u32_result_even_when_unused() {
    for (result, expected) in [
        ("", "operator must return exactly one value"),
        ("-> i32", "operator len must return u32"),
        ("-> (u32, u32)", "operator must return exactly one value"),
        ("-> Count", "operator len must return u32"),
    ] {
        for usage in ["", "_ = len(Value{})"] {
            assert_rejected(
                &format!(
                    "package main\n\
                     type Value struct {{}}\n\
                     type Count u32\n\
                     operator (self: Value) len() {result} {{ trap \"unused body\" }}\n\
                     func main() {{ {usage} }}\n"
                ),
                expected,
            );
        }
    }
}

#[test]
fn generic_len_requires_one_explicit_u32_result_even_when_unused() {
    for (result, expected) in [
        ("", "operator must return exactly one value"),
        ("-> i32", "operator len must return u32"),
        ("-> (u32, u32)", "operator must return exactly one value"),
        ("-> Count", "operator len must return u32"),
        ("-> T", "operator len must return u32"),
    ] {
        for usage in ["", "_ = len(Value[u32]{})"] {
            assert_rejected(
                &format!(
                    "package main\n\
                     type Value[T] struct {{ value: T }}\n\
                     type Count u32\n\
                     operator (self: Value[T]) len() {result} {{ trap \"unused body\" }}\n\
                     func main() {{ {usage} }}\n"
                ),
                expected,
            );
        }
    }
}

#[test]
fn len_results_are_u32_for_value_generic_pointer_and_array_receivers() {
    let compilation = test_support::compile(
        r#"package main
type Value struct { size: u32 }
type Box[T] struct { value: T; size: u32 }
type Buffer [2]u8
operator (self: Value) len() -> (u32) { return self.size }
operator (self: *Box[T]) len() -> u32 { return self.size }
operator (self: Buffer) len() -> u32 { return 99 }
func accept(value: u32) -> u32 { return value }
func observe() -> (u32, u32, u32, u32, u32) {
    value := Value{size: 3}
    unsigned := Box[u32]{value: 1, size: 5}
    boolean := Box[bool]{value: true, size: 7}
    var size: u32 = len(value)
    size = len(&value)
    return size, accept(len(unsigned)), len(&boolean), len(Buffer{}), len("日本語")
}
func main() {}
"#,
        None,
    )
    .expect("len should produce one u32 and preserve receiver and array selection");
    assert_eq!(
        memory_test_vm::execute(&compilation.mir, "main.observe", &[]).unwrap(),
        [3, 5, 7, 2, 9]
    );
}

#[test]
fn generic_interface_len_result_is_u32() {
    test_support::compile(
        r#"package main
type List[T] interface { Len() -> u32 }
type Value struct { size: u32 }
func (self: *Value) Len() -> u32 { return self.size }
operator (self: List[T]) len() -> u32 { return self.Len() }
func observe() -> u32 {
    value := Value{size: 11}
    var list: List[bool] = (&value) as List[bool]
    var size: u32 = len(list)
    return size
}
func main() {}
"#,
        None,
    )
    .expect("a generic interface len should produce one u32");
}

#[test]
fn len_cannot_expand_into_multiple_values() {
    for body in [
        "first, second := len(Value{}); return first, second",
        "var first: u32; var second: u32; first, second = len(Value{}); return first, second",
        "return len(Value{})",
    ] {
        assert_rejected(
            &format!(
                "package main\n\
                 type Value struct {{}}\n\
                 operator (self: Value) len() -> u32 {{ return 3 }}\n\
                 func observe() -> (u32, u32) {{ {body} }}\n\
                 func main() {{}}\n"
            ),
            "value count mismatch: expected 2, found 1",
        );
    }
}

#[test]
fn expected_result_type_does_not_change_len_selection() {
    for (declaration, receiver, value) in [
        ("type Value struct {}", "Value", "Value{}"),
        (
            "type Value[T] struct { value: T }",
            "Value[T]",
            "Value[u32]{}",
        ),
    ] {
        assert_rejected(
            &format!(
                "package main\n\
                 {declaration}\n\
                 operator (self: {receiver}) len() -> u32 {{ return 3 }}\n\
                 func observe() -> i32 {{ return len({value}) }}\n\
                 func main() {{}}\n"
            ),
            "type mismatch",
        );
    }
}
