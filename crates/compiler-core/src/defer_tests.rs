use crate::{
    memory_test_vm::execute,
    mir::validate_project,
    test_support::{compile, reject},
};

fn checked(source: &str) -> crate::Compilation {
    let compilation = compile(source, None).unwrap();
    validate_project(&compilation.mir).unwrap();
    compilation
}

#[test]
fn executes_reached_defers_once_in_lifo_order_at_function_exit() {
    let compilation = checked(
        r#"package main
var log: u32
func main() {}
func mark(value: u32) { log = log * 10 + value }
func work(mode: u32) -> u32 {
    defer { mark(1) }
    if mode == 1 {
        var local = 2 as u32
        defer { mark(local) }
        return 7
    }
    if mode == 2 { return 8 }
    var observed = 3 as u32
    defer { mark(observed) }
    observed = 4
    return 9
}
func run(mode: u32) -> (u32, u32) {
    log = 0
    var result = work(mode)
    return result, log
}
func fallthrough(enabled: bool) {
    defer { mark(4) }
    if enabled { defer { mark(5) } }
}
func observeFallthrough(enabled: bool) -> u32 {
    log = 0
    fallthrough(enabled)
    return log
}
"#,
    );

    assert_eq!(
        execute(&compilation.mir, "main.run", &[1]).unwrap(),
        [7, 21]
    );
    assert_eq!(execute(&compilation.mir, "main.run", &[2]).unwrap(), [8, 1]);
    assert_eq!(
        execute(&compilation.mir, "main.run", &[3]).unwrap(),
        [9, 41]
    );
    assert_eq!(
        execute(&compilation.mir, "main.observeFallthrough", &[0]).unwrap(),
        [4]
    );
    assert_eq!(
        execute(&compilation.mir, "main.observeFallthrough", &[1]).unwrap(),
        [54]
    );
}

#[test]
fn deferred_blocks_allow_internal_loop_control() {
    let compilation = checked(
        r#"package main
var log: i32
func main() {}
func run() -> u32 {
    defer {
        for i := 0; i < 5; i++ {
            if i == 1 { continue }
            if i == 3 { break }
            log = log * 10 + i + 1
        }
    }
    return 7
}
func observe() -> (u32, i32) {
    log = 0
    var result = run()
    return result, log
}
"#,
    );

    assert_eq!(
        execute(&compilation.mir, "main.observe", &[]).unwrap(),
        [7, 13]
    );
}

#[test]
fn rejects_defer_in_loops_nested_defer_and_return_from_defer() {
    reject(
        "package main\nfunc main() { for { defer {} } }\n",
        None,
        "defer is not allowed inside a loop",
        "defer {}",
    );
    reject(
        "package main\nfunc main() { defer { defer {} } }\n",
        None,
        "defer blocks cannot be nested",
        "defer {}",
    );
    reject(
        "package main\nfunc main() { defer { return } }\n",
        None,
        "return is not allowed inside a defer block",
        "return",
    );
}
