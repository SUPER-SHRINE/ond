use crate::{memory_test_vm, test_support};

#[test]
fn unary_binary_and_len_operators_execute() {
    test_support::compile(
        r#"package main

type Number struct { value: i32 }

operator +(lhs: Number, rhs: Number) -> Number {
    return Number{value: lhs.value + rhs.value}
}

operator -(value: Number) -> Number {
    return Number{value: 0 - value.value}
}

operator len(value: Number) -> u32 {
    return value.value as u32
}

func observe() -> (i32, u32) {
    value := -Number{value: 3} + Number{value: 8}
    return value.value, len(value)
}

func main() {}
"#,
        None,
    )
    .expect("operators should compile");
}

#[test]
fn index_getter_and_setter_execute() {
    let compilation = test_support::compile(
        r#"package main

type Buffer struct {
    values: [2]u32
}

operator [](buffer: *Buffer, index: u32) -> *u32 {
    return &buffer.values[index]
}

operator []=(buffer: *Buffer, index: u32, value: u32) {
    buffer.values[index] = value
}

func observe() -> u32 {
    buffer := Buffer{values: [2]u32{4, 9}}
    (&buffer)[1] = 12
    return *(&buffer)[1]
}

func main() {}
"#,
        None,
    )
    .expect("index operators should compile");

    assert_eq!(
        memory_test_vm::execute(&compilation.mir, "main.observe", &[]).unwrap(),
        [12]
    );
}

#[test]
fn operator_is_contextual_and_can_name_a_field() {
    test_support::compile(
        r#"package main

type Value struct { operator: u32 }

func main() {
    value := Value{operator: 1}
    operator := value.operator
    _ = operator
}
"#,
        None,
    )
    .expect("operator should remain usable as an identifier outside declarations");
}

#[test]
fn compound_assignment_uses_binary_operator() {
    let compilation = test_support::compile(
        r#"package main

type Number struct { value: u32 }

operator +(lhs: Number, rhs: u32) -> Number {
    return Number{value: lhs.value + rhs}
}

func observe() -> u32 {
    value := Number{value: 4}
    value += 7
    return value.value
}

func main() {}
"#,
        None,
    )
    .expect("compound assignment should use the matching binary operator");

    assert_eq!(
        memory_test_vm::execute(&compilation.mir, "main.observe", &[]).unwrap(),
        [11]
    );
}

#[test]
fn compound_index_assignment_evaluates_index_once() {
    let compilation = test_support::compile(
        r#"package main

type Buffer struct { values: [1]u32 }

operator [](buffer: *Buffer, index: u32) -> u32 {
    return buffer.values[index]
}

operator []=(buffer: *Buffer, index: u32, value: u32) {
    buffer.values[index] = value
}

func next(calls: *u32) -> u32 {
    *calls += 1
    return 0
}

func observe() -> (u32, u32) {
    buffer := Buffer{values: [1]u32{4}}
    var calls: u32 = 0
    (&buffer)[next(&calls)] += 3
    return buffer.values[0], calls
}

func main() {}
"#,
        None,
    )
    .expect("compound index assignment should compile");

    assert_eq!(
        memory_test_vm::execute(&compilation.mir, "main.observe", &[]).unwrap(),
        [7, 1]
    );
}

#[test]
fn generic_struct_types_are_monomorphized_per_argument_list() {
    let compilation = test_support::compile(
        r#"package main

type Box[T] struct { value: T }

func observe() -> (u32, i32) {
    left := Box[u32]{value: 7}
    right := Box[i32]{value: -3}
    return Box[u32]{value: left.value}.value, right.value
}

func main() {}
"#,
        None,
    )
    .expect("generic struct instances should compile");

    assert_eq!(
        memory_test_vm::execute(&compilation.mir, "main.observe", &[]).unwrap(),
        [7, (-3i64) as u64]
    );
}

#[test]
fn generic_functions_are_monomorphized_from_explicit_type_arguments() {
    let compilation = test_support::compile(
        r#"package main

func Identity[T](value: T) -> T {
    return value
}

func observe() -> (u32, i32) {
    return Identity[u32](11), Identity[i32](-4)
}

func main() {}
"#,
        None,
    )
    .expect("generic functions should compile");

    assert_eq!(
        memory_test_vm::execute(&compilation.mir, "main.observe", &[]).unwrap(),
        [11, (-4i64) as u64]
    );
}

