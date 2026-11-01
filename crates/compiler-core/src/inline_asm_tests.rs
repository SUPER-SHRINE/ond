//! Inline assemblyのCore適合拒否。根拠: specs/ond/conformance/cases/inline-assembly.md。
use crate::{Severity, test_support::compile};

fn reject(
    id: &str,
    requirement: &str,
    source: &str,
    context: &str,
    token: Option<&str>,
    reasons: &[&[&str]],
) {
    let context_start = source
        .find(context)
        .expect("cause context must occur in source");
    let cause = token.unwrap_or(context);
    let start = context_start
        + context
            .find(cause)
            .expect("cause token must occur in context");
    let end = start + cause.len();
    let error = compile(source, None).expect_err("Core must reject invalid inline assembly");
    assert!(error.diagnostics().iter().any(|diagnostic| {
        let span = diagnostic.primary.span;
        let message = diagnostic.message.to_lowercase();
        diagnostic.severity == Severity::Error
            && !span.is_synthetic()
            && span.start < span.end
            // R11はbindingが原因位置であり、その中のtokenを指してよい。
            && (token.is_none() || (span.start <= start && span.end >= end))
            // 周囲のasm全体だけを指すgeneric構文診断では原因を示していない。
            && span.start >= context_start
            && span.end <= context_start + context.len()
            && reasons.iter().all(|alternatives| alternatives.iter().any(|word| message.contains(word)))
    }), "{id} [{requirement}]: cause `{cause}` at {start}..{end}, diagnostics: {error:?}");
}

#[test]
fn asm_r01_duplicate_register() {
    reject(
        "ASM-R01.duplicate-register",
        "ASM-02",
        "package main\nfunc main(){var x:u32=1;var y:u32;asm(r1<-x,r1->y){ADD r1,r1,r0,0}}\n",
        "r1->y",
        Some("r1"),
        &[&["duplicate", "repeated"], &["register"]],
    );
}

#[test]
fn asm_r02_duplicate_output() {
    reject(
        "ASM-R02.duplicate-output",
        "ASM-02",
        "package main\nfunc main(){var x:u32;asm(r1->x,r2->x){ADD r1,r0,r0,0;ADD r2,r0,r0,0}}\n",
        "r2->x",
        Some("x"),
        &[&["duplicate", "repeated"], &["output"]],
    );
}

#[test]
fn asm_r03_invalid_output_type() {
    reject(
        "ASM-R03.invalid-output-type",
        "ASM-02",
        "package main\nfunc main(){var x:[1]u32;asm(r1->x){ADD r1,r0,r0,0}}\n",
        "r1->x",
        Some("x"),
        &[&["scalar", "aggregate", "binding type"]],
    );
}

#[test]
fn asm_r04_uninitialized_scratch() {
    reject(
        "ASM-R04.uninitialized-scratch",
        "ASM-03",
        "package main\nfunc main(){var x:u32;asm(r1<-x){ADD r1,r2,r0,0}}\n",
        "ADD r1,r2,r0,0",
        Some("r2"),
        &[&[
            "undefined",
            "uninitialized",
            "not defined",
            "before definition",
        ]],
    );
}

#[test]
fn asm_r05_output_branch_path() {
    reject(
        "ASM-R05.output-branch-path",
        "ASM-03",
        "package main\nfunc main(){var x:u32;var condition:u32=0;asm(r2<-condition,r1->x){JZ r0,r2,done;ADD r1,r0,r0,1;done:}}\n",
        "r1->x",
        Some("r1"),
        &[
            &["output"],
            &["undefined", "uninitialized", "not defined", "every", "all"],
        ],
    );
}

#[test]
fn asm_r06_normal_stack_register() {
    reject(
        "ASM-R06.normal-stack-register",
        "ASM-03",
        "package main\nfunc main(){asm{ADD r1,r14,r0,0}}\n",
        "ADD r1,r14,r0,0",
        Some("r14"),
        &[
            &["r14", "stack pointer"],
            &["normal", "ordinary", "reserved", "forbidden", "cannot"],
        ],
    );
}

#[test]
fn asm_r07_normal_external_jump() {
    reject(
        "ASM-R07.normal-external-jump",
        "ASM-03",
        "package main\nfunc main(){var target:u32=0;asm(r1<-target){JZ r0,r0,r1,0}}\n",
        "JZ r0,r0,r1,0",
        Some("JZ"),
        &[
            &["external", "indirect"],
            &["normal", "ordinary", "noreturn"],
        ],
    );
}

