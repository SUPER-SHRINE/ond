use crate::test_support::compile;
use std::{
    fs,
    process::{Command, Stdio},
    sync::Mutex,
    time::{Duration, Instant, SystemTime},
};

static TYPE05_CHILD: Mutex<()> = Mutex::new(());

fn type05_reject_isolated(test_name: &str, source: &str) {
    const ROOT_ENV: &str = "OND_TYPE05_CHILD_ROOT";
    if let Some(root) = std::env::var_os(ROOT_ENV) {
        let loaded = crate::load_project(&root).unwrap();
        crate::ond::parse_project(&loaded)
            .expect("TYPE-05 input must parse before semantic rejection");
        let file = loaded.packages[0].files[0];
        let error = match crate::compile_loaded_project(loaded) {
            Err(error) => error,
            Ok(_) => {
                panic!("TYPE-05 {test_name}: expected semantic rejection, compilation succeeded")
            }
        };
        let source = fs::read_to_string(std::path::Path::new(&root).join("main.ond")).unwrap();
        let (request, root_use, declaration) = match test_name {
            "type05_growing_function" => ("Grow[*T]()", "Grow[u8]()", "func Grow[T]"),
            "type05_mutual_function" => ("F[T]()", "F[u8]()", "func F[T]"),
            "type05_flipped_function" => ("Flip[B,A]()", "Flip[u8,u16]()", "func Flip[A,B]"),
            "type05_growing_nominal" | "type05_nested_growing_nominal" => {
                ("Grow[*T]", "Grow[u8]", "type Grow[T]")
            }
            "type05_growing_method" => ("next.Grow()", "b.Grow()", "func (b:Box[T]) Grow()"),
            "type05_growing_operator" => ("next+next", "b+b", "operator (b:Box[T]) +"),
            _ => panic!("unregistered TYPE-05 rejection case"),
        };
        let touches = |span: crate::source::Span, needle: &str| {
            let start = source.find(needle).unwrap();
            span.file == file
                && !span.is_synthetic()
                && span.start < span.end
                && span.end <= source.len()
                && source.is_char_boundary(span.start)
                && source.is_char_boundary(span.end)
                && span.start >= start
                && span.end <= start + needle.len()
        };
        assert!(
            error.diagnostics().iter().any(|d| {
                d.severity == crate::diagnostic::Severity::Error
                    && !d.message.contains("internal")
                    && (touches(d.primary.span, request) || touches(d.primary.span, root_use))
                    && d.related
                        .iter()
                        .any(|label| touches(label.span, declaration))
            }),
            "TYPE-05 requires a source cause with the generic declaration/use related: {error}"
        );
        fs::write(std::path::Path::new(&root).join("completed"), test_name).unwrap();
        return;
    }

    let _guard = TYPE05_CHILD
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let root = std::env::temp_dir().join(format!(
        "ond-type05-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("ond.toml"), "[project]\nname = \"test\"\n").unwrap();
    fs::write(root.join("main.ond"), source).unwrap();
    let module = module_path!().rsplit("::").next().unwrap();
    let exact = format!("{module}::{test_name}");
    let mut child = Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg(exact)
        .env(ROOT_ENV, &root)
        .stdout(Stdio::from(fs::File::create(root.join("stdout")).unwrap()))
        .stderr(Stdio::from(fs::File::create(root.join("stderr")).unwrap()))
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break Some(status);
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let output = format!(
        "{}\n{}",
        fs::read_to_string(root.join("stdout")).unwrap(),
        fs::read_to_string(root.join("stderr")).unwrap()
    );
    let completed = fs::read_to_string(root.join("completed")).ok().as_deref() == Some(test_name);
    let _ = fs::remove_dir_all(&root);
    assert!(
        status.is_some_and(|status| status.success()) && completed,
        "TYPE-05 child failed, timed out, or did not execute {test_name}: {status:?}\n{output}"
    );
}

fn assert_related(source: &str, error: &crate::FrontendError, primary: &str, related: &[&str]) {
    let diagnostics = error
        .diagnostics()
        .iter()
        .filter(|diagnostic| {
            &source[diagnostic.primary.span.start..diagnostic.primary.span.end] == primary
        })
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    let spans = diagnostics[0]
        .related
        .iter()
        .map(|label| &source[label.span.start..label.span.end])
        .collect::<Vec<_>>();
    let unique_spans = diagnostics[0]
        .related
        .iter()
        .map(|label| label.span)
        .collect::<Vec<_>>();
    assert!(
        unique_spans
            .iter()
            .enumerate()
            .all(|(index, span)| !unique_spans[..index].contains(span)),
        "{spans:?}"
    );
    assert_eq!(spans.len(), related.len(), "{spans:?}");
    for expected in related {
        let expected_count = related
            .iter()
            .filter(|candidate| *candidate == expected)
            .count();
        assert_eq!(
            spans.iter().filter(|span| **span == *expected).count(),
            expected_count,
            "wrong number of {expected:?} labels: {spans:?}"
        );
    }
}

#[test]
fn failed_function_specialization_collects_late_nested_requests() {
    let source = r#"package main
func Invalid[T](value: T) -> T { return value + value }
func Bridge[T](value: T) { _ = Invalid[T](value) }
func Outer[T](value: T) { Bridge[T](value) }
func main() {
    _ = Invalid[bool](true)
    Bridge[bool](false)
    Outer[bool](true)
    Outer[bool](false)
}
"#;
    let error = compile(source, None).expect_err("bool addition in a generic body must fail");
    assert_eq!(error.diagnostics().len(), 1, "{:?}", error.diagnostics());
    assert_related(
        source,
        &error,
        "value + value",
        &[
            "Invalid[bool](true)",
            "Bridge[bool](false)",
            "Outer[bool](true)",
            "Outer[bool](false)",
            "Bridge[T](value)",
            "Invalid[T](value)",
        ],
    );
}

#[test]
fn failed_generic_method_collects_late_requests() {
    let source = r#"package main
type Box[T] struct { value: T }
func (box: Box[T]) Invalid() -> T { return box.value + box.value }
func (box: Box[T]) Later() { _ = box.Invalid() }
func main() {
    box := Box[bool]{value: true}
    _ = box.Invalid()
    box.Later()
}
"#;
    let error = compile(source, None).expect_err("bool addition in a generic method must fail");
    assert_eq!(error.diagnostics().len(), 1, "{:?}", error.diagnostics());
    assert_related(
        source,
        &error,
        "box.value + box.value",
        &["box.Invalid()", "box.Later()", "box.Invalid()"],
    );
}

#[test]
fn failed_generic_operator_collects_late_method_requests() {
    let source = r#"package main
type Box[T] struct { value: T }
type Wrap[T] struct { value: Box[T] }
operator (lhs: Box[T]) + (rhs: Box[T]) -> Box[T] {
    return Box[T]{value: lhs.value + rhs.value}
}

operator (wrap: Wrap[T]) len() -> u32 {
    _ = wrap.value + wrap.value
    return 0
}

func main() {
    box := Box[bool]{value: true}
    _ = box + box
    wrap := Wrap[bool]{value: box}
    _ = len(wrap)
}
"#;
    let error = compile(source, None).expect_err("bool addition in a generic operator must fail");
    assert_eq!(error.diagnostics().len(), 1, "{:?}", error.diagnostics());
    assert_related(
        source,
        &error,
        "lhs.value + rhs.value",
        &["box + box", "len(wrap)", "wrap.value + wrap.value"],
    );
}

#[test]
fn mutual_generic_recursion_keeps_failed_descendant_reachable() {
    let source = r#"package main
func First[T](value: T) { Second[T](value) }
func Second[T](value: T) {
    First[T](value)
    Invalid[T](value)
}

func Invalid[T](value: T) { _ = value + value }
func main() {
    First[bool](true)
    Second[bool](false)
}
"#;
    let error = compile(source, None).expect_err("bool addition in a reachable generic must fail");
    assert_eq!(error.diagnostics().len(), 1, "{:?}", error.diagnostics());
    assert_related(
        source,
        &error,
        "value + value",
        &[
            "First[bool](true)",
            "Second[bool](false)",
            "Second[T](value)",
            "First[T](value)",
            "Invalid[T](value)",
        ],
    );
}

#[test]
fn mutually_recursive_generic_instances_compile_and_terminate() {
    let source = r#"package main
func First[T](value: T) { Second[T](value) }
func Second[T](value: T) { First[T](value) }
func main() { First[u32](1) }
"#;
    compile(source, None).expect("a recursive generic call cycle should reach a fixed point");
}

#[test]
fn densely_mutually_recursive_generic_instances_compile() {
    let source = r#"package main
func First[T](value: T) {
    Second[T](value)
    Third[T](value)
    Fourth[T](value)
}
func Second[T](value: T) {
    First[T](value)
    Third[T](value)
    Fourth[T](value)
}
func Third[T](value: T) {
    First[T](value)
    Second[T](value)
    Fourth[T](value)
}
func Fourth[T](value: T) {
    First[T](value)
    Second[T](value)
    Third[T](value)
}
func main() { First[u32](1) }
"#;
    compile(source, None).expect("dense mutual recursion should converge over distinct spans");
}

#[test]
fn generic_self_recursion_merges_distinct_call_site_spans() {
    let source = r#"package main
func Recursive[T](value: T) {
    Recursive[T](value)
    Recursive[T](value)
    Recursive[T](value)
    Recursive[T](value)
    Recursive[T](value)
    Recursive[T](value)
    Recursive[T](value)
}
func main() { Recursive[u32](1) }
"#;
    compile(source, None).expect("self-recursive call sites should merge by root use");
}

#[test]
fn fifteen_mutually_recursive_generic_instances_compile() {
    let functions = (0..15)
        .map(|index| format!("Node{index:02}"))
        .collect::<Vec<_>>();
    let mut source = String::from("package main\n");
    for (index, function) in functions.iter().enumerate() {
        source.push_str(&format!("func {function}[T](value: T) {{\n"));
        for (other, target) in functions.iter().enumerate() {
            if index != other {
                source.push_str(&format!("    {target}[T](value)\n"));
            }
        }
        source.push_str("}\n");
    }
    source.push_str("func main() { Node00[u32](1) }\n");

    compile(&source, None).expect("dense 15-node mutual recursion should converge");
}

#[test]
fn two_hundred_forty_mutually_recursive_generic_instances_compile() {
    let source = generic_ring_source(false);
    compile(&source, None).expect("a 240-node mutual recursion ring should converge");
}

fn generic_ring_source(failing: bool) -> String {
    let mut source = String::from("package main\n");
    for index in 0..240 {
        source.push_str(&format!(
            "func Node{index:03}[T](value: T) {{ Node{:03}[T](value)\n{}\n}}\n",
            (index + 1) % 240,
            if failing && index == 239 {
                "Invalid[T](value)"
            } else {
                ""
            },
        ));
    }
    if failing {
        source.push_str("func Invalid[T](value: T) { _ = value + value }\n");
        source.push_str("func main() { Node000[bool](true) }\n");
    } else {
        source.push_str("func main() { Node000[u32](1) }\n");
    }
    source
}

#[test]
fn failed_descendant_collects_all_requests_through_a_large_generic_ring() {
    let source = generic_ring_source(true);
    let error = compile(&source, None).expect_err("the reachable bool addition must fail");
    assert_eq!(error.diagnostics().len(), 1, "{:?}", error.diagnostics());
    let mut related = (0..240)
        .map(|index| format!("Node{index:03}[T](value)"))
        .collect::<Vec<_>>();
    related.push("Node000[bool](true)".into());
    related.push("Invalid[T](value)".into());
    assert_related(
        &source,
        &error,
        "value + value",
        &related.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    let labels = &error.diagnostics()[0].related;
    assert_eq!(
        labels
            .iter()
            .filter(|label| label.message == "generic function instantiated here")
            .count(),
        1,
    );
    assert_eq!(
        labels
            .iter()
            .filter(|label| label.message == "nested generic function instantiation requested here")
            .count(),
        241,
    );
}

#[test]
fn type05_direct_function() {
    let source = "package main\nfunc F[T](){F[T]()}\nfunc main(){F[u8]()}\n";
    compile(source, None).unwrap();
}

#[test]
fn type05_independent_functions() {
    let source = "package main\nfunc F[T](){F[T]()}\nfunc main(){F[u8]();F[u16]()}\n";
    compile(source, None).unwrap();
}

#[test]
fn type05_finite_function_chain() {
    let source = "package main\nfunc F[T](){G[*T]()}\nfunc G[T](){}\nfunc main(){F[u8]()}\n";
    compile(source, None).unwrap();
}

#[test]
fn type05_growing_function() {
    type05_reject_isolated(
        "type05_growing_function",
        "package main\nfunc Grow[T](){Grow[*T]()}\nfunc main(){Grow[u8]()}\n",
    );
}

#[test]
fn type05_mutual_function() {
    type05_reject_isolated(
        "type05_mutual_function",
        "package main\nfunc F[T](){G[*T]()}\nfunc G[T](){F[T]()}\nfunc main(){F[u8]()}\n",
    );
}

#[test]
fn type05_flipped_function() {
    type05_reject_isolated(
        "type05_flipped_function",
        "package main\nfunc Flip[A,B](){Flip[B,A]()}\nfunc main(){Flip[u8,u16]()}\n",
    );
}

#[test]
fn type05_finite_nominal() {
    let source = "package main\ntype Box[T] struct{value:T}\nvar B:Box[Box[u8]]\nfunc main(){}\n";
    compile(source, None).unwrap();
}

#[test]
fn type05_pointer_nominal() {
    let source =
        "package main\ntype Node[T] struct{next:*Node[T]}\nvar N:Node[u8]\nfunc main(){}\n";
    compile(source, None).unwrap();
}

#[test]
fn type05_growing_nominal() {
    type05_reject_isolated(
        "type05_growing_nominal",
        "package main\ntype Grow[T] struct{next:*Grow[*T]}\nvar G:Grow[u8]\nfunc main(){}\n",
    );
}

#[test]
fn type05_growing_method() {
    type05_reject_isolated(
        "type05_growing_method",
        "package main\ntype Box[T] struct{}\nfunc (b:Box[T]) Grow(){var next:Box[*T];next.Grow()}\nfunc main(){var b:Box[u8];b.Grow()}\n",
    );
}

#[test]
fn type05_growing_operator() {
    type05_reject_isolated(
        "type05_growing_operator",
        "package main\ntype Box[T] struct{}\noperator (b:Box[T]) +(rhs:Box[T])->u8{var next:Box[*T];return next+next}\nfunc main(){var b:Box[u8];_=b+b}\n",
    );
}

#[test]
fn type05_nested_growing_nominal() {
    type05_reject_isolated(
        "type05_nested_growing_nominal",
        "package main\ntype Relay[U] U\ntype Grow[T] struct{next:*Relay[Grow[*T]]}\nvar G:Grow[u8]\nfunc main(){}\n",
    );
}

#[test]
fn generic_paths_include_skipped_root_and_nested_source_uses() {
    let source = "package main\nfunc Flip[A,B]()->bool{return false&&Flip[B,A]()}\nfunc unused(){_=true||Flip[u8,u16]()}\nfunc main(){}\n";
    let error =
        compile(source, None).expect_err("skipped source uses still require valid instantiations");
    assert!(
        error
            .diagnostics()
            .iter()
            .any(|d| d.message.contains("same path"))
    );
}

#[test]
fn generic_paths_check_existing_instances_and_function_values() {
    let source = "package main\nfunc Pick[T](){_=Pick[u16]}\nfunc main(){_=Pick[u8];_=Pick[u16]}\n";
    let error = compile(source, None)
        .expect_err("already requested function values still form invalid paths");
    assert!(
        error
            .diagnostics()
            .iter()
            .any(|d| d.message.contains("same path")),
        "{error}"
    );
}

#[test]
fn generic_paths_replay_completed_nominal_requests() {
    let source = "package main\ntype A[T] struct{next:*B[u8]}\ntype B[T] struct{next:*A[u16]}\nvar Early:B[u8]\nvar Later:A[u8]\nfunc main(){}\n";
    let error = compile(source, None)
        .expect_err("a cached nominal must be checked in each requesting path");
    assert!(
        error
            .diagnostics()
            .iter()
            .any(|d| d.message.contains("same path"))
    );
}

#[test]
fn generic_paths_keep_many_independent_roots_separate() {
    let mut source = String::from("package main\nfunc F[T](){F[T]()}\nfunc main(){\n");
    for length in 0..32 {
        source.push_str(&format!("F[[{length}]u8]()\n"));
    }
    source.push_str("}\n");
    compile(&source, None).expect("independent roots may request different arguments");
}

#[test]
fn generic_paths_continue_checking_independent_failures() {
    let source = "package main\nfunc Grow[T](){Grow[*T]()}\nfunc Invalid[T](x:T){_=x.missing}\nfunc main(){Grow[u8]();Invalid[u8](1)}\n";
    let error = compile(source, None).expect_err("both independent roots must be checked");
    assert!(
        error
            .diagnostics()
            .iter()
            .any(|d| d.message.contains("same path"))
    );
    assert!(
        error.diagnostics().iter().any(|d| {
            let span = d.primary.span;
            &source[span.start..span.end] == "x.missing"
        }),
        "{error}"
    );
}

#[test]
fn diag02_generic_related_causes() {
    let source = "package main\ntype S struct{}\nfunc Need[T](x:T)->u32{return x.missing}\nfunc Outer[T](x:T)->u32{return Need[T](x)}\nfunc main(){_=Outer[S](S{});_=Need[S](S{});_=Need[u32](1)}\n";
    let error = compile(source, None).expect_err("both generic instantiations must fail");
    let within = |span: crate::source::Span, needle: &str| {
        let start = source.find(needle).unwrap();
        !span.is_synthetic()
            && span.start < span.end
            && span.start >= start
            && span.end <= start + needle.len()
            && source.is_char_boundary(span.start)
            && source.is_char_boundary(span.end)
    };
    let causes = error
        .diagnostics()
        .iter()
        .filter(|d| {
            d.severity == crate::diagnostic::Severity::Error && within(d.primary.span, "x.missing")
        })
        .collect::<Vec<_>>();
    assert_eq!(causes.len(), 2, "DIAG-02.generic-related-causes: {error}");
    let has = |d: &crate::diagnostic::Diagnostic, needle: &str| {
        d.related.iter().any(|label| within(label.span, needle))
    };
    let s_cause = causes
        .iter()
        .position(|d| has(d, "Outer[S](S{})") && has(d, "Need[S](S{})") && has(d, "Need[T](x)"));
    let u32_cause = causes.iter().position(|d| has(d, "Need[u32](1)"));
    assert!(
        s_cause.is_some() && u32_cause.is_some() && s_cause != u32_cause,
        "DIAG-02.generic-related-causes: independent specializations and all requests required: {error}"
    );
}
