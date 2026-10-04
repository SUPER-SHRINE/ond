use crate::{
    semantic::TypeKind,
    test_support::{compile, reject},
};

#[test]
fn pointer_interfaces_lower_conversion_dispatch_nil_and_identity_comparison() {
    let compilation = compile(
        "package main\n\
         type Reader interface { Read(value: i32) -> i32; }\n\
         type Device struct { bias: i32; }\n\
         func (device: *Device) Read(value: i32) -> i32 { return device.bias + value; }\n\
         func consume(reader: Reader) -> i32 { return reader.Read(3); }\n\
         func main() { var device: Device; var reader: Reader = (&device) as Reader; \
         if reader != nil { _ = consume(reader); }; }\n",
        None,
    )
    .unwrap();
    assert!(
        compilation
            .hir
            .types
            .definitions()
            .iter()
            .any(|definition| { matches!(definition.kind, TypeKind::Interface(_)) })
    );
    assert!(
        compilation
            .mir
            .packages
            .iter()
            .flat_map(|package| &package.statics)
            .any(|item| item.symbol.contains("\u{1f}vtable"))
    );
}

#[test]
fn interface_conversion_requires_exact_pointer_receiver_methods() {
    reject(
        "package main\ntype Runner interface { Run(); }\ntype S struct {}\nfunc (s: S) Run() {}\nfunc main() { var s: S; _ = (&s) as Runner; }\n",
        None,
        "does not implement interface method `Run`",
        "&s) as Runner",
    );
    reject(
        "package main\ntype Runner interface { Run(); }\ntype S struct {}\nfunc (s: *S) Run() {}\nfunc main() { var s: S; _ = s as Runner; }\n",
        None,
        "interface conversion requires a pointer value",
        "s as Runner",
    );
}

#[test]
fn interfaces_reject_private_methods_and_methods_on_interfaces() {
    reject(
        "package main\ntype Public interface { hidden(); }\nfunc main() {}\n",
        None,
        "interface methods must be exported",
        "hidden",
    );
    reject(
        "package main\ntype Public interface {}\nfunc (value: *Public) Run() {}\nfunc main() {}\n",
        None,
        "methods cannot be declared on interface types",
        "value: *Public",
    );
}

#[test]
fn package_interface_records_interface_and_concrete_method_metadata() {
    let compilation = compile(
        "package main\n\
         type Reader interface { Read(value: i32) -> i32; }\n\
         type Device struct {}\n\
         func (device: *Device) Read(value: i32) -> i32 { return value; }\n\
         func main() {}\n",
        None,
    )
    .unwrap();
    let path = compilation.hir.packages[0].logical_path.clone();
    let exported = crate::export::package(&compilation.hir, &path).unwrap();
    let device = match exported.exports.get("Device") {
        Some(ond_interface::Export::Type(id)) => &exported.types[*id as usize],
        other => panic!("missing Device export: {other:?}"),
    };
    assert_eq!(device.methods.len(), 1);
    assert_eq!(device.methods[0].name, "Read");
    assert!(device.methods[0].pointer_receiver);
    assert!(
        device.methods[0]
            .symbol
            .contains("\u{1f}method.Device.Read")
    );
}

#[test]
fn empty_and_self_referential_named_interfaces_are_valid() {
    compile(
        "package main\n\
         type Any interface {}\n\
         type Chain interface { Next() -> Chain; }\n\
         type Node struct {}\n\
         func (node: *Node) Next() -> Chain { return node as Chain; }\n\
         func main() { var node: Node; var any = (&node) as Any; _ = any; }\n",
        None,
    )
    .unwrap();
}

#[test]
fn generic_interfaces_substitute_method_signatures_and_dispatch() {
    compile(
        r#"package main

type Box[T] interface {
    Get() -> T
    Set(value: T)
}

type Cell[T] struct { value: T }

func (cell: *Cell[T]) Get() -> T { return cell.value }
func (cell: *Cell[T]) Set(value: T) { cell.value = value }

func observe() -> (u32, i32) {
    unsigned := Cell[u32]{value: 3}
    signed := Cell[i32]{value: -2}
    var left: Box[u32] = (&unsigned) as Box[u32]
    var right: Box[i32] = (&signed) as Box[i32]
    left.Set(11)
    right.Set(-7)
    return left.Get(), right.Get()
}

func main() {}
"#,
        None,
    )
    .expect("generic interface instances should compile");
}

#[test]
fn imported_types_can_implement_exported_generic_interfaces() {
    compile(
        r#"package main
import "shared"

type Values struct { value: u32 }
func (values: *Values) Get(index: u32) -> u32 { return values.value + index }

func main() {
    values := Values{value: 4}
    var list: shared.List[u32] = (&values) as shared.List[u32]
    _ = list.Get(3)
}
"#,
        Some(
            r#"package shared

type List[T] interface { Get(index: u32) -> T; }
"#,
        ),
    )
    .expect("an external type should implement a public generic interface");
}

#[test]
fn public_generic_interfaces_reject_private_methods() {
    reject(
        "package main\ntype Public[T] interface { hidden(value: T); }\nfunc main() {}\n",
        None,
        "interface methods must be exported",
        "hidden",
    );
    reject(
        "package main\ntype private[T] interface { hidden(value: T); }\nfunc main() {}\n",
        None,
        "interface methods must be exported",
        "hidden",
    );
}

#[test]
fn generic_interfaces_allow_indirect_self_references() {
    compile(
        r#"package main

type Chain[T] interface { Next() -> Chain[T]; Value() -> T; }
type Node[T] struct { value: T }
func (node: *Node[T]) Next() -> Chain[T] { return node as Chain[T] }
func (node: *Node[T]) Value() -> T { return node.value }
func main() { var node: Node[u32]; _ = (&node) as Chain[u32] }
"#,
        None,
    )
    .expect("a generic interface may mention its own concrete instance in a method signature");
}

#[test]
fn generic_interface_implementation_requires_the_substituted_signature() {
    reject(
        r#"package main
type Box[T] interface { Get() -> T; }
type Wrong struct {}
func (value: *Wrong) Get() -> i32 { return 0 }
func main() { var value: Wrong; _ = (&value) as Box[u32] }
"#,
        None,
        "does not implement interface method `Get`",
        "&value) as Box[u32]",
    );
}
