//! 成果物の規範caseを公開serde入口とcompiler出力で検証する。
use super::*;
use ond_protocol::{
    PackageArtifact,
    interface::{Interface, Value},
    object::{Section, SectionKind},
};
const ZERO_COMPILER: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[test]
fn unit_enum_variants_require_json_strings() {
    use ond_protocol::{
        debug::{DebugOperation, ReturnAddressLocation},
        object::{RelocationKind, SectionKind, SymbolBinding, SymbolKind},
    };
    macro_rules! check {
        ($ty:ty,$variant:literal) => {
            assert!(
                serde_json::from_str::<$ty>(concat!("{\"", $variant, "\":null}")).is_err(),
                stringify!($ty)
            );
            assert!(serde_json::from_str::<$ty>(concat!("\"", $variant, "\"")).is_ok());
        };
    }
    check!(SectionKind, "Text");
    check!(SymbolBinding, "Exported");
    check!(SymbolKind, "Function");
    check!(RelocationKind, "Abs32");
    check!(DebugOperation, "Call");
    check!(ReturnAddressLocation, "LinkRegister");
    check!(Kind, "U8");
    check!(ValueKind, "Zero");
    check!(ValueKind, "Nil");
}
const SHARED_INTERFACE: &str = r###"{"exports":{"T0":{"Global":0},"T1":{"Global":1},"T10":{"Global":10},"T11":{"Global":11},"T2":{"Global":2},"T3":{"Global":3},"T4":{"Global":4},"T5":{"Global":5},"T6":{"Global":6},"T7":{"Global":7},"T8":{"Global":8},"T9":{"Global":9}},"package":"p","types":[{"kind":"I8","methods":[],"name":null},{"kind":"U8","methods":[],"name":null},{"kind":"I16","methods":[],"name":null},{"kind":"I32","methods":[],"name":null},{"kind":"U32","methods":[],"name":null},{"kind":"F32","methods":[],"name":null},{"kind":"Bool","methods":[],"name":null},{"kind":{"Pointer":1},"methods":[],"name":null},{"kind":{"Function":{"parameters":[],"returns":[]}},"methods":[],"name":null},{"kind":{"Array":{"element":1,"length":2}},"methods":[],"name":null},{"kind":{"Struct":[{"name":"Value","private_owner":null,"ty":1}]},"methods":[],"name":null},{"kind":{"Defined":1},"methods":[],"name":"p.Byte"}]}"###;
fn typed_value(id: &str, json: &str, accept: bool, mutate: fn(&mut Interface)) {
    let mut interface: Interface = serde_json::from_str(SHARED_INTERFACE).unwrap();
    mutate(&mut interface);
    let value: Value = serde_json::from_str(json).unwrap();
    interface
        .exports
        .insert("Value".into(), Export::Constant(value));
    let result = interface.validate();
    assert_eq!(result.is_ok(), accept, "{id}: {result:?}");
}
macro_rules! value_case {
    ($name:ident,$id:literal,$json:literal,$accept:expr) => {
        #[test]
        fn $name() {
            typed_value($id, $json, $accept, |_| {});
        }
    };
}
value_case!(
    art_01_value_i8_min,
    "ART-01.value-i8-min",
    r###"{"kind":{"Integer":18446744073709551488},"ty":0}"###,
    true
);
value_case!(
    art_01_value_i8_unextended,
    "ART-01.value-i8-unextended",
    r###"{"kind":{"Integer":128},"ty":0}"###,
    false
);
value_case!(
    art_01_value_u8_max,
    "ART-01.value-u8-max",
    r###"{"kind":{"Integer":255},"ty":1}"###,
    true
);
value_case!(
    art_01_value_u8_overflow,
    "ART-01.value-u8-overflow",
    r###"{"kind":{"Integer":256},"ty":1}"###,
    false
);
value_case!(
    art_01_value_i32_minus_one,
    "ART-01.value-i32-minus-one",
    r###"{"kind":{"Integer":18446744073709551615},"ty":3}"###,
    true
);
value_case!(
    art_01_value_i32_unextended,
    "ART-01.value-i32-unextended",
    r###"{"kind":{"Integer":4294967295},"ty":3}"###,
    false
);
value_case!(
    art_01_value_wrong_kind,
    "ART-01.value-wrong-kind",
    r###"{"kind":{"Bool":true},"ty":1}"###,
    false
);
value_case!(
    art_01_value_nil_pointer,
    "ART-01.value-nil-pointer",
    r###"{"kind":"Nil","ty":7}"###,
    true
);
value_case!(
    art_01_value_nil_function,
    "ART-01.value-nil-function",
    r###"{"kind":"Nil","ty":8}"###,
    true
);
value_case!(
    art_01_value_nil_integer,
    "ART-01.value-nil-integer",
    r###"{"kind":"Nil","ty":4}"###,
    false
);
value_case!(
    art_01_value_false,
    "ART-01.value-false",
    r###"{"kind":{"Bool":false},"ty":6}"###,
    true
);
value_case!(
    art_01_value_positive_zero,
    "ART-01.value-positive-zero",
    r###"{"kind":{"Float32":0},"ty":5}"###,
    true
);
value_case!(
    art_01_value_negative_zero,
    "ART-01.value-negative-zero",
    r###"{"kind":{"Float32":2147483648},"ty":5}"###,
    true
);
value_case!(
    art_01_value_array_element,
    "ART-01.value-array-element",
    r###"{"kind":{"Composite":[[1,{"kind":{"Integer":255},"ty":1}]]},"ty":9}"###,
    true
);
value_case!(
    art_01_value_array_range,
    "ART-01.value-array-range",
    r###"{"kind":{"Composite":[[2,{"kind":"Zero","ty":1}]]},"ty":9}"###,
    false
);
value_case!(
    art_01_value_array_child_type,
    "ART-01.value-array-child-type",
    r###"{"kind":{"Composite":[[0,{"kind":{"Integer":1},"ty":0}]]},"ty":9}"###,
    false
);
value_case!(
    art_01_value_array_duplicate,
    "ART-01.value-array-duplicate",
    r###"{"kind":{"Composite":[[0,{"kind":"Zero","ty":1}],[0,{"kind":"Zero","ty":1}]]},"ty":9}"###,
    false
);
value_case!(
    art_01_value_struct_range,
    "ART-01.value-struct-range",
    r###"{"kind":{"Composite":[[1,{"kind":"Zero","ty":1}]]},"ty":10}"###,
    false
);
value_case!(
    art_01_value_aggregate_zero,
    "ART-01.value-aggregate-zero",
    r###"{"kind":{"Composite":[[1,{"kind":{"Integer":0},"ty":1}]]},"ty":9}"###,
    true
);
value_case!(
    art_01_value_defined_underlying,
    "ART-01.value-defined-underlying",
    r###"{"kind":{"Integer":255},"ty":11}"###,
    true
);

