//! Generationのsource境界を固定JSONで確認する。
use super::export_tests::Fixture;
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs};

fn generations(label: &str, source: &str, library: Option<&str>) -> BTreeMap<String, Value> {
    let f = Fixture::new(label);
    fs::write(f.0.join("ond.toml"), "[project]\nname = \"main\"\n").unwrap();
    fs::write(f.0.join("main.ond"), source).unwrap();
    if let Some(library) = library {
        fs::create_dir(f.0.join("lib")).unwrap();
        fs::write(f.0.join("lib/lib.ond"), library).unwrap();
    }
    let loaded = crate::input::load(&f.request()).unwrap();
    let (_, generation) =
        ond_compiler_core::compile_loaded_project_with_generation(loaded).unwrap();
    generation
        .packages
        .into_iter()
        .map(|(path, json)| (path, serde_json::from_str(&json).unwrap()))
        .collect()
}
fn generation(label: &str, declaration: &str) -> Value {
    generations(
        label,
        &format!("package main\n{declaration}\nfunc main(){{}}\n"),
        None,
    )
    .remove("main")
    .unwrap()
}
fn owner(variant: &str, name: &str) -> Value {
    json!({"declaration":{variant:{"package":"main","name":name}},"receiver_arguments":[],"type_arguments":[]})
}
fn value(ty: Value, kind: Value) -> Value {
    json!({"ty":ty,"kind":kind})
}
fn integer(ty: Value, bits: u64) -> Value {
    value(
        ty,
        if bits == 0 {
            json!("Zero")
        } else {
            json!({"Integer":bits})
        },
    )
}
fn array(length: u32, element: Value) -> Value {
    json!({"Array":{"length":length,"element":element}})
}
fn composite(ty: Value, children: Vec<Value>) -> Value {
    value(
        ty,
        json!({"Composite":children.into_iter().enumerate().map(|(index,child)|json!([index,child])).collect::<Vec<_>>()}),
    )
}
fn assert_constants(generation: &Value, owner: Value, values: Vec<Value>) {
    let mut expected = values
        .into_iter()
        .map(|value| json!({"owner":owner,"value":value}))
        .collect::<Vec<_>>();
    expected.sort_by_key(Value::to_string);
    assert_eq!(generation["constants"], json!(expected));
}
#[test]
fn maximal_and_duplicate_constants() {
    let g = generation(
        "generation-maximal",
        "func value(x:i32)->i32{return x+((1+2)*3)}",
    );
    assert_constants(
        &g,
        owner("Function", "value"),
        vec![integer(json!("I32"), 9)],
    );
    let g = generation(
        "generation-duplicate",
        "func value(x:i32)->i32{if x>0{return x+1};return x+1}",
    );
    assert_constants(
        &g,
        owner("Function", "value"),
        vec![integer(json!("I32"), 0), integer(json!("I32"), 1)],
    );
}

#[test]
fn source_information_calls_add_only_source_types() {
    let g = generation(
        "generation-source-information",
        "func value(){_=thisFile();_=thisLine()}",
    );
    assert_eq!(g["constants"], json!([]));
    assert!(
        !g["types"]
            .as_array()
            .unwrap()
            .iter()
            .any(|ty| ty.get("Array").is_some()),
        "{g}"
    );
    assert!(g["types"].as_array().unwrap().contains(&json!("U32")));
    let g=generations("generation-source-information-contexts","package main\nfunc value[T](x:T)->T{_=thisLine();return x}\nfunc emit(line:u32){}\nfunc main(){_=false&&(thisLine()==0);defer {emit(thisLine())};_=value[u8](1)}\n",None).remove("main").unwrap();
    assert!(
        g["constants"]
            .as_array()
            .unwrap()
            .iter()
            .all(|constant| constant["owner"]["declaration"]["Function"]["name"] == "main"),
        "{g}"
    );
    assert!(g["types"].as_array().unwrap().contains(&json!("U32")));
}
#[test]
fn short_circuit_preserves_types_and_declaration_values() {
    let g = generation(
        "generation-short",
        "const Flag=false && (len([1]u8{256 as u8})==0)",
    );
    assert_constants(
        &g,
        owner("Constant", "Flag"),
        vec![value(json!("Bool"), json!("Zero"))],
    );
    for ty in [
        json!("U8"),
        json!("U32"),
        json!("Bool"),
        array(1, json!("U8")),
    ] {
        assert!(g["types"].as_array().unwrap().contains(&ty), "{ty}: {g}");
    }
    let g = generations(
        "generation-skipped-reference",
        "package main\nconst N=7\nfunc main(){_=false&&(N==0)}\n",
        None,
    )
    .remove("main")
    .unwrap();
    assert!(
        g["constants"]
            .as_array()
            .unwrap()
            .contains(&json!({"owner":owner("Constant","N"),"value":integer(json!("I32"),7)})),
        "{g}"
    );
}
#[test]
fn type_length_contributes_only_to_type_shape() {
    let g = generation("generation-length", "type A [1+2]u8");
    assert_eq!(g["constants"], json!([]));
    assert_eq!(g["instances"], json!([]));
    let identity = json!({"package":"main","name":"A","arguments":[]});
    let mut types = vec![
        json!({"Function":{"parameters":[],"returns":[]}}),
        json!({"Defined":identity}),
        array(3, json!("U8")),
        json!("U8"),
    ];
    types.sort_by_key(Value::to_string);
    assert_eq!(g["types"], json!(types));
    assert_eq!(
        g["nominals"],
        json!([{"identity":identity,"underlying":array(3,json!("U8")),"methods":[]}])
    );
}

