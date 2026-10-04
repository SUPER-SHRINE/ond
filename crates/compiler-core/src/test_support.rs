use crate::{Compilation, FrontendError, compile_project};

/// Legacy test adapter: still checks the common DIAG contract and resolves file IDs
/// through SourceDb, never assuming that main.ond was loaded first.
#[track_caller]
pub(super) fn reject(main: &str, shared: Option<&str>, message: &'static str, span: &str) {
    let occurrence = main
        .match_indices(span)
        .count()
        .checked_sub(1)
        .expect("expected span absent from source");
    reject_at(main, shared, message, span, occurrence);
}

#[track_caller]
pub(super) fn reject_at(
    main: &str,
    shared: Option<&str>,
    message: &'static str,
    span: &str,
    occurrence: usize,
) {
    use crate::conformance::*;
    let mut files = vec![SourceFile {
        path: "main.ond",
        text: main,
    }];
    if let Some(text) = shared {
        files.push(SourceFile {
            path: "shared/shared.ond",
            text,
        });
    }
    let caller = std::panic::Location::caller();
    let id = format!("DIAG-01.legacy.{}:{}", caller.file(), caller.line());
    run(&Case {
        id: &id,
        specs: &["DIAG-01"],
        layer: Layer::Core,
        manifest: true,
        files: &files,
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: crate::diagnostic::Severity::Error,
            message: Message::Contains(message),
            location: Location::Source {
                file: "main.ond",
                text: span,
                occurrence,
            },
        }]),
    })
    .unwrap_or_else(|error| panic!("{error}\nsource:\n{main}"));
}

pub(super) fn compile(main: &str, shared: Option<&str>) -> Result<Compilation, FrontendError> {
    let mut files = vec![("main.ond", main)];
    if let Some(shared) = shared {
        files.push(("shared/shared.ond", shared));
    }
    compile_files(&files)
}

pub(super) fn compile_files(files: &[(&str, &str)]) -> Result<Compilation, FrontendError> {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "ond-semantics-{}-{unique}-{sequence}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("ond.toml"), "").unwrap();
    for (name, source) in files {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, source).unwrap();
    }
    let result = compile_project(&root);
    std::fs::remove_dir_all(root).unwrap();
    result
}