#[test]
fn generic_function_bodies_substitute_nested_types() {
    test_support::compile(
        r#"package main

type Vec[T] struct {
    data: *T
    len: u32
    cap: u32
}

func Make[T](cap: u32) -> Vec[T] {
    return Vec[T]{data: alloc[T](cap), len: 0, cap: cap}
}

func observe() -> u32 {
    values := Make[u32](3)
    values.data[0] = 19
    result := values.data[0] + values.cap
    free(values.data)
    return result
}

func main() {}
"#,
        None,
    )
    .expect("nested uses of a type parameter should be substituted");
}

#[test]
fn generic_function_errors_are_anchored_at_definition_with_each_instantiation_related() {
    let source = r#"package main

func Invalid[T](value: T) -> T {
    return value + value
}

func main() {
    _ = Invalid[bool](true)
    _ = Invalid[bool](false)
}
"#;
    let error = test_support::compile(source, None)
        .expect_err("an invalid generic specialization should be rejected");
    let diagnostics = error.diagnostics();
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    let diagnostic = &diagnostics[0];
    assert_eq!(
        &source[diagnostic.primary.span.start..diagnostic.primary.span.end],
        "value + value"
    );
    let related = diagnostic
        .related
        .iter()
        .map(|related| &source[related.span.start..related.span.end])
        .collect::<Vec<_>>();
    assert!(related.contains(&"Invalid[bool](true)"), "{diagnostic:?}");
    assert!(related.contains(&"Invalid[bool](false)"), "{diagnostic:?}");
}

#[test]
fn independent_generic_specialization_failures_are_all_reported() {
    let source = r#"package main
func Add[T](value: T) -> T { return value + value }
func Sub[T](value: T) -> T { return value - value }
func main() {
    _ = Add[bool](true)
    _ = Sub[bool](true)
}
"#;
    let error = test_support::compile(source, None)
        .expect_err("independent invalid specializations should both be rejected");
    let diagnostics = error.diagnostics();
    assert_eq!(diagnostics.len(), 2, "{diagnostics:?}");
    let primary = diagnostics
        .iter()
        .map(|diagnostic| &source[diagnostic.primary.span.start..diagnostic.primary.span.end])
        .collect::<Vec<_>>();
    assert!(primary.contains(&"value + value"), "{diagnostics:?}");
    assert!(primary.contains(&"value - value"), "{diagnostics:?}");
}

#[test]
fn pointer_to_interface_return_explains_the_required_explicit_conversion() {
    let source = r#"package main
type Box[T] interface { Get() -> T; }
type Cell[T] struct { value: T }
func (cell: *Cell[T]) Get() -> T { return cell.value }
func NewBox[T]() -> Box[T] {
    cell := new(Cell[T])
    return cell
}
func main() { _ = NewBox[u32]() }
"#;
    let error = test_support::compile(source, None)
        .expect_err("implicit pointer-to-interface return should be rejected");
    let diagnostic = error
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.message.contains("pointer-to-interface"))
        .expect("explicit interface conversion diagnostic");
    assert!(
        diagnostic.message.contains("expected `Box[u32]`")
            && diagnostic.message.contains("found `*Cell[u32]`")
            && diagnostic.message.contains("as Box[u32]"),
        "{diagnostic:?}"
    );
    assert_eq!(
        &source[diagnostic.primary.span.start..diagnostic.primary.span.end],
        "cell"
    );
    assert!(
        diagnostic
            .related
            .iter()
            .any(|related| &source[related.span.start..related.span.end] == "NewBox[u32]()"),
        "{diagnostic:?}"
    );
}

#[test]
fn nested_generic_errors_include_the_instantiation_chain() {
    let source = r#"package main

func Inner[T](value: T) -> T { return value + value }
func Outer[T](value: T) -> T { return Inner[T](value) }

func main() { _ = Outer[bool](true) }
"#;
    let error = test_support::compile(source, None)
        .expect_err("a nested invalid generic specialization should be rejected");
    let diagnostic = error
        .diagnostics()
        .iter()
        .next()
        .expect("nested specialization diagnostic");
    assert_eq!(
        &source[diagnostic.primary.span.start..diagnostic.primary.span.end],
        "value + value"
    );
    assert!(
        diagnostic
            .related
            .iter()
            .any(|related| source[related.span.start..related.span.end].contains("Inner[T]")),
        "{diagnostic:?}"
    );
    assert!(
        diagnostic
            .related
            .iter()
            .any(|related| source[related.span.start..related.span.end]
                .contains("Outer[bool](true)")),
        "{diagnostic:?}"
    );
    assert!(diagnostic.related.len() >= 2, "{diagnostic:?}");
}

