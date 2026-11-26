use super::{
    local_aggregate::LocalAggregates,
    tests::{Fixture, machine},
    *,
};
use kagura::{Bus, FaultCode};
use ond_protocol::debug::DebugOperation;
use ram::Ram;

const INPUT: u32 = 0x2000_0000;
const OUTPUT: u32 = INPUT + 0x100;
const END: u32 = 0xffff_8040;
const SENTINEL: u32 = 0xdead_beef;

fn eligible(project: &mir::Project, name: &str) -> bool {
    let f = project.packages[0]
        .functions
        .iter()
        .find(|f| f.name == format!("main.{name}"))
        .unwrap();
    let signature = mir::FunctionType {
        parameters: f
            .parameters
            .iter()
            .map(|id| f.locals[id.0 as usize].ty)
            .collect(),
        returns: f.returns.clone(),
    };
    let abi = layout::abi(&project.types, &signature, f.span).unwrap();
    LocalAggregates::new(&project.types, f, &abi).is_some()
}

#[test]
fn private_aggregate_copies_keep_snapshots_and_exact_stack_boundary() {
    let fixture = Fixture::new(include_str!("local_aggregate_fixture.ond"));
    let p = fixture.core();
    assert!(eligible(&p, "structs"));
    assert!(eligible(&p, "arrays"));
    let output = codegen_raw(&p).unwrap();
    let mut objects = vec![crate::test_support::startup_object(&[], "main.main")];
    objects.extend(output.objects);
    // The original frame reservations leave exactly 576 bytes at the second call.
    for (stack_size, fits) in [(688, true), (680, false)] {
        let linked = crate::linker::link_objects_with_debug(
            &objects,
            &output.debug,
            &crate::linker::LinkPlan {
                read_only: None,
                memory_base: 0x1000,
                memory_size: 65536,
                stack_size,
                entry_symbol: "__ond.entry".into(),
                heap_base_symbol: "__ond_heap_base".into(),
                stack_bottom_symbol: "__ond_stack_bottom".into(),
            },
        )
        .unwrap();
        for (name, expected) in [
            ("main.structs", 544),
            ("main.arrays", 576),
            ("main.main", 112),
        ] {
            let f = linked
                .debug
                .functions
                .iter()
                .find(|f| f.symbol == name)
                .unwrap();
            assert_eq!(f.frame_size, expected, "{name}");
        }
        let (mut cpu, mut bus, top) = machine(&linked.image);
        bus.map_device(INPUT, 512, Ram::new(512)).unwrap();
        bus.write32(INPUT, 19).unwrap();
        bus.write32(OUTPUT, SENTINEL).unwrap();
        bus.write32(OUTPUT + 4, SENTINEL).unwrap();
        let bottom = linked.debug.stack_bottom;
        bus.write32(bottom - 4, SENTINEL).unwrap();
        let mut low_sp = top;
        let mut completed = false;
        for _ in 0..20_000 {
            if let Err(fault) = cpu.step(&mut bus) {
                assert!(!fits);
                assert_eq!(fault.code(), FaultCode::InvalidInstruction);
                let pc = fault.faulting_pc();
                assert!(linked.debug.ranges.iter().any(|r| {
                    r.start <= pc
                        && pc < r.end
                        && r.operation == DebugOperation::StackOverflow
                        && r.function == "main.arrays"
                }));
                assert_eq!(cpu.reg(14), top - 112);
                completed = true;
                break;
            }
            low_sp = low_sp.min(cpu.reg(14));
            if bus.read32(END).unwrap() == 0 {
                assert!(fits);
                assert_eq!(low_sp, bottom);
                assert_eq!(cpu.reg(14), top);
                completed = true;
                break;
            }
        }
        assert!(completed);
        assert_eq!(bus.read32(OUTPUT).unwrap(), 24);
        assert_eq!(
            bus.read32(OUTPUT + 4).unwrap(),
            if fits { 43 } else { SENTINEL }
        );
        assert_eq!(bus.read32(bottom - 4).unwrap(), SENTINEL);
    }
}