#[test]
fn intrinsic_call_results_are_source_types_in_all_value_contexts() {
    let mut expected = vec![
        json!("U8"),
        json!("U32"),
        json!({"Pointer":"U8"}),
        json!({"Function":{"parameters":[],"returns":[]}}),
    ];
    expected.sort_by_key(Value::to_string);
    for (label, body) in [
        ("discard", "_=alloc[u8](1)"),
        ("statement", "alloc[u8](1)"),
        ("declaration", "var p=alloc[u8](1);_=p"),
    ] {
        let g = generations(
            &format!("generation-alloc-{label}"),
            &format!("package main\nfunc main(){{{body}}}\n"),
            None,
        )
        .remove("main")
        .unwrap();
        assert_eq!(g["types"], json!(expected), "{body}");
    }
}

#[test]
fn root_generic_nominals_use_manifest_identity_in_generation_and_interface() {
    let f = Fixture::new("generation-root-generic");
    fs::write(f.0.join("ond.toml"), "[project]\nname = \"p\"\n").unwrap();
    fs::write(f.0.join("main.ond"),"package main\ntype Box[T] struct{Value:T}\ntype Public struct{Value:Box[u8]}\nfunc main(){var b:Box[u8];_=b}\n").unwrap();
    let (compilation, generation) = ond_compiler_core::compile_loaded_project_with_generation(
        crate::input::load(&f.request()).unwrap(),
    )
    .unwrap();
    let g: Value = serde_json::from_str(&generation.packages["p"]).unwrap();
    let boxed = json!({"package":"p","name":"Box","arguments":["U8"]});
    let public = json!({"package":"p","name":"Public","arguments":[]});
    let mut nominals = vec![
        json!({"identity":boxed,"underlying":{"Struct":[{"name":"Value","ty":"U8","private_owner":null}]},"methods":[]}),
        json!({"identity":public,"underlying":{"Struct":[{"name":"Value","ty":{"Defined":boxed},"private_owner":null}]},"methods":[]}),
    ];
    nominals.sort_by_key(|value| value["identity"].to_string());
    assert_eq!(g["nominals"], json!(nominals));
    assert_eq!(
        g["interface"],
        json!({"package":"p","exports":{"Public":{"Type":{"Defined":public}}}})
    );
    let interface = ond_compiler_core::export::package(&compilation.hir, "p").unwrap();
    assert!(
        interface
            .types
            .iter()
            .any(|ty| ty.name.as_deref() == Some("p.Box[\"U8\"]")),
        "{interface:?}"
    );
}

#[test]
fn root_normalization_preserves_a_foreign_named_generic_owner() {
    let f = Fixture::new("generation-foreign-main-generic");
    fs::write(f.0.join("ond.toml"), "[project]\nname = \"p\"\n").unwrap();
    fs::write(f.0.join("main.ond"),"package main\nimport other \"foreign\"\ntype Public struct{Value:other.Box[u8]}\nfunc main(){}\n").unwrap();
    let request = f.request();
    crate::dependency_test_support::source(
        &f.0,
        "foreign/box.ond",
        "package foreign\ntype Box[T] struct{Value:T}\n",
    );
    let (compilation, generation) = ond_compiler_core::compile_loaded_project_with_generation(
        crate::input::load(&request).unwrap(),
    )
    .unwrap();
    let g: Value = serde_json::from_str(&generation.packages["p"]).unwrap();
    assert!(
        g["nominals"]
            .as_array()
            .unwrap()
            .iter()
            .any(|nominal| nominal["identity"]
                == json!({"package":"foreign","name":"Box","arguments":["U8"]})),
        "{g}"
    );
    let interface = ond_compiler_core::export::package(&compilation.hir, "p").unwrap();
    assert!(
        interface
            .types
            .iter()
            .any(|ty| ty.name.as_deref() == Some("foreign.Box[\"U8\"]")),
        "{interface:?}"
    );
}
#[test]
fn nested_static_and_strings_include_semantic_aggregates() {
    let inner = array(1, json!("U8"));
    let a = composite(inner.clone(), vec![integer(json!("U8"), 3)]);
    let b = composite(inner.clone(), vec![integer(json!("U8"), 4)]);
    let outer = composite(array(2, inner), vec![a.clone(), b.clone()]);
    let g = generation("generation-grid", "const Grid=[2][1]u8{[1]u8{3},[1]u8{4}}");
    assert_constants(&g, owner("Constant", "Grid"), vec![outer, a, b]);
    let inner = array(2, json!("U8"));
    let bytes = composite(
        inner.clone(),
        vec![integer(json!("U8"), 97), integer(json!("U8"), 98)],
    );
    let outer = composite(array(1, inner), vec![bytes.clone()]);
    let g = generation("generation-text", "const Text=[1][2]u8{\"ab\"}");
    assert_constants(
        &g,
        owner("Constant", "Text"),
        vec![outer.clone(), bytes.clone()],
    );
    let g = generation(
        "generation-runtime-text",
        "func value(){var t=[1][2]u8{\"ab\"};_=t}",
    );
    assert_constants(&g, owner("Function", "value"), vec![outer, bytes]);
}

