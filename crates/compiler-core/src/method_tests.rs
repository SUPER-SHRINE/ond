use crate::{
    ir::hir,
    memory_test_vm,
    test_support::{compile, reject, reject_at},
};

#[test]
fn value_and_pointer_methods_lower_to_direct_calls_with_exact_receivers() {
    let compilation = compile(
        "package main\n\
         type Counter struct { value: i32; }\n\
         func (counter: Counter) Value() -> i32 { return counter.value; }\n\
         func (counter: *Counter) Add(amount: i32) { counter.value += amount; }\n\
         func main() { var counter: Counter; counter.Value(); (&counter).Add(2); }\n",
        None,
    )
    .unwrap();
    let functions = compilation
        .hir
        .packages
        .iter()
        .flat_map(|package| &package.items)
        .filter_map(|item| match item {
            hir::Item::Func(function) => Some(function),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(functions.iter().any(|function| {
        function
            .canonical_name
            .contains("\u{1f}method.Counter.Value")
    }));
    assert!(
        functions
            .iter()
            .any(|function| { function.canonical_name.contains("\u{1f}method.Counter.Add") })
    );
}

#[test]
fn method_receivers_never_gain_implicit_address_or_dereference_conversions() {
    reject(
        "package main\ntype S struct {}\nfunc (s: *S) Run() {}\nfunc main() { var s: S; s.Run(); }\n",
        None,
        "unknown struct field `Run`",
        "s.Run",
    );
    reject(
        "package main\ntype S struct {}\nfunc (s: S) Run() {}\nfunc main() { var s: *S; s.Run(); }\n",
        None,
        "unknown struct field `Run`",
        "s.Run",
    );
}

#[test]
fn methods_reject_duplicate_receiver_names_and_field_conflicts() {
    reject(
        "package main\ntype S struct {}\nfunc (s: S) Run() {}\nfunc (s: *S) Run() {}\nfunc main() {}\n",
        None,
        "duplicate method `S.Run`",
        "Run",
    );
    reject(
        "package main\ntype S struct { Run: i32; }\nfunc (s: *S) Run() {}\nfunc main() {}\n",
        None,
        "conflicts with a field",
        "Run",
    );
}

#[test]
fn methods_on_named_scalars_coexist_with_functions_but_are_not_values() {
    compile(
        "package main\ntype Count i32\nfunc (value: Count) Next() -> Count { return value + 1; }\nfunc Next(value: Count) -> Count { return value + 2; }\nfunc main() { var value: Count = 1 as Count; _ = value.Next(); _ = Next(value); }\n",
        None,
    )
    .unwrap();
    reject(
        "package main\ntype S struct {}\nfunc (value: *S) Run() {}\nfunc main() { var value: *S; _ = value.Run; }\n",
        None,
        "unknown struct field `Run`",
        "value.Run",
    );
}

#[test]
fn generic_type_methods_infer_receiver_arguments_and_monomorphize() {
    let compilation = compile(
        "package main\n\
         type Cell[T] struct { value: T; }\n\
         func (cell: *Cell[Element]) Set(value: Element) { cell.value = value; }\n\
         func (cell: Cell[Element]) Get() -> Element { return cell.value; }\n\
         func observe() -> u32 { var cell = Cell[u32]{value: 3}; (&cell).Set(9); return cell.Get(); }\n\
         func main() {}\n",
        None,
    )
    .unwrap();

    assert_eq!(
        memory_test_vm::execute(&compilation.mir, "main.observe", &[]).unwrap(),
        [9]
    );
    let generated = compilation
        .hir
        .packages
        .iter()
        .flat_map(|package| &package.items)
        .filter_map(|item| match item {
            hir::Item::Func(function) => Some(&function.canonical_name),
            _ => None,
        })
        .filter(|name| name.contains("\u{1f}generic_method.Cell."))
        .count();
    assert_eq!(generated, 2);
}

#[test]
fn instantiated_generic_methods_satisfy_interfaces() {
    let compilation = compile(
        "package main\n\
         type Reader interface { Read() -> u32; }\n\
         type Cell[T] struct { value: T; }\n\
         func (cell: *Cell[Element]) Read() -> Element { return cell.value; }\n\
         func consume(reader: Reader) -> u32 { return reader.Read(); }\n\
         func observe() -> u32 { var cell = Cell[u32]{value: 7}; return consume((&cell) as Reader); }\n\
         func main() {}\n",
        None,
    )
    .unwrap();

    assert!(compilation.hir.packages.iter().any(|package| {
        package.items.iter().any(|item| {
            matches!(item, hir::Item::Func(function) if function.canonical_name.contains("\u{1f}generic_method.Cell.Read"))
        })
    }));
}

#[test]
fn imported_generic_methods_are_specialized_in_their_owner_package() {
    let compilation = compile(
        "package main\nimport \"shared\"\nfunc observe() -> u32 { var cell = shared.Cell[u32]{Value: 11}; return cell.Get(); }\nfunc main() {}\n",
        Some(
            "package shared\ntype Cell[T] struct { Value: T; }\nfunc (cell: Cell[Element]) Get() -> Element { return cell.Value; }\n",
        ),
    )
    .unwrap();

    assert_eq!(
        memory_test_vm::execute(&compilation.mir, "main.observe", &[]).unwrap(),
        [11]
    );
    assert!(compilation.hir.packages.iter().any(|package| {
        package.logical_path == "shared"
            && package.items.iter().any(|item| {
                matches!(item, hir::Item::Func(function) if function.canonical_name.contains("shared.\u{1f}generic_method.Cell.Get"))
            })
    }));
}

#[test]
fn generic_method_receivers_reject_specialization_extra_parameters_and_duplicates() {
    reject_at(
        "package main\ntype Cell[T] struct { value: T; }\nfunc (cell: *Cell[u32]) Set(value: u32) {}\nfunc main() {}\n",
        None,
        "must be fresh",
        "u32",
        0,
    );
    reject(
        "package main\ntype Pair[A, B] struct {}\nfunc (pair: Pair[T, T]) Read() {}\nfunc main() {}\n",
        None,
        "must be fresh",
        "T",
    );
    reject(
        "package main\ntype Cell[T] struct {}\nfunc (cell: Cell[T]) Map[U]() {}\nfunc main() {}\n",
        None,
        "methods cannot declare their own type parameters",
        "Map",
    );
}