#[test]
fn asm_r08_noreturn_fallthrough() {
    reject(
        "ASM-R08.noreturn-fallthrough",
        "ASM-04",
        "package main\nfunc main(){asm noreturn{ADD r1,r0,r0,1}}\n",
        "{ADD r1,r0,r0,1}",
        Some("}"),
        &[
            &["noreturn"],
            &["fallthrough", "fall through", "exit", "end"],
        ],
    );
}

#[test]
fn asm_r09_invalid_shift_immediate() {
    reject(
        "ASM-R09.invalid-shift-immediate",
        "ASM-03",
        "package main\nfunc main(){asm{SHIFT r1,r0,r0,32}}\n",
        "SHIFT r1,r0,r0,32",
        Some("32"),
        &[&["shift"], &["immediate", "encoding", "reserved", "range"]],
    );
}

#[test]
fn asm_r10_invalid_compare_immediate() {
    reject(
        "ASM-R10.invalid-compare-immediate",
        "ASM-03",
        "package main\nfunc main(){asm{CMP r1,r0,r0,3}}\n",
        "CMP r1,r0,r0,3",
        Some("3"),
        &[
            &["cmp", "comparison", "compare"],
            &["immediate", "encoding", "reserved", "range"],
        ],
    );
}

#[test]
fn asm_r11_noreturn_output_binding() {
    reject(
        "ASM-R11.noreturn-output-binding",
        "ASM-04",
        "package main\nfunc main(){var x:u32;asm noreturn(r1->x){ADD r1,r0,r0,1;JZ r0,r0,r1,0}}\n",
        "r1->x",
        None,
        &[&["noreturn"], &["output"]],
    );
}

#[test]
fn asm_context_keeps_ordinary_identifiers_and_comparison_tokens() {
    compile("package main\nfunc asm(){}\nfunc main(){asm();var noreturn:u32;var r1:i32=1;var b:i32=2;var value:bool;asm(r2<-r1<-b,r3->value){ADD r3,r2,r0,0};_=noreturn;_=value}\n", None).unwrap();
}

#[test]
fn asm_rejects_noncanonical_external_zero_and_multiline_instruction() {
    for immediate in ["+0", "-0", "0x0"] {
        let source = format!(
            "package main\nfunc main(){{asm noreturn(r1<-0){{JZ r0,r0,r1,{immediate}}}}}\n"
        );
        assert!(
            compile(&source, None).is_err(),
            "external JZ requires literal 0: {immediate}"
        );
    }
    assert!(
        compile("package main\nfunc main(){asm{ADD r1,\nr0,r0,1}}\n", None).is_err(),
        "source newline separates asm items"
    );
}

#[test]
fn asm_unused_generic_body_still_checks_type_independent_constraints() {
    let source = "package main\nfunc unused[T](){asm{ADD r1,r2,r0,0}}\nfunc main(){}\n";
    assert!(
        compile(source, None).is_err(),
        "unused generic must reject an undefined scratch register"
    );
}

#[test]
fn asm_rejects_register_label_output_and_initialization_boundaries() {
    for statement in [
        "asm(r0<-1){}",
        "asm(r15<-1){}",
        "asm{ADD r15,r0,r0,1}",
        "asm(r1->values[0]){ADD r1,r0,r0,1}",
        "asm(r1->*pointer){ADD r1,r0,r0,1}",
        "asm(r1->_){ADD r1,r0,r0,1}",
        "asm(r1<-values){}",
        "asm{JZ r0,r0,missing}",
        "asm{same:;same:}",
        "asm{JZ r0,r0,read;write:;ADD r1,r0,r0,1;read:;ADD r2,r1,r0,0;JZ r0,r0,write}",
        "asm noreturn{ADD r1,r15,r0,0;JZ r0,r0,r1,0}",
    ] {
        let source = format!(
            "package main\nfunc main(){{var values:[1]u32;var pointer:*u32;{statement}}}\n"
        );
        assert!(compile(&source, None).is_err(), "{statement}");
    }
}