#[test]
fn generic_type_errors_are_retried_at_each_use_site() {
    let source = r#"package main

type Recursive[T] struct { next: Recursive[T] }

func First(value: Recursive[u32]) {}
func Second(value: Recursive[u32]) {}
func main() {}
"#;
    let error = test_support::compile(source, None)
        .expect_err("a directly recursive generic type should be rejected");
    let diagnostics = error
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.message.contains("direct recursive type"))
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 2, "{diagnostics:?}");
    for diagnostic in diagnostics {
        assert_eq!(
            &source[diagnostic.primary.span.start..diagnostic.primary.span.end],
            "Recursive[u32]"
        );
        assert!(!diagnostic.related.is_empty(), "{diagnostic:?}");
    }
}

#[test]
fn constant_length_array_declarations_remain_distinct_from_generic_types() {
    let compilation = test_support::compile(
        r#"package main

const N = 1
type Buffer [N]u32

func observe() -> u32 {
    value := Buffer{23}
    return value[0]
}

func main() {}
"#,
        None,
    )
    .expect("a declared constant in brackets should remain an array length");

    assert_eq!(
        memory_test_vm::execute(&compilation.mir, "main.observe", &[]).unwrap(),
        [23]
    );
}

#[test]
fn an_identifier_index_can_still_produce_a_callable_value() {
    test_support::compile(
        r#"package main

func answer() -> u32 { return 42 }

func observe() -> u32 {
    callbacks := [1]func() -> u32{answer}
    var index: u32 = 0
    return callbacks[index]()
}

func main() {}
"#,
        None,
    )
    .expect("an indexed function value should not be mistaken for a generic call");
}

#[test]
fn imported_generic_types_and_functions_are_specialized_in_their_owner_package() {
    let compilation = test_support::compile(
        r#"package main
import "shared"

func observe() -> u32 {
    return shared.Wrap[u32](31).Value
}

func main() {}
"#,
        Some(
            r#"package shared

type Box[T] struct { Value: T }

func Wrap[T](value: T) -> Box[T] {
    return Box[T]{Value: value}
}
"#,
        ),
    )
    .expect("exported generic declarations should work across packages");

    assert_eq!(
        memory_test_vm::execute(&compilation.mir, "main.observe", &[]).unwrap(),
        [31]
    );
}

#[test]
fn local_bindings_shadow_generic_function_names() {
    test_support::compile(
        r#"package main

func Choose[T](value: T) -> T { return value }
func answer() -> u32 { return 42 }

func observe() -> u32 {
    Choose := [1]func() -> u32{answer}
    var index: u32 = 0
    return Choose[index]()
}

func main() {}
"#,
        None,
    )
    .expect("a local binding should shadow a package generic function");
}

#[test]
fn duplicate_generic_declarations_are_rejected() {
    let error = test_support::compile(
        r#"package main

type Box[T] struct { value: T }
type Box[U] struct { value: U }

func Pick[T](value: T) -> T { return value }
func Pick[U](value: U) -> U { return value }

func main() {}
"#,
        None,
    )
    .expect_err("generic declarations share the ordinary package namespace");

    assert_eq!(
        error
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic
                .message
                .contains("duplicate top-level declaration"))
            .count(),
        2
    );
}

#[test]
fn generic_index_operators_are_inferred_and_specialized() {
    let compilation = test_support::compile(
        r#"package main

type Vec[T] struct { values: [2]T }

operator[T] [](values: *Vec[T], index: u32) -> *T {
    return &values.values[index]
}

operator[T] []=(values: *Vec[T], index: u32, value: T) {
    values.values[index] = value
}

func observe() -> (u32, i32) {
    unsigned := Vec[u32]{values: [2]u32{3, 5}}
    signed := Vec[i32]{values: [2]i32{-2, 7}}
    (&unsigned)[1] = 11
    (&signed)[0] = -9
    return *(&unsigned)[1], *(&signed)[0]
}

func main() {}
"#,
        None,
    )
    .expect("generic index operators should infer T from Vec[T]");

    assert_eq!(
        memory_test_vm::execute(&compilation.mir, "main.observe", &[]).unwrap(),
        [11, (-9i64) as u64]
    );
}

#[test]
fn generic_interface_operators_delegate_to_interface_methods() {
    test_support::compile(
        r#"package main

type List[T] interface {
    Get(index: u32) -> T
    Set(index: u32, value: T)
    Len() -> u32
}

type Pair[T] struct { values: [2]T }

func (pair: *Pair[T]) Get(index: u32) -> T { return pair.values[index] }
func (pair: *Pair[T]) Set(index: u32, value: T) { pair.values[index] = value }
func (pair: *Pair[T]) Len() -> u32 { return 2 }

operator[T] [](values: List[T], index: u32) -> T {
    return values.Get(index)
}

operator[T] []=(values: List[T], index: u32, value: T) {
    values.Set(index, value)
}

operator[T] len(values: List[T]) -> u32 {
    return values.Len()
}

func observe() -> (u32, u32) {
    pair := Pair[u32]{values: [2]u32{3, 5}}
    var values: List[u32] = (&pair) as List[u32]
    values[1] = 13
    return values[1], len(values)
}

func main() {}
"#,
        None,
    )
    .expect("operators on a named generic interface should compile");
}