#[test]
fn private_aggregate_preserves_zero_narrow_and_f32_bits() {
    let fixture = Fixture::new(
        r#"package main
type Code u8
type Mixed struct { code: Code; small: i8; medium: i16; bits: f32; }
func mixed(input: f32) -> (f32, f32, u32) {
    var zero: Mixed
    var a = Mixed{code: 255 as Code, small: -128, medium: -32768, bits: input}
    var b = a
    b = b
    return b.bits, zero.bits, (b.code as u32) + ((b.small as i32) + (b.medium as i32) + 32896) as u32
}
func main() {
    var bits, zero, code = mixed(*(536870912 as *f32))
    *(536871168 as *f32) = bits
    *(536871172 as *f32) = zero
    *(536871176 as *u32) = code
}"#,
    );
    assert!(eligible(&fixture.core(), "mixed"));
    let image = crate::test_support::compile_image(&fixture.0).unwrap();
    for bits in [0x8000_0000, 0x7fc1_2345] {
        let (mut cpu, mut bus, _) = machine(&image);
        bus.map_device(INPUT, 512, Ram::new(512)).unwrap();
        bus.write32(INPUT, bits).unwrap();
        let completed = (0..20_000).any(|_| {
            cpu.step(&mut bus).unwrap();
            bus.read32(END).unwrap() == 0
        });
        assert!(completed);
        assert_eq!(bus.read32(OUTPUT).unwrap(), bits);
        assert_eq!(bus.read32(OUTPUT + 4).unwrap(), 0);
        assert_eq!(bus.read32(OUTPUT + 8).unwrap(), 255);
    }
}

#[test]
fn private_aggregate_screen_excludes_other_observations() {
    let fixture = Fixture::new(
        r#"package main
type Pair struct { x: u32; y: u32; }
func address() { var p: Pair; var ptr = &p; _ = ptr }
func raw(p: *u32) { var a: Pair; a.x = *p; _ = a.x }
func assembly() { var p: Pair; asm () { ADD r1, r0, r0, 0 }; _ = p }
func dynamic(i: u32) -> u32 { var a: [2]u32; return a[i] }
func branch(flag: bool) -> u32 { var p: Pair; if flag { p.x = 1 }; return p.x }
func call() -> u32 { var p: Pair; p.x = dynamic(0); return p.x }
func helper(x: u32) -> u32 { var p: Pair; p.x = x / 2; return p.x }
func indirect(p: Pair) -> Pair { var q = p; return q }
func wide(n: u32) -> u8 { var a: [5]u8; a[0] = n as u8; return a[0] }
func main() {}"#,
    );
    let p = fixture.core();
    for name in [
        "address", "raw", "assembly", "dynamic", "branch", "call", "helper", "indirect", "wide",
    ] {
        assert!(!eligible(&p, name), "{name}");
    }
}

#[test]
fn private_aggregate_snapshot_registers_survive_large_frame_accesses() {
    let fixture = Fixture::new(include_str!("local_aggregate_fixture.ond"));
    let mut p = fixture.core();
    let f = p.packages[0]
        .functions
        .iter_mut()
        .find(|f| f.name == "main.structs")
        .unwrap();
    let ty = p.types.intern(
        mir::TypeKind::Array {
            length: 8192,
            element: mir::TypeId::U32,
        },
        f.span,
    );
    // Keep the original declarations and operations. Shift only the ValueId and scratch slots.
    f.locals.push(mir::Local {
        id: mir::LocalId(f.locals.len() as u32),
        name: None,
        ty,
        kind: mir::LocalKind::Temporary,
        address_taken: false,
        span: f.span,
    });
    assert!(eligible(&p, "structs"));
    let image = crate::test_support::build_image(
        &p,
        crate::ProjectConfig {
            ram_size: 131072,
            stack_size: 65536,
        },
    )
    .unwrap();
    let (mut cpu, mut bus, top) = machine(&image);
    cpu.set_pc(image.symbols["main.structs"]);
    cpu.set_reg(1, 19);
    cpu.set_reg(15, 0x2000_2000);
    let returned = (0..20_000).any(|_| {
        cpu.step(&mut bus).unwrap();
        cpu.pc() == 0x2000_2000
    });
    assert!(returned);
    assert_eq!(cpu.reg(1), 24);
    assert_eq!(cpu.reg(14), top);
}
