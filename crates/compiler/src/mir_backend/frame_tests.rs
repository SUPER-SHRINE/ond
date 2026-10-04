use super::{
    tests::{Fixture, execute, execute_steps, machine},
    *,
};
use kagura::{Bus, Cpu, DefaultBus};
use ram::Ram;

fn enlarge(project: &mut mir::Project) {
    for f in project.packages.iter_mut().flat_map(|p| &mut p.functions) {
        let template = f
            .locals
            .first()
            .expect("test functions have locals")
            .clone();
        while f.locals.len() < 10000 {
            let mut local = template.clone();
            local.id = mir::LocalId(f.locals.len() as u32);
            local.name = None;
            local.kind = mir::LocalKind::Temporary;
            local.ty = mir::TypeId::U32;
            f.locals.push(local);
        }
    }
    mir::validate_project(project).unwrap();
}

#[test]
fn large_frames_preserve_stack_arguments_results_snapshots_and_indirect_calls() {
    let fixture = Fixture::new(
        "package main\n
        type S struct {a:u32;b:u32;}
        func pair(a:u32,b:u32,c:u32,d:u32,e:u32,f:u32,g:u32)->(u32,u32){return a+b+c+d+e+f+g,g}
        func copy(s:S)->S {s.a=s.a+1;return s}
        func main(){var s=S{a:10,b:20};var p=pair;var a,b=p(1,2,3,4,5,6,7);
            var t=copy(s);store32(4294934608,a+b+s.a+t.a+t.b)}",
    );
    let mut project = fixture.core();
    enlarge(&mut project);
    let bytes = crate::test_support::build_image(
        &project,
        crate::ProjectConfig {
            ram_size: 1024 * 1024,
            stack_size: 262144,
        },
    )
    .unwrap();
    assert_eq!(execute(&bytes), 76);
}

#[test]
fn large_frame_stack_exhaustion_faults_before_writes_or_sp_update() {
    let fixture = Fixture::new("package main\nfunc main(){var x:u32;x=1}");
    let mut project = fixture.core();
    enlarge(&mut project);
    let bytes = crate::test_support::build_image(
        &project,
        crate::ProjectConfig {
            ram_size: 262144,
            stack_size: 8192,
        },
    )
    .unwrap();
    let (mut cpu, mut bus, top) = machine(&bytes);
    bus.write32(top - 40008, 0x12345678).unwrap();
    bus.write32(top - 4, 0xabcdef01).unwrap();
    let mut trapped = false;
    for _ in 0..200 {
        if let Err(e) = cpu.step(&mut bus) {
            assert!(format!("{e:?}").contains("InvalidInstruction"));
            trapped = true;
            break;
        }
    }
    assert!(trapped);
    assert_eq!(cpu.reg(14), top);
    assert_eq!(bus.read32(top - 40008).unwrap(), 0x12345678);
    assert_eq!(bus.read32(top - 4).unwrap(), 0xabcdef01);
}

#[test]
fn source_large_array_addresses_and_high_spills_execute() {
    let fixture = Fixture::new(
        "package main\nfunc main(){var x:[9000]u32;
        x[8999]=41;var p=&x[8999];*p=*p+1;store32(4294934608,*p)}",
    );
    let bytes = crate::test_support::build_image(
        &fixture.core(),
        crate::ProjectConfig {
            ram_size: 8 * 1024 * 1024,
            stack_size: 1024 * 1024,
        },
    )
    .unwrap();
    assert_eq!(execute_steps(&bytes, 2_000_000), 42);
}

#[test]
fn frame_arithmetic_overflow_remains_a_diagnostic() {
    let fixture = Fixture::new("package main\nfunc main(){var x:[536870912]u32}");
    let failure = codegen_objects(&fixture.core()).unwrap_err();
    assert!(
        failure.diagnostics()[0].message.contains("frame overflow"),
        "{failure:?}"
    );
}

#[test]
fn stack_access_and_address_boundaries_preserve_data_registers() {
    for offset in [32764, 32768, 65536, 131072] {
        for reg in [1, 6, 12, 13, 15] {
            let mut e = emit::Emitter::default();
            e.store(reg, offset);
            e.load(reg, offset);
            e.stack_address(5, offset);
            let count = e.words.len();
            let (bytes, _) = e.finish().unwrap();
            let mut bus = DefaultBus::new();
            bus.map_device(0x1000, 262144, Ram::new(262144)).unwrap();
            for (i, b) in bytes.iter().enumerate() {
                bus.write8(0x1000 + i as u32, *b).unwrap();
            }
            let mut cpu = Cpu::new();
            cpu.set_pc(0x1000);
            cpu.set_reg(14, 0x2000);
            for r in 1..=15 {
                if r != 14 {
                    cpu.set_reg(r, 0xabcdef00 + r as u32);
                }
            }
            for _ in 0..count {
                cpu.step(&mut bus).unwrap();
            }
            assert_eq!(cpu.reg(reg as usize), 0xabcdef00 + reg as u32);
            assert_eq!(
                bus.read32(0x2000 + offset).unwrap(),
                0xabcdef00 + reg as u32
            );
            assert_eq!(cpu.reg(5), 0x2000 + offset);
            assert_eq!(cpu.reg(14), 0x2000);
            for r in 1..=15 {
                if [5, 12, 13, 14].contains(&r) {
                    continue;
                }
                assert_eq!(cpu.reg(r), 0xabcdef00 + r as u32);
            }
        }
    }
}