#[test]
fn static_storage_retains_constant_scalar_parents() {
    let bytes = composite(
        array(2, json!("U8")),
        vec![integer(json!("U8"), 97), integer(json!("U8"), 98)],
    );
    let g = generation(
        "generation-static-len",
        "func value()->u32{return len(\"ab\")}",
    );
    assert_constants(
        &g,
        owner("Function", "value"),
        vec![integer(json!("U32"), 2), bytes.clone()],
    );
    let g = generation(
        "generation-static-index",
        "func value()->u8{return \"ab\"[0]}",
    );
    assert_constants(
        &g,
        owner("Function", "value"),
        vec![integer(json!("U8"), 97), bytes],
    );
    let g = generation(
        "generation-named-index",
        "const A=[1]u8{3}\nfunc value()->u8{return A[0]}",
    );
    assert!(
        g["constants"]
            .as_array()
            .unwrap()
            .contains(&json!({"owner":owner("Function","value"),"value":integer(json!("U8"),3)})),
        "{g}"
    );
}
#[test]
fn imported_methods_close_types_and_adjusted_receiver_types() {
    let g=generations("generation-method-types","package main\nimport \"main/lib\"\nfunc main(){var b:lib.Box;_=b;b.Set(1);lib.Identity[u8](1)}\n",Some("package lib\ntype hidden struct{}\ntype Box struct{}\nfunc(b:Box) Method(x:hidden)->hidden{return x}\nfunc(b:*Box) Set(v:u8){}\nfunc Identity[T](x:T)->T{return x}\n")).remove("main").unwrap();
    let hidden = json!({"package":"main/lib","name":"hidden","arguments":[]});
    let boxed = json!({"Defined":{"package":"main/lib","name":"Box","arguments":[]}});
    assert!(
        g["nominals"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["identity"] == hidden),
        "{g}"
    );
    for ty in [
        json!({"Pointer":boxed}),
        json!({"Function":{"parameters":["U8"],"returns":["U8"]}}),
    ] {
        assert!(g["types"].as_array().unwrap().contains(&ty), "{ty}: {g}");
    }
}

#[test]
fn generic_instances_and_constant_owners_use_source_identity() {
    let generations = generations(
        "generation-instances",
        r#"package main
import "main/lib"
func observe()->u32{var b:lib.Box[u8];var f:func(u8)->u8=lib.Identity[u8];return (lib.Identity[u8](1) as u32)+(f(2) as u32)+lib.Identity[u32](4)+(b.Get() as u32)+((b+3) as u32)}
func main(){}
"#,
        Some(
            r#"package lib
type Box[T] struct{Value:T}
type Holder struct{Small:Box[u8];Nested:Box[Box[u8]]}
func Identity[T](x:T)->T{return x}
func Unused[T](x:T)->T{return x}
func (b:Box[T]) Get()->T{_=7;return b.Value}
operator (b:Box[T]) +(rhs:T)->T{_=8;return b.Value+rhs}
"#,
        ),
    );
    let g = &generations["main/lib"];
    let function = |ty: &str| json!({"declaration":{"Function":{"name":"Identity","package":"main/lib"}},"receiver_arguments":[],"type_arguments":[ty]});
    let method = json!({"declaration":{"Method":{"package":"main/lib","receiver":"Box","name":"Get","pointer_receiver":false}},"receiver_arguments":["U8"],"type_arguments":[]});
    let operator = json!({"declaration":{"Operator":{"package":"main/lib","receiver":"Box","token":"+","operand_count":2,"pointer_receiver":false}},"receiver_arguments":["U8"],"type_arguments":[]});
    let mut expected = vec![
        json!({"identity":function("U8"),"signature":{"parameters":["U8"],"returns":["U8"]}}),
        json!({"identity":function("U32"),"signature":{"parameters":["U32"],"returns":["U32"]}}),
        json!({"identity":method,"signature":{"parameters":[],"returns":["U8"]}}),
        json!({"identity":operator,"signature":{"parameters":["U8"],"returns":["U8"]}}),
    ];
    expected.sort_by_key(Value::to_string);
    assert_eq!(g["instances"], json!(expected));
    assert_eq!(generations["main"]["instances"], json!([]));
    let mut constants = vec![
        json!({"owner":method,"value":integer(json!("I32"),7)}),
        json!({"owner":operator,"value":integer(json!("I32"),8)}),
    ];
    constants.sort_by_key(Value::to_string);
    assert_eq!(g["constants"], json!(constants));
    assert_eq!(
        g["interface"]["exports"],
        json!({"Holder":{"Type":{"Defined":{"package":"main/lib","name":"Holder","arguments":[]}}}})
    );
    let boxed = g["nominals"]
        .as_array()
        .unwrap()
        .iter()
        .find(|nominal| {
            nominal["identity"] == json!({"package":"main/lib","name":"Box","arguments":["U8"]})
        })
        .unwrap();
    assert_eq!(
        boxed["methods"],
        json!([{"name":"Get","identity":method,"pointer_receiver":false,"signature":{"parameters":[],"returns":["U8"]}}])
    );
}

