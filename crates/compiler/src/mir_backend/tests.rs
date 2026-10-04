//! Actual Kagura CPU + linked images, never a container format or MIR VM.
use super::*;
use kagura::{Bus, Cpu, DefaultBus};
use ram::Ram;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

pub(super) struct Fixture(pub PathBuf);
impl Fixture {
    pub(super) fn new(source: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "ond-mir-backend-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::write(
            path.join("ond.toml"),
            "[target.kagura]\nram = 65536\nstack = 8192\n",
        )
        .unwrap();
        fs::write(path.join("main.ond"), source).unwrap();
        Self(path)
    }
    pub(super) fn core(&self) -> mir::Project {
        ond_compiler_core::compile_project(&self.0).unwrap().mir
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
const OUTPUT: u32 = 0xffff8050;
const EXIT: u32 = 0xffff8040;
pub(super) fn machine(bytes: &crate::linker::LinkedImage) -> (Cpu, DefaultBus, u32) {
    let executable = bytes;
    let mut bus = DefaultBus::new();
    bus.map_device(
        0x1000,
        executable.ram_size,
        Ram::new(executable.ram_size as usize),
    )
    .unwrap();
    bus.map_device(EXIT, 4, Ram::new(4)).unwrap();
    bus.map_device(OUTPUT, 4, Ram::new(4)).unwrap();
    bus.write32(EXIT, 0xdeadbeef).unwrap();
    for section in &executable.sections {
        // Mirror the machine loader contract explicitly, not Ram::new's zero fill.
        if section.kind == crate::ir::object::SectionKind::Bss {
            for offset in 0..section.memory_size {
                bus.write8(section.load_address + offset, 0xa5).unwrap();
            }
            for offset in 0..section.memory_size {
                bus.write8(section.load_address + offset, 0).unwrap();
            }
        }
        for (i, byte) in section.data.iter().enumerate() {
            bus.write8(section.load_address + i as u32, *byte).unwrap();
        }
    }
    let top = 0x1000 + executable.ram_size;
    let mut cpu = Cpu::new();
    cpu.set_pc(executable.entry_point);
    cpu.set_reg(14, top);
    for r in 7..=11 {
        cpu.set_reg(r, 0xabcd0000 + r as u32);
    }
    (cpu, bus, top)
}
pub(super) fn execute(bytes: &crate::linker::LinkedImage) -> u32 {
    execute_steps(bytes, 200_000)
}
pub(super) fn execute_steps(bytes: &crate::linker::LinkedImage, limit: usize) -> u32 {
    let (mut cpu, mut bus, top) = machine(bytes);
    for _ in 0..limit {
        cpu.step(&mut bus).unwrap();
        if bus.read32(EXIT).unwrap() != 0xdeadbeef {
            assert_eq!(bus.read32(EXIT).unwrap(), 0);
            assert_eq!(cpu.reg(14), top, "unbalanced stack");
            for r in 7..=11 {
                assert_eq!(cpu.reg(r), 0xabcd0000 + r as u32, "callee save r{r}");
            }
            return bus.read32(OUTPUT).unwrap();
        }
    }
    panic!("CPU step limit");
}
pub(super) fn run(declarations: &str, expression: &str, expected: u32) {
    let source = format!(
        "package main\n{declarations}\nfunc main() {{ store32(4294934608, ({expression}) as u32) }}\n"
    );
    let fixture = Fixture::new(&source);
    let bytes = crate::test_support::compile_image(&fixture.0)
        .unwrap_or_else(|e| panic!("{source}\n{e:?}"));
    assert_eq!(execute(&bytes), expected, "{source}");
}

#[test]
fn interface_dispatch_and_identity_execute() {
    run(
        "type Reader interface { Read(value: i32) -> i32; }\ntype Device struct { bias: i32; }\ntype Holder struct { reader: Reader; }\nfunc (device: *Device) Read(value: i32) -> i32 { return device.bias + value; }\nfunc pass(reader: Reader) -> Reader { return reader; }\nfunc call(reader: Reader, value: i32) -> i32 { return reader.Read(value); }\nfunc calc() -> i32 { var device = Device { bias: 30 }; var reader = (&device) as Reader; if reader == nil { return 1; }; if reader != ((&device) as Reader) { return 2; }; var holder = Holder { reader: pass(reader) }; return call(holder.reader, 12); }",
        "calc()",
        42,
    );
    run(
        "type NilCheck interface { IsNil() -> bool; }\ntype A struct {}\ntype B struct {}\nfunc (value: *A) IsNil() -> bool { return value == nil; }\nfunc (value: *B) IsNil() -> bool { return value == nil; }\nfunc calc() -> i32 { var a: *A; var b: *B; var first = a as NilCheck; var second = b as NilCheck; if first != nil { return 1; }; if first != second { return 2; }; if first.IsNil() { return 42; }; return 3; }",
        "calc()",
        42,
    );
}

#[test]
fn methods_preserve_exact_receivers_nil_and_evaluation_order() {
    run(
        "type S struct { value: i32; }\nvar Instance: S\nvar Trace: i32\nfunc make() -> *S { Trace = Trace * 10 + 1; return &Instance; }\nfunc argument() -> i32 { Trace = Trace * 10 + 2; return 7; }\nfunc (value: *S) Set(next: i32) -> i32 { Trace = Trace * 10 + 3; value.value = next; return Trace; }",
        "make().Set(argument())",
        123,
    );
    run(
        "type S struct { value: i32; }\nfunc (value: S) Get() -> i32 { return value.value; }",
        "(S { value: 41 }).Get() + 1",
        42,
    );
    run(
        "type S struct {}\nfunc (value: *S) IsNil() -> bool { return value == nil; }\nfunc calc() -> i32 { var value: *S; if value.IsNil() { return 1; }; return 0; }",
        "calc()",
        1,
    );
}

#[test]
fn exported_methods_on_an_inferred_private_type_work_across_packages() {
    let fixture = Fixture::new(
        "package main\nimport \"shared\"\ntype Reader interface { Read() -> i32; }\nfunc main() { var direct = shared.New().Read(); var dynamic = (shared.New() as Reader).Read(); store32(4294934608, (direct + dynamic) as u32); }\n",
    );
    fs::create_dir(fixture.0.join("shared")).unwrap();
    fs::write(
        fixture.0.join("shared/shared.ond"),
        "package shared\ntype device struct { value: i32; }\nfunc New() -> *device { var value = new(device); value.value = 21; return value; }\nfunc (value: *device) Read() -> i32 { return value.value; }\n",
    )
    .unwrap();
    let image = crate::test_support::compile_image(&fixture.0).unwrap();
    assert_eq!(execute(&image), 42);
}

#[test]
fn scalar_layout_and_abi_are_independent_of_stack_slots() {
    let types = mir::TypeTable::new();
    for (ty, size, signed) in [
        (mir::TypeId::BOOL, 1, false),
        (mir::TypeId::U8, 1, false),
        (mir::TypeId::I8, 1, true),
        (mir::TypeId::U16, 2, false),
        (mir::TypeId::I16, 2, true),
        (mir::TypeId::U32, 4, false),
        (mir::TypeId::I32, 4, true),
    ] {
        let l = layout::scalar(&types, ty, Span::synthetic()).unwrap();
        assert_eq!((l.size, l.alignment, l.signed), (size, size, signed));
    }
    for n in [0, 1, 6, 7, 8, 9] {
        let plan = layout::abi(
            &types,
            &mir::FunctionType {
                parameters: vec![mir::TypeId::I8; n],
                returns: vec![mir::TypeId::BOOL],
            },
            Span::synthetic(),
        )
        .unwrap();
        assert_eq!(
            plan.stack_bytes,
            match n {
                0..=6 => 0,
                7..=8 => 8,
                _ => 16,
            }
        );
        assert_eq!(plan.result, Some(1));
        if n > 6 {
            assert_eq!(plan.arguments[6], layout::Argument::Stack(0));
        }
    }
}
#[test]
fn locals_branches_loops_and_boolean_block_parameters_execute() {
    run(
        "func calc(n: i32) -> i32 { var sum: i32; for i := 0; i < n; i = i + 1 { if i == 2 { continue }; if i == 6 { break }; sum = sum + i }; return sum }",
        "calc(10)",
        13,
    );
    run(
        "func flag(x: bool, y: bool) -> bool { return x && (y || !x) }\nfunc calc() -> i32 { if flag(true, false) { return 9 }; if flag(true, true) { return 7 }; return 8 }",
        "calc()",
        7,
    );
    // Skipped RHS would fault: verifies actual branch execution, not merely result bits.
    run(
        "func bad() -> bool { store32(0, 1); return true }\nfunc calc(x: bool) -> i32 { if x && bad() { return 9 }; if !x || bad() { return 3 }; return 8 }",
        "calc(false)",
        3,
    );
}
#[test]
fn direct_calls_recursion_stack_arguments_and_spills_execute() {
    run(
        "func fact(n: i32) -> i32 { if n < 2 { return 1 }; return n * fact(n - 1) }",
        "fact(6)",
        720,
    );
    run(
        "func sum(a:i32,b:i32,c:i32,d:i32,e:i32,f:i32,g:i8,h:u16,i:bool) -> i32 { if i { return a+b+c+d+e+f+(g as i32)+(h as i32) }; return 0 }\nfunc nested(x:i32) -> i32 { return sum(x,2,3,4,5,6,-7,500,true)+sum(1,2,3,4,5,6,7,8,false) }",
        "nested(1)",
        514,
    );
    run(
        "func set(v:u32) { store32(4294934608,v) }\nfunc pair(x:i32) -> i32 { set(99); return x+1 }",
        "pair(4)+pair(6)",
        12,
    );
}
#[test]
fn all_integer_widths_wrap_sign_extend_and_shift_modulo() {
    for (ty, max, min, bits) in [
        ("u8", "255", "0", 8),
        ("i8", "127", "-128", 8),
        ("u16", "65535", "0", 16),
        ("i16", "32767", "-32768", 16),
        ("u32", "4294967295", "0", 32),
        ("i32", "2147483647", "-2147483648", 32),
    ] {
        let mask = if bits == 32 {
            u32::MAX
        } else {
            (1u32 << bits) - 1
        };
        let expected = if ty.starts_with('i') {
            (1u32 << (bits - 1)).wrapping_sub(1) | !mask
        } else {
            u32::MAX
        };
        run(
            &format!("func calc(x:{ty}) -> {ty} {{ return x - 1 }}"),
            &format!("calc({min})"),
            if ty.starts_with('i') {
                expected & mask
            } else {
                mask
            },
        );
        run(
            &format!("func calc(x:{ty}, k:u32) -> {ty} {{ return x << k }}"),
            &format!("calc(1, {bits})"),
            1,
        );
        run(
            &format!(
                "func calc(x:{ty}) -> bool {{ return x > 0 }}\nfunc choose(x:{ty}) -> i32 {{ if calc(x) {{ return 1 }}; return 0 }}"
            ),
            &format!("choose({max})"),
            1,
        );
    }
    run(
        "func calc(x:i8) -> i32 { return x as i32 }",
        "calc(-1)",
        u32::MAX,
    );
    run(
        "func calc(x:i16,k:u32) -> i16 { return x >> k }",
        "calc(-128, 17)",
        (-64i32) as u32,
    );
    run(
        "func calc(x:u32) -> u8 { return x as u8 }",
        "calc(511)",
        255,
    );
}
#[test]
fn integer_operations_are_selected_not_constant_folded() {
    for (op, want) in [
        ("+", 17),
        ("-", 7),
        ("*", 60),
        ("&", 4),
        ("|", 13),
        ("^", 9),
        ("&^", 8),
    ] {
        run(
            &format!("func calc(a:u32,b:u32) -> u32 {{ return a {op} b }}"),
            "calc(12,5)",
            want,
        );
    }
    for (op, want) in [
        ("==", 0),
        ("!=", 1),
        ("<", 1),
        ("<=", 1),
        (">", 0),
        (">=", 0),
    ] {
        run(
            &format!("func calc(a:i8,b:i8) -> i32 {{ if a {op} b {{ return 1 }}; return 0 }}"),
            "calc(-1,1)",
            want,
        );
    }
    run("func calc(x:u8) -> u8 { return ^x }", "calc(240)", 15);
    run(
        "func calc(x:i8) -> i8 { return -x }",
        "calc(-128)",
        (-128i32) as u32,
    );
}
#[test]
fn unsupported_checked_operations_never_silently_wrap() {
    let fixture = Fixture::new("package main\nfunc calc(x:i32)->i32{return x+1}\nfunc main(){}\n");
    let mut p = fixture.core();
    let f = p.packages[0]
        .functions
        .iter_mut()
        .find(|f| f.name == "main.calc")
        .unwrap();
    for i in f.blocks.iter_mut().flat_map(|b| &mut b.instructions) {
        if let mir::InstructionKind::Binary { overflow, .. } = &mut i.kind {
            *overflow = mir::OverflowBehavior::Checked;
        }
    }
    mir::validate_project(&p).unwrap();
    let failure = codegen_objects(&p).unwrap_err();
    assert!(
        failure.diagnostics()[0]
            .message
            .contains("unsupported checked binary operation")
    );
    assert!(!failure.diagnostics()[0].primary.span.is_synthetic());
}
#[test]
fn recursive_stack_overflow_faults_before_writing_below_stack() {
    let fixture = Fixture::new(
        "package main\nfunc recurse(x:i32) -> i32 { return recurse(x+1) }\nfunc main() { recurse(1) }\n",
    );
    let bytes = crate::test_support::compile_image(&fixture.0).unwrap();
    let (mut cpu, mut bus, top) = machine(&bytes);
    let bottom = top - 8192;
    bus.write32(bottom - 4, 0xfeedcafe).unwrap();
    let mut faulted = false;
    for _ in 0..100_000 {
        if let Err(fault) = cpu.step(&mut bus) {
            assert!(
                format!("{fault:?}").contains("InvalidInstruction"),
                "{fault:?}"
            );
            faulted = true;
            break;
        }
    }
    assert!(faulted);
    assert!(cpu.reg(14) >= bottom);
    assert_eq!(bus.read32(bottom - 4).unwrap(), 0xfeedcafe);
}

#[test]
fn large_scalar_frame_is_accepted() {
    let fixture = Fixture::new("package main\nfunc main() { var x:i32; x = x+1 }\n");
    let mut project = fixture.core();
    let f = &mut project.packages[0].functions[0];
    let template = f.locals[0].clone();
    while f.locals.len() < 9000 {
        let mut local = template.clone();
        local.id = mir::LocalId(f.locals.len() as u32);
        local.name = None;
        f.locals.push(local);
    }
    mir::validate_project(&project).unwrap();
    codegen_objects(&project).unwrap();
}

#[test]
fn local_branch_boundaries_are_expanded_in_both_directions() {
    for (delta, near) in [
        (32767i32, true),
        (32768, false),
        (-32768, true),
        (-32769, false),
    ] {
        let mut e = emit::Emitter::default();
        let target = e.label();
        if delta >= 0 {
            e.branch(0, target, Span::synthetic());
            e.words.resize(delta as usize + 1, 0);
            e.bind(target);
        } else {
            e.bind(target);
            e.words.resize((-delta - 1) as usize, 0);
            e.branch(0, target, Span::synthetic());
        }
        let at = if delta >= 0 { 0 } else { (-delta - 1) as usize };
        let (bytes, _) = e.finish().unwrap();
        let first = u32::from_le_bytes(bytes[at * 4..at * 4 + 4].try_into().unwrap());
        assert_eq!(first as u16 as i16, if near { delta as i16 } else { 1 });
    }
}
