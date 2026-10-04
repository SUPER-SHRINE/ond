use crate::{memory_test_vm, test_support};

#[test]
fn accepts_all_compound_assignments_and_postfix_updates() {
    test_support::compile(
        r#"package main

func integer_ops(value: u32) -> u32 {
    value += 3
    value -= 1
    value *= 2
    value /= 2
    value %= 7
    value &= 15
    value |= 16
    value ^= 3
    value &^= 1
    value <<= 2
    value >>= 1
    value++
    value--
    return value
}

func float_ops(value: f32) -> f32 {
    value += 1.0
    value -= 1.0
    value *= 2.0
    value /= 2.0
    value++
    value--
    return value
}

func main() {}
"#,
        None,
    )
    .expect("all compound assignments and postfix updates should compile");
}

#[test]
fn compound_target_is_evaluated_once() {
    let compilation = test_support::compile(
        r#"package main

func next(counter: *u32) -> u32 {
    *counter += 1
    return 0
}

func observe() -> (u32, u32) {
    values := [1]u32{2}
    var calls: u32 = 0
    values[next(&calls)] += 3
    return values[0], calls
}

func main() {}
"#,
        None,
    )
    .expect("compound indexed assignment should compile");

    assert_eq!(
        memory_test_vm::execute(&compilation.mir, "main.observe", &[]).unwrap(),
        [5, 1]
    );
}

#[test]
fn rejects_updates_as_expressions_and_prefix_updates() {
    test_support::reject(
        "package main\nfunc main() { value := 1; other := value++; _ = other }",
        None,
        "expected",
        "++",
    );
    test_support::reject(
        "package main\nfunc main() { value := 1; ++value }",
        None,
        "expected expression",
        "++",
    );
}