#[test]
fn address_operands_keep_maximal_static_member_values() {
    for (label, source, extra) in [
        (
            "named-address",
            "const A=[1]u8{97}\nfunc observe(){_=&A[0]}",
            vec![],
        ),
        (
            "field-address",
            "type S struct{Value:u8}\nconst A=S{Value:97}\nfunc observe(){_=&A.Value}",
            vec![],
        ),
        (
            "text-address",
            "func observe(){_=&\"ab\"[0]}",
            vec![composite(
                array(2, json!("U8")),
                vec![integer(json!("U8"), 97), integer(json!("U8"), 98)],
            )],
        ),
    ] {
        let g = generation(label, source);
        let mut actual = g["constants"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v["owner"] == owner("Function", "observe"))
            .cloned()
            .collect::<Vec<_>>();
        actual.sort_by_key(Value::to_string);
        let mut values = extra;
        values.push(integer(json!("U8"), 97));
        let mut expected = values
            .into_iter()
            .map(|v| json!({"owner":owner("Function","observe"),"value":v}))
            .collect::<Vec<_>>();
        expected.sort_by_key(Value::to_string);
        assert_eq!(actual, expected, "{label}");
        if label == "named-address" {
            assert!(g["constants"].as_array().unwrap().contains(&json!({"owner":owner("Constant","A"),"value":composite(array(1,json!("U8")),vec![integer(json!("U8"),97)])})));
        }
    }
}

#[test]
fn qualified_field_types_survive_value_node_coalescing() {
    let lib = "package lib\ntype Node struct{next:*Node;Value:u8}\nconst A=Node{Value:7}\nfunc Get[T](v:T)->T{return v}\n";
    let mut previous = None;
    for access in ["lib.A.Value", "(lib.A).Value"] {
        let g=generations("qualified-field-types",&format!("package main\nimport \"main/lib\"\nfunc f()->u32{{return ({access} as u32)+lib.Get[u32](1)}}\nfunc main(){{}}\n"),Some(lib)).remove("main").unwrap();
        let identity = json!({"package":"main/lib","name":"Node","arguments":[]});
        assert_eq!(
            g["nominals"],
            json!([{"identity":identity,"underlying":{"Struct":[{"name":"next","ty":{"Pointer":{"Defined":identity}},"private_owner":"main/lib"},{"name":"Value","ty":"U8","private_owner":null}]},"methods":[]}])
        );
        assert!(
            g["types"]
                .as_array()
                .unwrap()
                .contains(&json!({"Defined":identity}))
        );
        if let Some(previous) = previous {
            assert_eq!(g["types"], previous);
        }
        let mut expected_types = vec![
            json!("U8"),
            json!("U32"),
            json!({"Defined":identity}),
            json!({"Pointer":{"Defined":identity}}),
            json!({"Struct":[{"name":"next","ty":{"Pointer":{"Defined":identity}},"private_owner":"main/lib"},{"name":"Value","ty":"U8","private_owner":null}]}),
            json!({"Function":{"parameters":[],"returns":[]}}),
            json!({"Function":{"parameters":[],"returns":["U32"]}}),
            json!({"Function":{"parameters":["U32"],"returns":["U32"]}}),
        ];
        expected_types.sort_by_key(Value::to_string);
        assert_eq!(g["types"], json!(expected_types));
        previous = Some(g["types"].clone());
    }
}
