use crate::{memory_test_vm, test_support};

#[test]
fn compound_assignment_dereferences_receiver_before_rhs() {
    let compilation = test_support::compile(
        r#"package main
type N struct { value: u32 }
var result: N
operator (self: N) + (other: u32) -> *N {
    result.value = self.value + other
    return &result
}
func change(value: *N) -> u32 { value.value = 9; return 2 }
func observe() -> u32 {
    value := N{value: 1}
    pointer := &value
    pointer += change(&value)
    return pointer.value
}
func main() {}
"#,
        None,
    )
    .expect("compound assignment should compile");
    assert_eq!(
        memory_test_vm::execute(&compilation.mir, "main.observe", &[]).unwrap(),
        [3]
    );
}

#[test]
fn compound_index_assignment_freezes_getter_and_setter_receivers() {
    let compilation = test_support::compile(
        r#"package main
type Buffer struct { values: [1]u32 }
operator (self: *Buffer) [](index: u32) -> u32 { return self.values[index] }
operator (self: *Buffer) []=(index: u32, value: u32) { self.values[index] = value }
func redirect(pointer: **Buffer, target: *Buffer) -> u32 {
    *pointer = target
    return 1
}
func observe() -> (u32, u32) {
    a := Buffer{values: [1]u32{4}}
    b := Buffer{values: [1]u32{20}}
    pointer := &a
    (&pointer)[0] += redirect(&pointer, &b)
    return a.values[0], b.values[0]
}
func main() {}
"#,
        None,
    )
    .expect("compound index assignment should compile");
    assert_eq!(
        memory_test_vm::execute(&compilation.mir, "main.observe", &[]).unwrap(),
        [5, 20]
    );
}

#[test]
fn compound_index_assignment_resolves_getter_before_index() {
    let compilation = test_support::compile(
        r#"package main
type Buffer struct { values: [1]u32 }
operator (self: *Buffer) [](index: u32) -> u32 { return self.values[index] }
operator (self: *Buffer) []=(index: u32, value: u32) { self.values[index] = value }
func redirect(pointer: **Buffer, target: *Buffer) -> u32 {
    *pointer = target
    return 0
}
func observe() -> (u32, u32) {
    a := Buffer{values: [1]u32{4}}
    b := Buffer{values: [1]u32{20}}
    pointer := &a
    (&pointer)[redirect(&pointer, &b)] += 1
    return a.values[0], b.values[0]
}
func main() {}
"#,
        None,
    )
    .expect("compound index assignment should compile");
    assert_eq!(
        memory_test_vm::execute(&compilation.mir, "main.observe", &[]).unwrap(),
        [5, 20]
    );
}

#[test]
fn compound_index_assignment_dereferences_binary_receiver_before_rhs() {
    let compilation = test_support::compile(
        r#"package main
type N struct { value: u32 }
type Buffer struct { values: [1]N }
var result: N
operator (self: *Buffer) [](index: u32) -> *N { return &self.values[index] }
operator (self: *Buffer) []=(index: u32, value: *N) { self.values[index] = *value }
operator (self: N) + (other: u32) -> *N {
    result.value = self.value + other
    return &result
}
func change(value: *N) -> u32 { value.value = 9; return 2 }
func observe() -> u32 {
    buffer := Buffer{values: [1]N{N{value: 1}}}
    buffer[0] += change(&buffer.values[0])
    return buffer.values[0].value
}
func main() {}
"#,
        None,
    )
    .expect("compound index assignment should compile");
    assert_eq!(
        memory_test_vm::execute(&compilation.mir, "main.observe", &[]).unwrap(),
        [3]
    );
}

