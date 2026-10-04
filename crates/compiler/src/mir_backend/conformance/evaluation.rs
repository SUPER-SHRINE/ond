use super::runner::*;
use kagura::FaultCode;

#[test]
fn shared_25_evaluation_cases_preserve_trace_and_final_values() {
    let cases = ond_compiler_core::test_vectors::evaluation::CASES;
    assert_eq!(cases.len(), 25);
    for case in cases {
        let names = (0..case.expected.len())
            .map(|i| format!("v{i}"))
            .collect::<Vec<_>>();
        let stores = names
            .iter()
            .enumerate()
            .map(|(i, v)| format!("store32({}, {v} as u32);", OUTPUT + i as u32 * 4))
            .collect::<String>();
        let entry = format!("func main(){{var {}=test();{stores}}}", names.join(","));
        let sources = case
            .files
            .iter()
            .map(|f| {
                let text = if f.path == "main.ond" {
                    assert_eq!(f.text.matches("func main() {}").count(), 1);
                    f.text.replacen("func main() {}", &entry, 1)
                } else {
                    f.text.to_owned()
                };
                (f.path, text)
            })
            .collect::<Vec<_>>();
        let files = sources
            .iter()
            .map(|(p, s)| (*p, s.as_str()))
            .collect::<Vec<_>>();
        let bytes = compile(&files);
        let expected = case.expected.iter().map(|n| *n as u32).collect::<Vec<_>>();
        run(
            &bytes,
            Case {
                id: case.id,
                specs: case.specs,
                inputs: &[],
                output: &expected,
                end: End::Return,
                steps: 200_000,
            },
        );
    }
}

#[test]
fn discard_and_callee_evaluation_keep_effects_and_multi_result_positions() {
    for (id, source, want) in [
        (
            "DECL-03.discard",
            format!(
                "package main\nvar trace:u32\nfunc mark(n:u32)->u32{{trace=trace*10+n;return n}}\nfunc pair(_:u32)->(u32,u32,u32){{return mark(3),mark(4),mark(5)}}\nfunc main(){{_=mark(1);var _,b,_=pair(mark(2));_=mark(6);store32({OUTPUT},trace);store32({},b)}}",
                OUTPUT + 4
            ),
            vec![123456, 4],
        ),
        (
            "EVAL-01.callee",
            format!(
                "package main\nvar trace:u32\nfunc mark(n:u32)->u32{{trace=trace*10+n;return n}}\nfunc f(a:u32,b:u32)->u32{{return a+b}}\nfunc target()->func(u32,u32)->u32{{_=mark(1);return f}}\nfunc main(){{var n=target()(mark(2),mark(3));store32({OUTPUT},trace);store32({},n)}}",
                OUTPUT + 4
            ),
            vec![123, 5],
        ),
    ] {
        run(
            &compile(&[("main.ond", &source)]),
            Case {
                id,
                specs: &["DECL-03", "EVAL-01"],
                inputs: &[],
                output: &want,
                end: End::Return,
                steps: 200_000,
            },
        );
    }
}

#[test]
fn faults_do_not_execute_following_effects_or_overwrite_destination() {
    // Guards on BOTH sides catch even a speculative negative-index store.
    let array = OUTPUT + 4;
    for (id, body, code) in [
        (
            "NIL-02.call",
            "var f:func();f()".to_owned(),
            FaultCode::BusFault,
        ),
        (
            "DECL-03.discard-fault",
            format!("var x=load32({INPUT});_=(1 as u32)/x"),
            FaultCode::InvalidInstruction,
        ),
        (
            "BOUND-01.negative",
            format!("var p={array} as *[2]u32;var i=load32({INPUT}) as i32;(*p)[i]=9"),
            FaultCode::InvalidInstruction,
        ),
        (
            "BOUND-01.length",
            format!("var p={array} as *[2]u32;var i=load32({INPUT});(*p)[i]=9"),
            FaultCode::InvalidInstruction,
        ),
        (
            "BOUND-01.empty",
            format!("var p={array} as *[0]u32;var i=load32({INPUT});(*p)[i]=9"),
            FaultCode::InvalidInstruction,
        ),
    ] {
        let input = if id.ends_with("negative") {
            u32::MAX
        } else if id.ends_with("length") {
            2
        } else {
            0
        };
        let source = format!(
            "package main\nfunc main(){{{body};store32({},99)}}",
            OUTPUT + 16
        );
        run(
            &compile(&[("main.ond", &source)]),
            Case {
                id,
                specs: &["NIL-02", "DECL-03", "BOUND-01"],
                inputs: &[input],
                output: &[SENTINEL; 5],
                end: End::Fault(code),
                steps: 200_000,
            },
        );
    }
    let source = format!(
        "package main\nfunc main(){{var p={OUTPUT} as *[2]u32;var i=load32({INPUT});(*p)[i]=9}}"
    );
    run(
        &compile(&[("main.ond", &source)]),
        Case {
            id: "BOUND-01.last",
            specs: &["BOUND-01"],
            inputs: &[1],
            output: &[SENTINEL, 9],
            end: End::Return,
            steps: 200_000,
        },
    );
}