#[test]
fn asm_branch_range_has_no_implicit_thunk() {
    let source = format!(
        "package main\nfunc main(){{asm{{JZ r0,r0,end;{}end:}}}}\n",
        "ADD r1,r0,r0,0;".repeat(32768)
    );
    let error = compile(&source, None).expect_err("offset 32768 must be rejected");
    assert!(
        error
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.message.contains("branch")
                && diagnostic.message.contains("range")),
        "{error:?}"
    );
}

#[test]
fn asm_mir_validator_rejects_malformed_registers_values_results_and_cfg() {
    use crate::{
        inline_asm::{Operation, Register},
        mir,
    };
    let project = compile(
        "package main\nfunc main(){var x:u32=1;asm(r1<->x){ADD r1,r1,r0,1}}\n",
        None,
    )
    .unwrap()
    .mir;
    for mutation in 0..9 {
        let mut changed = project.clone();
        let instruction = changed
            .packages
            .iter_mut()
            .flat_map(|package| &mut package.functions)
            .flat_map(|function| &mut function.blocks)
            .flat_map(|block| &mut block.instructions)
            .find(|instruction| {
                matches!(
                    instruction.kind,
                    mir::InstructionKind::InlineAssembly { .. }
                )
            })
            .unwrap();
        let mir::InstructionKind::InlineAssembly {
            inputs,
            outputs,
            program,
            noreturn,
        } = &mut instruction.kind
        else {
            unreachable!()
        };
        match mutation {
            0 => inputs[0].0.number = 16,
            1 => inputs[0].1 = mir::ValueId(u32::MAX),
            2 => {
                instruction.results.clear();
            }
            3 => outputs[0].1 = mir::TypeId(u32::MAX),
            4 => inputs.push(inputs[0].clone()),
            5 => *noreturn = true,
            6 => {
                program.instructions[0].operation = Operation::LocalBranch {
                    condition: Register {
                        number: 0,
                        span: instruction.span,
                    },
                    target: "missing".into(),
                    target_span: instruction.span,
                }
            }
            7 => {
                let Operation::Machine { opcode, .. } = &mut program.instructions[0].operation
                else {
                    unreachable!()
                };
                *opcode = 10;
            }
            8 => {
                let Operation::Machine { rs1, .. } = &mut program.instructions[0].operation else {
                    unreachable!()
                };
                rs1.number = 15;
            }
            _ => unreachable!(),
        }
        assert!(
            mir::validate_project(&changed).is_err(),
            "malformed asm mutation {mutation}"
        );
    }
}

#[test]
fn asm_comments_empty_forms_and_noreturn_flow_are_accepted() {
    compile("package main\nfunc end()->u32{asm noreturn{loop:\nJZ r0,r0,loop}}\nfunc main(){asm{};asm(){};asm{ADD r1,r0,r0,1/*\n*/ADD r2,r0,r0,2// line\nADD r3,r0,r0,3};_=end}\n", None).unwrap();
    assert!(
        compile(
            "package main\nfunc need()->u32{asm{loop:;JZ r0,r0,loop}}\nfunc main(){}\n",
            None
        )
        .is_err(),
        "ordinary asm keeps normal Ond continuation even if its body loops"
    );
}

#[test]
fn asm_collects_independent_binding_name_errors() {
    let error = compile(
        "package main\nfunc main(){asm(r1<-missingA,r2<-missingB){}}\n",
        None,
    )
    .expect_err("two unknown input expressions");
    for name in ["missingA", "missingB"] {
        assert!(
            error
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.message.contains(name)),
            "missing independent diagnostic for {name}: {error:?}"
        );
    }
}

#[test]
fn asm_reports_each_duplicate_binding_cause_once() {
    for bindings in ["r1<-1,r1<-2", "r1->x,r1->y", "r1<->x,r1<->y"] {
        let source = format!(
            "package main\nfunc main(){{var x:u32;var y:u32;asm({bindings}){{ADD r1,r0,r0,0}}}}\n"
        );
        let error = compile(&source, None).expect_err("duplicate register binding");
        assert_eq!(
            error.diagnostics().len(),
            1,
            "one cause must not gain a generic recovery error: {error:?}"
        );
        assert_eq!(
            error
                .diagnostics()
                .iter()
                .filter(|diagnostic| diagnostic.message == "duplicate asm register binding")
                .count(),
            1,
            "one diagnostic per duplicate cause: {error:?}"
        );
    }
}