#[test]
fn unary_binary_and_len_operators_execute() {
    test_support::compile(
        r#"package main

type Number struct { value: i32 }

operator (lhs: Number) + (rhs: Number) -> Number {
    return Number{value: lhs.value + rhs.value}
}

operator (value: Number) - () -> Number {
    return Number{value: 0 - value.value}
}

operator (value: Number) len() -> u32 {
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

operator (buffer: *Buffer) [](index: u32) -> *u32 {
    return &buffer.values[index]
}

operator (buffer: *Buffer) []=(index: u32, value: u32) {
    buffer.values[index] = value
}

func observe() -> u32 {
    buffer := Buffer{values: [2]u32{4, 9}}
    buffer[1] = 12
    return *buffer[1]
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

operator (lhs: Number) + (rhs: u32) -> Number {
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

operator (buffer: *Buffer) [](index: u32) -> u32 {
    return buffer.values[index]
}

operator (buffer: *Buffer) []=(index: u32, value: u32) {
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
import "test/shared"

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

operator (values: *Vec[T]) [](index: u32) -> *T {
    return &values.values[index]
}

operator (values: *Vec[T]) []=(index: u32, value: T) {
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

operator (values: List[T]) [](index: u32) -> T {
    return values.Get(index)
}

operator (values: List[T]) []=(index: u32, value: T) {
    values.Set(index, value)
}

operator (values: List[T]) len() -> u32 {
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

operator (lhs: Number[T]) + (rhs: Number[T]) -> Number[T] {
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
fn concrete_generic_receiver_is_rejected() {
    let error = test_support::compile(
        r#"package main

type Number[T] struct { value: T }

operator (lhs: Number[T]) + (rhs: Number[T]) -> Number[T] {
    return lhs
}

operator (lhs: Number[u32]) + (rhs: Number[u32]) -> Number[u32] {
    return rhs
}

func observe() -> Number[u32] {
    return Number[u32]{value: 1} + Number[u32]{value: 2}
}

func main() {}
"#,
        None,
    )
    .expect_err("a concrete generic receiver must be rejected");

    assert!(
        error
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.message.contains("fresh, distinct"))
    );
}

#[test]
fn pointer_and_value_operator_receivers_are_duplicates() {
    let error = test_support::compile(
        r#"package main
type Number struct { value: u32 }
operator (self: Number) + (other: Number) -> Number { return self }
operator (self: *Number) + (other: Number) -> Number { return other }
func main() {}
"#,
        None,
    )
    .expect_err("pointer and value receivers share an operator slot");
    assert!(error.diagnostics().iter().any(|diagnostic| {
        diagnostic
            .message
            .contains("duplicate operator declaration")
    }));
}

#[test]
fn unary_and_binary_forms_share_a_receiver_without_conflicting() {
    test_support::compile(
        r#"package main
type Number struct { value: u32 }
operator (self: Number) + () -> Number { return self }
operator (self: Number) + (other: Number) -> Number { return self }
func main() { value := Number{value: 1}; _ = +value; _ = value + value }
"#,
        None,
    )
    .expect("arity distinguishes unary and binary operator forms");
}

#[test]
fn operator_self_uses_one_implicit_address_or_dereference() {
    let compilation = test_support::compile(
        r#"package main
type Number struct { value: u32 }
operator (self: *Number) + (other: u32) -> u32 { return self.value + other }
operator (self: Number) * (other: u32) -> u32 { return self.value + other }
func observe() -> (u32, u32) {
    value := Number{value: 7}
    return value + 3, (&value) * 2
}
func main() {}
"#,
        None,
    )
    .expect("operator self should auto-address and auto-dereference");
    assert_eq!(
        memory_test_vm::execute(&compilation.mir, "main.observe", &[]).unwrap(),
        [10, 9]
    );
}

#[test]
fn operator_auto_address_requires_addressable_self() {
    let error = test_support::compile(
        r#"package main
type Number struct { value: u32 }
operator (self: *Number) + (other: u32) -> u32 { return self.value + other }
func make() -> Number { return Number{value: 7} }
func main() { _ = make() + 3 }
"#,
        None,
    )
    .expect_err("temporary receiver cannot be auto-addressed");
    assert!(
        error
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.message.contains("operand")),
        "{:?}",
        error.diagnostics()
    );
}

#[test]
fn other_operator_operands_are_exact() {
    let error = test_support::compile(
        r#"package main
type Number struct { value: u32 }
operator (self: Number) + (other: Number) -> Number { return self }
func main() {
    left := Number{value: 1}
    right := Number{value: 2}
    _ = left + &right
}
"#,
        None,
    )
    .expect_err("other operands cannot be auto-dereferenced");
    assert!(
        error
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.message.contains("assignment type mismatch")),
        "{:?}",
        error.diagnostics()
    );
}

#[test]
fn imported_generic_operators_are_specialized_in_their_owner_package() {
    let compilation = test_support::compile(
        r#"package main
import "test/shared"

func observe() -> u32 {
    values := shared.Values[u32]{Data: [1]u32{37}}
    return *(&values)[0]
}

func main() {}
"#,
        Some(
            r#"package shared

type Values[T] struct { Data: [1]T }

operator (values: *Values[T]) [](index: u32) -> *T {
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
fn generic_pointer_operators_are_found_through_a_bridge_import() {
    let compilation = test_support::compile_files(&[
        (
            "main.ond",
            r#"package main
import facade "test/bridge"

func observe() -> u32 {
    values := facade.Make[u32]()
    return *(&values)[0]
}

func main() {}
"#,
        ),
        (
            "bridge/bridge.ond",
            r#"package bridge
import ownerpkg "test/owner"

func Make[T]() -> ownerpkg.Values[T] {
    return ownerpkg.Values[T]{Data: [1]T{37}}
}
"#,
        ),
        (
            "owner/owner.ond",
            r#"package owner

type Values[T] struct { Data: [1]T }

operator (values: *Values[T]) [](index: u32) -> *T {
    return &values.Data[index]
}
"#,
        ),
    ])
    .expect("generic pointer operators should be found in the receiver owner package");

    assert_eq!(
        memory_test_vm::execute(&compilation.mir, "main.observe", &[]).unwrap(),
        [37]
    );
}

#[test]
fn operator_receiver_must_be_local_and_use_declared_type_parameters() {
    for (source, expected) in [
        (
            r#"package main
type Values[T] struct { value: T }
operator (values: *Values[T, U]) [](index: u32) -> U { trap "not implemented" }
func main() {}
"#,
            "expects 1 type parameters, found 2",
        ),
        (
            r#"package main
operator (lhs: T) + (rhs: T) -> T { return lhs }
func main() {}
"#,
            "operator receiver type must be declared in the same package",
        ),
        (
            r#"package main
type Pair[T, U] struct { left: T; right: U }
operator (self: Pair[T, T]) + (other: Pair[T, T]) -> Pair[T, T] { return self }
func main() {}
"#,
            "fresh, distinct",
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
import "test/shared"

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

operator (values: List[T]) [](index: u32) -> T {
    return values.Get(index)
}
"#,
        ),
    )
    .expect("an imported generic interface operator should be available by operand type");
}

#[test]
fn generic_interface_operators_are_found_through_a_bridge_import() {
    test_support::compile_files(&[
        (
            "main.ond",
            r#"package main
import facade "test/bridge"

func main() {
    values := facade.Make()
    _ = values[0]
}
"#,
        ),
        (
            "bridge/bridge.ond",
            r#"package bridge
import ownerpkg "test/owner"

type Pair struct { Values: [1]u32 }
var pair = Pair{Values: [1]u32{37}}
func (pair: *Pair) Get(index: u32) -> u32 { return pair.Values[index] }

func Make() -> ownerpkg.List[u32] {
    return (&pair) as ownerpkg.List[u32]
}
"#,
        ),
        (
            "owner/owner.ond",
            r#"package owner

type List[T] interface { Get(index: u32) -> T; }

operator (values: List[T]) [](index: u32) -> T {
    return values.Get(index)
}
"#,
        ),
    ])
    .expect("generic interface operators should be found in the receiver owner package");
}

#[test]
fn interface_operators_do_not_apply_to_implementing_concrete_types_implicitly() {
    let error = test_support::compile(
        r#"package main

type List interface { Get(index: u32) -> u32; }
type Pair struct { values: [1]u32 }
func (pair: *Pair) Get(index: u32) -> u32 { return pair.values[index] }
operator (values: List) + (index: u32) -> u32 { return values.Get(index) }

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

#[test]
fn compound_assignment_passes_the_resolved_pointer_receiver() {
    let compilation = test_support::compile(
        r#"package main
type N struct { value: u32 }
operator (self: *N) + (other: u32) -> N {
    return N{value: self.value + other}
}
func observe() -> u32 {
    value := N{value: 1}
    value += 2
    return value.value
}
func main() {}
"#,
        None,
    )
    .expect("compound assignment should preserve auto-addressed self");
    assert_eq!(
        memory_test_vm::execute(&compilation.mir, "main.observe", &[]).unwrap(),
        [3]
    );
}

#[test]
fn compound_index_assignment_reuses_the_base_with_resolved_receivers() {
    let compilation = test_support::compile(
        r#"package main
type Buffer struct { values: [1]u32 }
operator (self: *Buffer) [](index: u32) -> u32 { return self.values[index] }
operator (self: *Buffer) []=(index: u32, value: u32) { self.values[index] = value }
func observe() -> u32 {
    buffer := Buffer{values: [1]u32{4}}
    buffer[0] += 1
    return buffer.values[0]
}
func main() {}
"#,
        None,
    )
    .expect("compound index assignment should auto-address getter and setter self");
    assert_eq!(
        memory_test_vm::execute(&compilation.mir, "main.observe", &[]).unwrap(),
        [5]
    );
}

#[test]
fn double_pointer_operand_finds_pointer_receiver_in_one_step() {
    let compilation = test_support::compile(
        r#"package main
type N struct { value: u32 }
operator (self: *N) + (other: u32) -> u32 { return self.value + other }
func observe() -> u32 {
    value := N{value: 1}
    pointer := &value
    return (&pointer) + 2
}
func main() {}
"#,
        None,
    )
    .expect("**N should discover *N operator receiver");
    assert_eq!(
        memory_test_vm::execute(&compilation.mir, "main.observe", &[]).unwrap(),
        [3]
    );
}

#[test]
fn generic_double_pointer_operand_finds_pointer_receiver_in_one_step() {
    let compilation = test_support::compile(
        r#"package main
type N[T] struct { value: T }
operator (self: *N[T]) + (other: T) -> T { return self.value + other }
func observe() -> u32 {
    value := N[u32]{value: 1}
    pointer := &value
    return (&pointer) + 2
}
func main() {}
"#,
        None,
    )
    .expect("**N[T] should discover *N[T] operator receiver");
    assert_eq!(
        memory_test_vm::execute(&compilation.mir, "main.observe", &[]).unwrap(),
        [3]
    );
}

#[test]
fn compound_index_assignment_evaluates_converted_base_once() {
    let compilation = test_support::compile(
        r#"package main
type Buffer struct { values: [1]u32 }
operator (self: *Buffer) [](index: u32) -> u32 { return self.values[index] }
operator (self: *Buffer) []=(index: u32, value: u32) { self.values[index] = value }
func next(buffer: *Buffer, calls: *u32) -> *Buffer {
    *calls += 1
    return buffer
}
func observe() -> (u32, u32) {
    buffer := Buffer{values: [1]u32{4}}
    var calls: u32 = 0
    (*next(&buffer, &calls))[0] += 1
    return buffer.values[0], calls
}
func main() {}
"#,
        None,
    )
    .expect("converted base should be evaluated once");
    assert_eq!(
        memory_test_vm::execute(&compilation.mir, "main.observe", &[]).unwrap(),
        [5, 1]
    );
}

#[test]
fn operator_receiver_conversion_stops_after_one_dereference() {
    let error = test_support::compile(
        r#"package main
type N struct { value: u32 }
operator (self: *N) + (other: u32) -> u32 { return self.value + other }
func main() {
    value := N{value: 1}
    pointer := &value
    double := &pointer
    var two: u32 = 2
    _ = (&double) + two
}
"#,
        None,
    )
    .expect_err("***N cannot reach *N in one step");
    assert!(
        error
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.message.contains("operand"))
    );
}

#[test]
fn compound_assignment_passes_the_resolved_value_receiver() {
    test_support::compile(
        r#"package main
type N struct { value: u32 }
operator (self: N) + (other: u32) -> *N {
    result := new(N)
    result.value = self.value + other
    return result
}
func main() {
    value := N{value: 1}
    pointer := &value
    pointer += 2
}
"#,
        None,
    )
    .expect("compound assignment should auto-dereference self in MIR");
}

#[test]
fn compound_index_assignment_passes_dereferenced_receivers() {
    let compilation = test_support::compile(
        r#"package main
type Buffer struct { values: [1]u32 }
operator (self: *Buffer) [](index: u32) -> u32 { return self.values[index] }
operator (self: *Buffer) []=(index: u32, value: u32) { self.values[index] = value }
func observe() -> u32 {
    buffer := Buffer{values: [1]u32{4}}
    pointer := &buffer
    (&pointer)[0] += 1
    return buffer.values[0]
}
func main() {}
"#,
        None,
    )
    .expect("compound index getter and setter should auto-dereference self");
    assert_eq!(
        memory_test_vm::execute(&compilation.mir, "main.observe", &[]).unwrap(),
        [5]
    );
}