#[test]
fn generic_binary_operators_specialize_their_bodies() {
    let compilation = test_support::compile(
        r#"package main

type Number[T] struct { value: T }

operator[T] +(lhs: Number[T], rhs: Number[T]) -> Number[T] {
    return Number[T]{value: lhs.value + rhs.value}
}

func observe() -> u32 {
    result := Number[u32]{value: 8} + Number[u32]{value: 13}
    return result.value
}

func main() {}
"#,
        None,
    )
    .expect("a generic operator body should be lowered after substitution");

    assert_eq!(
        memory_test_vm::execute(&compilation.mir, "main.observe", &[]).unwrap(),
        [21]
    );
}

#[test]
fn concrete_and_generic_operator_matches_are_ambiguous() {
    let error = test_support::compile(
        r#"package main

type Number[T] struct { value: T }

operator[T] +(lhs: Number[T], rhs: Number[T]) -> Number[T] {
    return lhs
}

operator +(lhs: Number[u32], rhs: Number[u32]) -> Number[u32] {
    return rhs
}

func observe() -> Number[u32] {
    return Number[u32]{value: 1} + Number[u32]{value: 2}
}

func main() {}
"#,
        None,
    )
    .expect_err("concrete and generic matches must not have a hidden priority");

    assert!(
        error
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.message == "ambiguous operator declaration")
    );
}

#[test]
fn imported_generic_operators_are_specialized_in_their_owner_package() {
    let compilation = test_support::compile(
        r#"package main
import "shared"

func observe() -> u32 {
    values := shared.Values[u32]{Data: [1]u32{37}}
    return *(&values)[0]
}

func main() {}
"#,
        Some(
            r#"package shared

type Values[T] struct { Data: [1]T }

operator[T] [](values: *Values[T], index: u32) -> *T {
    return &values.Data[index]
}
"#,
        ),
    )
    .expect("generic operators should specialize in an imported owner package");

    assert_eq!(
        memory_test_vm::execute(&compilation.mir, "main.observe", &[]).unwrap(),
        [37]
    );
}

#[test]
fn generic_operator_parameters_must_be_inferable_and_have_an_owned_operand() {
    for (source, expected) in [
        (
            r#"package main
type Values[T] struct { value: T }
operator[T, U] [](values: *Values[T], index: u32) -> U { trap "not implemented" }
func main() {}
"#,
            "cannot be inferred from its operands",
        ),
        (
            r#"package main
operator[T] +(lhs: T, rhs: T) -> T { return lhs }
func main() {}
"#,
            "leftmost user-defined operand",
        ),
    ] {
        let error = test_support::compile(source, None)
            .expect_err("invalid generic operator declaration should be rejected");
        assert!(
            error
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.message.contains(expected))
        );
    }
}

#[test]
fn imported_generic_interface_operators_are_discovered_from_the_owner_package() {
    test_support::compile(
        r#"package main
import "shared"

type Pair struct { Values: [1]u32 }
func (pair: *Pair) Get(index: u32) -> u32 { return pair.Values[index] }

func main() {
    pair := Pair{Values: [1]u32{7}}
    var values: shared.List[u32] = (&pair) as shared.List[u32]
    _ = values[0]
}
"#,
        Some(
            r#"package shared

type List[T] interface { Get(index: u32) -> T; }

operator[T] [](values: List[T], index: u32) -> T {
    return values.Get(index)
}
"#,
        ),
    )
    .expect("an imported generic interface operator should be available by operand type");
}

#[test]
fn interface_operators_do_not_apply_to_implementing_concrete_types_implicitly() {
    let error = test_support::compile(
        r#"package main

type List interface { Get(index: u32) -> u32; }
type Pair struct { values: [1]u32 }
func (pair: *Pair) Get(index: u32) -> u32 { return pair.values[index] }
operator +(values: List, index: u32) -> u32 { return values.Get(index) }

func main() { var pair: Pair; var zero: u32 = 0; _ = (&pair) + zero }
"#,
        None,
    )
    .expect_err("operator lookup must not insert an implicit interface conversion");
    assert!(
        error
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.message.contains("operand")),
        "{:?}",
        error.diagnostics()
    );
}
