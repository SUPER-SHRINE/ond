use crate::CompileRequest;
use crate::{cache, compile};
use ond_protocol::{
    Contract,
    interface::{Export, Kind, ValueKind},
};
use std::{collections::BTreeMap, fs, path::PathBuf};
struct Fixture(PathBuf);
impl Fixture {
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!("ond-exports-{label}-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("ond.toml"), "").unwrap();
        Self(root)
    }
    fn request(&self) -> CompileRequest {
        CompileRequest {
            excluded_dirs: Vec::new(),
            contract: Contract::default(),
            project_root: self.0.clone(),
            sources: BTreeMap::new(),
            libraries: BTreeMap::new(),
            cache_dir: None,
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn export_closure_roundtrips_recursive_types_private_owners_and_constant_bits() {
    let f = Fixture::new("closure");
    let mut request = f.request();
    request.sources = BTreeMap::from([
        (
            "main.ond".into(),
            "package main\nimport \"api\"\nfunc main(){var _=api.Flag}".into(),
        ),
        (
            "api/api.ond".into(),
            "package api
type hidden struct { value:i32 }
type Node struct { Next:*Node; hidden:hidden }
type unused struct { Field:u32 }
const Flag:bool=true
const Count:i32=-7
const Real:f32=-0.0
const Empty:*Node=nil
const Bytes=[...]u8{1,2,3}
const Record=hidden{value:42}
const private=99
var Storage:Node
func Pair(node:*Node)->(*Node,i32){return node,Count}
func secret(){}
"
            .into(),
        ),
    ]);
    let result = compile(request.clone(), "test").unwrap();
    let reference = result
        .manifest
        .packages
        .iter()
        .find(|p| p.package == "api")
        .unwrap();
    let artifact = cache::read_artifact(reference).unwrap();
    let data = &artifact.interface;
    assert!(!data.exports.contains_key("secret") && !data.exports.contains_key("private"));
    assert!(!data.exports.contains_key("hidden"));
    assert!(
        !data
            .types
            .iter()
            .any(|t| t.name.as_deref() == Some("api.unused"))
    );
    assert!(
        data.types
            .iter()
            .any(|t| t.name.as_deref() == Some("api.hidden"))
    );
    let Export::Type(node) = data.exports["Node"] else {
        panic!("not a type")
    };
    assert!(data.types.iter().any(|t| t.kind == Kind::Pointer(node)));
    assert!(
        data.types
            .iter()
            .any(|t| matches!(&t.kind, Kind::Struct(fields)
        if fields.iter().any(|f| f.name=="hidden" && f.private_owner.as_deref()==Some("api"))))
    );
    assert!(
        matches!(&data.exports["Real"], Export::Constant(v) if v.kind == ValueKind::Float32(0x80000000))
    );
    assert!(matches!(&data.exports["Empty"], Export::Constant(v) if v.kind == ValueKind::Nil));
    assert!(
        matches!(&data.exports["Bytes"], Export::Constant(v) if matches!(&v.kind, ValueKind::Composite(items) if items.len()==3))
    );
    let serialized = cache::encode(data).unwrap();
    let decoded: ond_protocol::interface::Interface = serde_json::from_slice(&serialized).unwrap();
    assert_eq!(data, &decoded);
    decoded.validate().unwrap();
    // An unrelated package shifts project-global IDs, but not artifact-local references.
    request.sources.insert(
        "aaa/extra.ond".into(),
        "package aaa\ntype Z struct{v:u32}".into(),
    );
    request.cache_dir = Some("target/fresh-cache".into());
    let changed = compile(request, "test").unwrap();
    let changed = cache::read_artifact(
        changed
            .manifest
            .packages
            .iter()
            .find(|p| p.package == "api")
            .unwrap(),
    )
    .unwrap();
    assert_eq!(data, &changed.interface);
    let mut invalid = decoded;
    invalid
        .exports
        .insert("Broken".into(), Export::Type(u32::MAX));
    assert!(invalid.validate().unwrap_err().contains("missing type"));
}
#[test]
fn optional_library_reachability_and_required_package_membership() {
    let f = Fixture::new("optional");
    let mut request = f.request();
    request
        .sources
        .insert("main.ond".into(), "package main\nfunc main(){}".into());
    request.libraries.insert(
        "lib/lib.ond".into(),
        "package lib\nimport \"dep\"\nconst Value=dep.Value".into(),
    );
    request
        .libraries
        .insert("dep/dep.ond".into(), "package dep\nconst Value=5".into());
    let result = compile(request.clone(), "test").unwrap();
    assert_eq!(result.rebuilt, ["."]);
    request.sources.insert(
        "main.ond".into(),
        "package main\nimport \"lib\"\nfunc main(){var _=lib.Value}".into(),
    );
    assert_eq!(
        compile(request.clone(), "test").unwrap().rebuilt,
        [".", "dep", "lib"]
    );
    request
        .sources
        .insert("main.ond".into(), "package main\nfunc main(){}".into());
    request.sources.insert(
        "lib/required.ond".into(),
        "package lib\nconst Extra=2".into(),
    );
    let result = compile(request, "test").unwrap();
    assert_eq!(result.manifest.packages.len(), 3);
}