#[test]
fn art01_value_defined_cycle() {
    typed_value(
        "ART-01.value-defined-cycle",
        r#"{"ty":11,"kind":"Zero"}"#,
        false,
        |i| i.types[11].kind = Kind::Defined(11),
    );
}
#[test]
fn art01_value_pointer_cycle() {
    typed_value(
        "ART-01.value-pointer-cycle",
        r#"{"ty":11,"kind":"Zero"}"#,
        true,
        |i| {
            i.types[11].kind = Kind::Defined(7);
            i.types[7].kind = Kind::Pointer(11);
        },
    );
}
#[test]
fn art01_value_equivalent_child() {
    typed_value(
        "ART-01.value-equivalent-child",
        r#"{"ty":9,"kind":{"Composite":[[0,{"ty":12,"kind":{"Integer":1}}]]}}"#,
        true,
        |i| i.types.push(i.types[1].clone()),
    );
}
#[test]
fn art01_value_distinct_nominal() {
    typed_value(
        "ART-01.value-distinct-nominal",
        r#"{"ty":9,"kind":{"Composite":[[0,{"ty":12,"kind":{"Integer":1}}]]}}"#,
        false,
        |i| {
            let mut ty = i.types[11].clone();
            ty.name = Some("p.OtherByte".into());
            i.types.push(ty);
            i.types[9].kind = Kind::Array {
                length: 2,
                element: 11,
            };
        },
    );
}
fn written_constant(label: &str, declaration: &str) -> Interface {
    let f = Fixture::new(label);
    fs::write(
        f.0.join("main.ond"),
        format!("package main\n{declaration}\nfunc main(){{}}\n"),
    )
    .unwrap();
    let c = ond_compiler_core::compile_project(&f.0).unwrap();
    ond_compiler_core::export::package(&c.hir, &c.hir.packages[0].logical_path).unwrap()
}
macro_rules! writer_zero {($name:ident,$id:literal,$decl:literal)=>{#[test]fn $name(){let i=written_constant(stringify!($name),$decl);assert!(matches!(&i.exports["Value"],Export::Constant(v) if v.kind==ValueKind::Zero),"{}: {:?}",$id,i.exports);}};}
writer_zero!(
    art01_writer_nil_pointer,
    "ART-01.value-nil-pointer",
    "const Value:*u8=nil"
);
writer_zero!(
    art01_writer_nil_function,
    "ART-01.value-nil-function",
    "const Value:func()=nil"
);
writer_zero!(
    art01_writer_false,
    "ART-01.value-false",
    "const Value:bool=false"
);
writer_zero!(
    art01_writer_positive_zero,
    "ART-01.value-positive-zero",
    "const Value:f32=0.0"
);
writer_zero!(
    art01_writer_aggregate_zero,
    "ART-01.value-aggregate-zero",
    "const Value=[2]u8{1:0}"
);
#[test]
fn art01_writer_negative_zero() {
    let i = written_constant("negative-zero", "const Value:f32=-0.0");
    assert!(
        matches!(&i.exports["Value"],Export::Constant(v) if v.kind==ValueKind::Float32(2147483648))
    );
}
#[test]
fn art01_writer_signed_bits() {
    let i = written_constant("signed-bits", "const Value:i8=-128");
    assert!(
        matches!(&i.exports["Value"],Export::Constant(v) if v.kind==ValueKind::Integer(18446744073709551488))
    );
}
#[test]
fn art01_writer_sparse_order() {
    let i = written_constant("sparse-order", "const Value=[3]u8{2:7,0:0}");
    assert!(
        matches!(&i.exports["Value"],Export::Constant(v) if matches!(&v.kind,ValueKind::Composite(items) if items.len()==1 && items[0].0==2 && items[0].1.kind==ValueKind::Integer(7))),
        "ART-01 sparse writer: {:?}",
        i.exports
    );
}
const ARTIFACT_A: &str = r###"{"build_id":"1111111111111111111111111111111111111111111111111111111111111111","compiler_id":"0000000000000000000000000000000000000000000000000000000000000000","contract":{"abi":"ond-kagura-v1","revision":7,"target":"kagura-v1"},"debug":{"functions":[],"package":"sample","ranges":[]},"dependencies":{},"imports":[],"initializer":null,"interface":{"exports":{},"package":"sample","types":[]},"object":{"name":"diagnostic","relocations":[],"sections":[],"symbols":[]},"package":"sample"}"###;
fn artifact_mutation(
    change: fn(&mut serde_json::Value),
) -> Result<PackageArtifact, serde_json::Error> {
    let mut a: serde_json::Value = serde_json::from_str(ARTIFACT_A).unwrap();
    change(&mut a);
    serde_json::from_value(a)
}
#[test]
fn art01_closed_accept() {
    artifact_mutation(|_| {}).expect("ART-01.closed.accept");
}
#[test]
fn art01_required_null() {
    assert!(
        artifact_mutation(|a| {
            a.as_object_mut().unwrap().remove("initializer");
        })
        .is_err(),
        "ART-01.required-null"
    );
}
#[test]
fn art01_unknown_nested() {
    assert!(
        artifact_mutation(|a| a["object"]["extra"] = 0.into()).is_err(),
        "ART-01.unknown-nested"
    );
}
#[test]
fn closed_artifact_and_manifest_reject_removed_tests_field() {
    for tests in [
        serde_json::Value::Null,
        serde_json::json!({"cases": []}),
        serde_json::json!({"cases":[{"package":"sample","name":"TestValue","symbol":"sample.TestValue"}]}),
    ] {
        let mut artifact: serde_json::Value = serde_json::from_str(ARTIFACT_A).unwrap();
        artifact["tests"] = tests.clone();
        assert!(serde_json::from_value::<PackageArtifact>(artifact).is_err());
        let manifest = serde_json::json!({"contract":{"revision":7,"target":"kagura-v1","abi":"ond-kagura-v1"},"compiler_id":ZERO_COMPILER,"packages":[],"program":{"entry_symbol":"main.main","initializers":[]},"tests":tests});
        assert!(serde_json::from_value::<ond_protocol::CompilationManifest>(manifest).is_err());
    }
}
#[test]
fn art01_duplicate_decoded_key() {
    assert!(
        serde_json::from_str::<Contract>(
            r#"{"revision":7,"\u0072evision":7,"target":"kagura-v1","abi":"ond-kagura-v1"}"#
        )
        .is_err(),
        "ART-01.duplicate-decoded-key"
    );
}
#[test]
fn art01_composite_tuple() {
    serde_json::from_str::<ValueKind>(r#"{"Composite":[[0,{"ty":0,"kind":"Zero"}]]}"#)
        .expect("ART-01.composite-tuple");
}
#[test]
fn art01_composite_arity() {
    assert!(
        serde_json::from_str::<ValueKind>(r#"{"Composite":[[0]]}"#).is_err(),
        "ART-01.composite-arity"
    );
}
#[test]
fn art01_enum_case() {
    assert!(
        serde_json::from_str::<SectionKind>(r#""text""#).is_err(),
        "ART-01.enum-case"
    );
}
#[test]
fn art01_u32_overflow() {
    assert!(
        serde_json::from_str::<u32>("4294967296").is_err(),
        "ART-01.u32-overflow"
    );
}
#[test]
fn art01_integer_spelling() {
    for input in ["1.0", "1e0", "-0"] {
        assert!(
            serde_json::from_str::<u32>(input).is_err(),
            "ART-01.integer-spelling: {input}"
        );
    }
}
fn section_bytes(value: &str) -> Result<Section, serde_json::Error> {
    serde_json::from_str(&format!(
        r#"{{"kind":"Text","alignment":4,"data":{value},"memory_size":4}}"#
    ))
}
#[test]
fn art01_base64() {
    for (input, expected) in [
        (r#""""#, &b""[..]),
        (r#""TQ==""#, &b"M"[..]),
        (r#""TWE=""#, &b"Ma"[..]),
        (r#""TWFu""#, &b"Man"[..]),
    ] {
        assert_eq!(
            section_bytes(input).unwrap().data,
            expected,
            "ART-01.base64: {input}"
        );
    }
}
#[test]
fn art01_base64_reject() {
    for input in [r#""TQ""#, r#""TR==""#, r#""TQ==\n""#] {
        assert!(
            section_bytes(input).is_err(),
            "ART-01.base64-reject: {input}"
        );
    }
}
#[test]
fn contract_revision_seven() {
    let expected = Contract {
        revision: 7,
        target: "kagura-v1".into(),
        abi: "ond-kagura-v1".into(),
    };
    assert_eq!(Contract::default(), expected);
    expected.validate().unwrap();
    let mut legacy = expected;
    legacy.revision = 6;
    assert!(legacy.validate().is_err(), "明示入力の旧revisionは拒否する");
}
#[test]
fn hash01_public_struct_canonical_writer() {
    let c = Contract {
        revision: 7,
        target: "kagura-v1".into(),
        abi: "ond-kagura-v1".into(),
    };
    assert_eq!(
        cache::encode(&c).unwrap(),
        br#"{"abi":"ond-kagura-v1","revision":7,"target":"kagura-v1"}"#
    );
}
#[test]
fn hash01_file_bytes() {
    assert_eq!(ARTIFACT_A.len(), 486);
    assert_eq!(
        cache::hash(ARTIFACT_A.as_bytes()),
        "46abc29ca81801ce7a3bfcded39db1424d1d5ba7f53490384b8dcbc2dcb005f2"
    );
    assert_eq!(
        cache::hash(format!("{ARTIFACT_A}\n").as_bytes()),
        "da4b0eb92e4e1a6c9e3c5b4337f602c46c4902acb131b5f41db6ddc12ad66ea3"
    );
}
#[test]
fn hash01_empty_object() {
    let actual = cache::encode(&serde_json::json!({})).unwrap();
    assert_eq!(actual, [123, 125]);
    assert_eq!(
        cache::hash(&actual),
        "44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a"
    );
}
#[test]
fn hash01_string_escapes() {
    let actual = cache::encode(
        &serde_json::json!({"a":"\u{8}\t\n\u{c}\r\0\u{1f}\"\\/","z":u64::MAX,"あ":"é"}),
    )
    .unwrap();
    assert_eq!(
        actual,
        [
            123, 34, 97, 34, 58, 34, 92, 98, 92, 116, 92, 110, 92, 102, 92, 114, 92, 117, 48, 48,
            48, 48, 92, 117, 48, 48, 49, 102, 92, 34, 92, 92, 47, 34, 44, 34, 122, 34, 58, 49, 56,
            52, 52, 54, 55, 52, 52, 48, 55, 51, 55, 48, 57, 53, 53, 49, 54, 49, 53, 44, 34, 227,
            129, 130, 34, 58, 34, 195, 169, 34, 125
        ]
    );
    assert_eq!(
        cache::hash(&actual),
        "4de8ae025c8158e7530835967808cff223b8a64ce8126d19d1898c201256ad36"
    );
}
#[test]
fn hash01_scalar_key_order() {
    let actual = cache::encode(&serde_json::json!({"\u{e000}":0,"\u{1f600}":1})).unwrap();
    assert_eq!(
        actual,
        [
            123, 34, 238, 128, 128, 34, 58, 48, 44, 34, 240, 159, 152, 128, 34, 58, 49, 125
        ]
    );
    assert_eq!(
        cache::hash(&actual),
        "a712a1804e0980b85da1fbebfb21070144773d51ac011435af287d757b3e7dc5"
    );
}
fn hash_package(label: &str, source: &str, expected: &str, length: usize) {
    let f = Fixture::new(label);
    fs::write(f.0.join("ond.toml"), "[project]\nname = \"main\"\n").unwrap();
    fs::write(f.0.join("main.ond"), source).unwrap();
    let mut request = f.request();
    let loaded = crate::input::load(&request).unwrap();
    let (_, generation) =
        ond_compiler_core::compile_loaded_project_with_generation(loaded).unwrap();
    let generation: serde_json::Value = serde_json::from_str(&generation.packages["main"]).unwrap();
    let input = serde_json::json!({"compiler_id":ZERO_COMPILER,"contract":{"abi":"ond-kagura-v1","revision":7,"target":"kagura-v1"},"dependencies":{},"generation":generation,"manifest_hash":cache::hash(b"[project]\nname = \"main\"\n"),"package":"main","sources":[{"content_hash":cache::hash(source.as_bytes()),"path":"main.ond"}]});
    let bytes = cache::encode(&input).unwrap();
    assert_eq!(bytes.len(), length, "{label}");
    assert_eq!(cache::hash(&bytes), expected, "{label}: canonical bytes");
    request.contract.revision = 7;
    let result = compile(request, ZERO_COMPILER).expect("HASH-01 revision 7 compile precondition");
    assert_eq!(result.manifest.packages.len(), 1);
    assert_eq!(result.manifest.packages[0].build_id, expected, "{label}");
}
#[test]
fn hash01_package_empty() {
    hash_package(
        "HASH-01.package-empty",
        "package main\nfunc main(){}\n",
        "6ab6bd11899fd905bb33b9127f979f963d96e035338d3b3b7ba37ac57d65752a",
        540,
    );
}
#[test]
fn hash01_package_unused_body() {
    hash_package(
        "HASH-01.package-unused-body",
        "package main\nfunc add(x:i32)->i32{return x+1}\nfunc main(){}\n",
        "4e6ba067bcfba3520df5fb9a94eea4847c2e3cddd5b13bd2b4a1dea52f78de00",
        756,
    );
}

const ROOT_SOURCE: &str = r###"package main
type Root struct{secret:u8}
var Global:u8
func Id[T](x:T)->T{return x}
func main(){Global=Id[u8](1)}
"###;
const ROOT_TEST: &str = r###"package main
func TestRoot(){var r:Root;_=r.secret}
"###;
fn root_project(label: &str) -> Fixture {
    let f = Fixture::new(label);
    fs::write(f.0.join("ond.toml"), "[project]\nname = \"p\"\n").unwrap();
    fs::write(f.0.join("main.ond"), ROOT_SOURCE).unwrap();
    fs::write(f.0.join("root_test.ond"), ROOT_TEST).unwrap();
    f
}
#[test]
fn pkg01_root_logical_normal() {
    let f = root_project("PKG-01.root-logical-normal");
    let result = compile(f.request(), ZERO_COMPILER).unwrap();
    let reference = &result.manifest.packages[0];
    let artifact = cache::read_artifact(reference).unwrap();
    let owners = artifact
        .interface
        .types
        .iter()
        .filter_map(|ty| match &ty.kind {
            Kind::Struct(fields) => Some(fields),
            _ => None,
        })
        .flatten()
        .filter(|field| field.name == "secret")
        .map(|field| field.private_owner.as_deref())
        .collect::<Vec<_>>();
    let names = artifact
        .interface
        .types
        .iter()
        .filter_map(|ty| ty.name.as_deref())
        .collect::<Vec<_>>();
    assert_eq!(result.manifest.program.entry_symbol, "main.main");
    for symbol in ["main.main", "main.Global"] {
        assert!(
            artifact.object.symbols.iter().any(|s| s.name == symbol),
            "{symbol}"
        );
    }
    assert_eq!(
        (
            vec![
                reference.package.as_str(),
                artifact.package.as_str(),
                artifact.interface.package.as_str(),
                artifact.debug.package.as_str()
            ],
            names.contains(&"p.Root"),
            owners
        ),
        (vec!["p", "p", "p", "p"], true, vec![Some("p")]),
        "PKG-01.root-logical-normal: logical fields, nominal identity, private owner"
    );
}
fn generic_interface(label: &str) -> Interface {
    let f = Fixture::new(label);
    fs::write(f.0.join("ond.toml"), "[project]\nname = \"p\"\n").unwrap();
    fs::write(f.0.join("main.ond"),r###"package main
import "p/lib"
func observe()->u32{var b:lib.Box[u8];var f:func(u8)->u8=lib.Identity[u8];return (lib.Identity[u8](1) as u32)+(f(2) as u32)+lib.Identity[u32](4)+(b.Get() as u32)+((b+3) as u32)}
func main(){}
"###).unwrap();
    fs::create_dir(f.0.join("lib")).unwrap();
    fs::write(
        f.0.join("lib/lib.ond"),
        r###"package lib
type Box[T] struct{Value:T}
type Holder struct{Small:Box[u8];Nested:Box[Box[u8]]}
func Identity[T](x:T)->T{return x}
func Unused[T](x:T)->T{return x}
func (b:Box[T]) Get()->T{return b.Value}
operator (b:Box[T]) +(rhs:T)->T{return b.Value+rhs}
"###,
    )
    .unwrap();
    let c = ond_compiler_core::compile_project(&f.0).unwrap();
    ond_compiler_core::export::package(&c.hir, "p/lib").unwrap()
}
#[test]
fn art01_generic_exports() {
    let i = generic_interface("ART-01.generic-exports");
    assert_eq!(
        i.exports.keys().map(String::as_str).collect::<Vec<_>>(),
        ["Holder"]
    );
    assert!(matches!(i.exports["Holder"], Export::Type(_)));
}
#[test]
fn art01_generic_types() {
    let i = generic_interface("ART-01.generic-types");
    for name in [
        "p/lib.Box[\"U8\"]",
        r#"p/lib.Box[{"Defined":{"arguments":["U8"],"name":"Box","package":"p/lib"}}]"#,
    ] {
        assert!(
            i.types.iter().any(|ty| ty.name.as_deref() == Some(name)),
            "ART-01.generic-types: {name}: {:?}",
            i.types
        );
    }
}

#[test]
fn art01_duplicate_decoded_map_key() {
    assert!(serde_json::from_str::<Interface>(r#"{"package":"p","types":[{"name":null,"kind":"U8","methods":[]}],"exports":{"X":{"Global":0},"\u0058":{"Global":0}}}"#).is_err(),"ART-01 closed wire: duplicate decoded map key");
}

#[test]
fn direct_storage_cycles_reject_but_function_cycles_are_indirect() {
    let interface = |kind| {
        format!(
            r#"{{"package":"p","types":[{{"name":"p.T","kind":{{"Defined":1}},"methods":[]}},{{"name":null,"kind":{kind},"methods":[]}}],"exports":{{"T":{{"Type":0}}}}}}"#
        )
    };
    for kind in [
        r#"{"Array":{"length":1,"element":0}}"#,
        r#"{"Struct":[{"name":"x","ty":0,"private_owner":"p"}]}"#,
    ] {
        assert!(
            serde_json::from_str::<Interface>(&interface(kind))
                .unwrap()
                .validate()
                .is_err()
        );
    }
    for kind in [
        r#"{"Pointer":0}"#,
        r#"{"Function":{"parameters":[0],"returns":[0]}}"#,
    ] {
        serde_json::from_str::<Interface>(&interface(kind))
            .unwrap()
            .validate()
            .unwrap();
    }
}

#[test]
fn artifact_reader_rejects_invalid_build_hashes_with_matching_reference() {
    let f = Fixture::new("invalid-build-hashes");
    for (index, bad) in [
        "x",
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    ]
    .into_iter()
    .enumerate()
    {
        let mut artifact: PackageArtifact = serde_json::from_str(ARTIFACT_A).unwrap();
        artifact.build_id = bad.into();
        let reference = cache::publish(&f.0, &artifact).unwrap();
        assert!(cache::read_artifact(&reference).is_err(), "{index}");
    }
    let mut artifact: PackageArtifact = serde_json::from_str(ARTIFACT_A).unwrap();
    artifact.dependencies.insert("p".into(), "bad".into());
    let reference = cache::publish(&f.0, &artifact).unwrap();
    assert!(cache::read_artifact(&reference).is_err());
}
#[test]
fn hash01_image_build_id_uses_full_binary_bytes() {
    let f = Fixture::new("HASH-01.image-build-id");
    fs::write(f.0.join("main.ond"), "package main\nfunc main(){}\n").unwrap();
    let compiled = compile(f.request(), &crate::compiler_identity().unwrap()).unwrap();
    let settings:crate::settings::Settings=toml::from_str("[link]\nstartup='external'\nstack-size=8192\n[link.ram]\nbase=4096\nsize=65536\n[link.exit]\nmode='loop'\n").unwrap();
    let (linked, bytes) =
        crate::link_executable(compiled.manifest, &settings.link.unwrap()).unwrap();
    assert_eq!(linked.image.build_id, cache::hash(&bytes));
    assert_eq!(linked.debug.build_id, cache::hash(&bytes));
    ond_protocol::executable::Executable::read(&bytes).unwrap();
}
