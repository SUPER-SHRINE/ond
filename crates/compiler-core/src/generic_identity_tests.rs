//! Generic names must preserve source type identity across allocation histories.
use crate::{Compilation, ir::hir, semantic::TypeId, test_support::compile_files};
use std::collections::BTreeMap;

fn returns(compilation: &Compilation) -> BTreeMap<String, TypeId> {
    compilation
        .hir
        .packages
        .iter()
        .flat_map(|package| &package.items)
        .filter_map(|item| match item {
            hir::Item::Func(f) if f.name.starts_with('F') => {
                Some((f.name.clone(), f.signature.returns[0]))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn generic_identity_is_stable_and_preserves_type_distinctions() {
    let shapes = [
        "u32",
        "i32",
        "A",
        "B",
        "*Node",
        "[2]A",
        "[3]A",
        "[2]B",
        "struct { X:u32; Y:*Node }",
        "struct { X:u32; Y:*Node }",
        "struct { Y:*Node; X:u32 }",
        "struct { Z:u32; Y:*Node }",
        "struct { X:i32; Y:*Node }",
        "struct { X:u32 }",
        "struct {}",
        "func(first:A)->(*Node,u32)",
        "func(second:A)->(*Node,u32)",
        "func(first:B)->(*Node,u32)",
        "func(first:A)->(u32,*Node)",
        "Box[A]",
        "Box[B]",
        "Pair[A,B]",
        "Pair[B,A]",
        "Reader[A]",
        "Reader[B]",
        "*A",
        "*B",
        "struct { x:u32 }",
        "struct { x:u32 }",
        "bool",
        "u8",
        "i8",
        "u16",
        "i16",
        "f32",
        "func()",
        "func(u32)",
        "func()->u32",
    ];
    let mut source = String::from(
        "package shared\ntype A u32\ntype B u32\ntype Node struct { Next:*Node; Wrapped:*Box[*Node] }\ntype Box[T] struct { Value:T }\ntype Pair[T,U] struct { Left:T; Right:U }\ntype Reader[T] struct { Value:T; Next:*Reader[T] }\n",
    );
    for (index, shape) in shapes.iter().enumerate() {
        source.push_str(&format!(
            "func F{index}()->Box[{shape}] {{ var value:Box[{shape}]; return value }}\n"
        ));
    }
    let main = "package main\nimport \"test/shared\"\nfunc main() { _=shared.F0() }";
    let before = compile_files(&[("main.ond", main), ("shared/shared.ond", &source)]).unwrap();
    let after = compile_files(&[
        ("main.ond", main),
        ("shared/shared.ond", &format!("\n\n{source}")),
        (
            "aaa/extra.ond",
            "package aaa\ntype Earlier struct { Value:u32 }",
        ),
    ])
    .unwrap();
    assert_ne!(
        before.hir.types.find_named("test/shared.Node"),
        after.hir.types.find_named("test/shared.Node")
    );
    let before_returns = returns(&before);
    let after_returns = returns(&after);
    for (name, id) in &before_returns {
        assert_eq!(
            before.hir.types.get(*id).unwrap().name,
            after.hir.types.get(after_returns[name]).unwrap().name,
            "{name}"
        );
    }
    for left in 0..shapes.len() {
        for right in 0..shapes.len() {
            let equivalent = left == right
                || matches!(
                    (left.min(right), left.max(right)),
                    (8, 9) | (15, 16) | (27, 28)
                );
            let left_id = before_returns[&format!("F{left}")];
            let right_id = before_returns[&format!("F{right}")];
            assert_eq!(
                left_id == right_id,
                equivalent,
                "{} versus {}",
                shapes[left],
                shapes[right]
            );
        }
    }
}

#[test]
fn generic_anonymous_arguments_keep_private_owner_logical_paths() {
    let main = r#"package main
import "test/shared"
func FLocal()->shared.Box[struct { x:u32 }] { var value:shared.Box[struct { x:u32 }]; return value }
func FPublic()->shared.Box[struct { X:u32 }] { var value:shared.Box[struct { X:u32 }]; return value }
func main() {}
"#;
    let shared = r#"package shared
type Box[T] struct { Value:T }
func FRemote()->Box[struct { x:u32 }] { var value:Box[struct { x:u32 }]; return value }
func FExported()->Box[struct { X:u32 }] { var value:Box[struct { X:u32 }]; return value }
"#;
    let before = compile_files(&[("main.ond", main), ("shared/shared.ond", shared)]).unwrap();
    let after = compile_files(&[
        ("main.ond", main),
        ("shared/shared.ond", shared),
        ("aaa/extra.ond", "package aaa\ntype Shift u32"),
    ])
    .unwrap();
    let before_returns = returns(&before);
    assert_ne!(before_returns["FLocal"], before_returns["FRemote"]);
    assert_eq!(before_returns["FPublic"], before_returns["FExported"]);
    for (name, id) in before_returns {
        assert_eq!(
            before.hir.types.get(id).unwrap().name,
            after.hir.types.get(returns(&after)[&name]).unwrap().name
        );
    }
}

#[test]
fn specialization_symbols_ignore_unrelated_declaration_order_and_type_ids() {
    let main = r#"package main
import "test/shared"
func main() {
    cell:=shared.Cell[shared.Token]{}
    _=cell.Get()
    _=len(cell)
    _=shared.Plain{}+cell
}
"#;
    let declarations = r#"type Token struct { Value:u32 }
type Plain struct {}
type Cell[T] struct { Value:T }
func (cell:Cell[T]) Get()->T { return cell.Value }
operator (cell:Cell[T]) len()->u32 { return 1 }
operator (plain:Plain) + (cell:Cell[Token])->u32 { return cell.Value.Value }
"#;
    let before_source = format!("package shared\n{declarations}");
    // Both local declaration indices change, independently of project type IDs.
    let after_source = format!(
        "package shared\nfunc (plain:Plain) Other() {{}}\noperator (plain:Plain) len()->u32 {{ return 0 }}\n{declarations}"
    );
    let before =
        compile_files(&[("main.ond", main), ("shared/shared.ond", &before_source)]).unwrap();
    let after = compile_files(&[
        ("main.ond", main),
        ("shared/shared.ond", &after_source),
        ("aaa/extra.ond", "package aaa\ntype Shift u32"),
    ])
    .unwrap();
    let symbols = |compilation: &Compilation| {
        compilation
            .hir
            .packages
            .iter()
            .flat_map(|p| &p.items)
            .filter_map(|item| match item {
                hir::Item::Func(f)
                    if f.canonical_name.contains(".\u{1f}generic_method.")
                        || f.canonical_name.contains(".\u{1f}generic_operator.")
                        || f.canonical_name.contains(".\u{1f}operator.add.") =>
                {
                    Some(f.canonical_name.clone())
                }
                _ => None,
            })
            .collect::<std::collections::BTreeSet<_>>()
    };
    assert_eq!(symbols(&before).len(), 3);
    assert_eq!(symbols(&before), symbols(&after));
}
