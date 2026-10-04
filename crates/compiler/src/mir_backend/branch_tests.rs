//! Long branches are executed by the real CPU, not interpreted by a test MIR VM.
use super::{
    tests::{Fixture, execute, execute_steps, run},
    *,
};
use kagura::{Bus, Cpu, DefaultBus};
use ram::Ram;

#[test]
fn deferred_blocks_execute_on_the_real_target() {
    let declarations = "
        var State:u32
        func mark(value:u32){State=State*10+value}
        func work(mode:u32)->u32{
            defer {mark(1)}
            if mode==1 {defer {mark(2)};return 7}
            if mode==2 {return 8}
            var observed=3 as u32
            defer {mark(observed)}
            observed=4
            return 9
        }";
    run(declarations, "work(1)*100+State", 721);
    run(declarations, "work(2)*100+State", 801);
    run(declarations, "work(3)*100+State", 941);
}

#[test]
fn conditional_local_islands_execute_both_directions_and_skip_literals() {
    for delta in [32767i32, 32768, -32768, -32769] {
        for condition in [0, 1] {
            let mut e = emit::Emitter::default();
            let target = e.label();
            let (at, destination);
            if delta > 0 {
                at = 0;
                e.branch(1, target, Span::synthetic());
                e.words.resize(delta as usize + 1, 0);
                destination = e.words.len();
                e.bind(target);
                e.trap();
            } else {
                destination = 0;
                e.bind(target);
                e.words.resize((-delta - 1) as usize, 0);
                at = e.words.len();
                e.branch(1, target, Span::synthetic());
                e.trap();
            }
            let (bytes, _) = e.finish().unwrap();
            // Repeat at different bases to prove the local literal is relative.
            for base in [0x1000, 0x90000] {
                let mut bus = DefaultBus::new();
                bus.map_device(base, 0x40000, Ram::new(0x40000)).unwrap();
                for (i, byte) in bytes.iter().enumerate() {
                    bus.write8(base + i as u32, *byte).unwrap();
                }
                let mut cpu = Cpu::new();
                for r in 1..=15 {
                    cpu.set_reg(r, 0xaabb0000 + r as u32);
                }
                cpu.set_reg(1, condition);
                cpu.set_pc(base + at as u32 * 4);
                let goal = base + if condition == 0 { destination } else { at + 7 } as u32 * 4;
                for _ in 0..6 {
                    cpu.step(&mut bus).unwrap();
                    if cpu.pc() == goal {
                        break;
                    }
                }
                assert_eq!(cpu.pc(), goal, "delta={delta}, condition={condition}");
                assert_eq!(cpu.reg(1), condition);
                for r in 2..=15 {
                    if condition == 0
                        && !(-32768..=32767).contains(&delta)
                        && (12..=13).contains(&r)
                    {
                        continue;
                    }
                    assert_eq!(cpu.reg(r), 0xaabb0000 + r as u32);
                }
            }
        }
    }
}

fn large_config() -> crate::ProjectConfig {
    crate::ProjectConfig {
        ram_size: 1024 * 1024,
        stack_size: 65536,
    }
}

#[test]
fn far_startup_initializer_calls_stack_arguments_and_function_addresses_execute() {
    let fixture = Fixture::new(
        "package main\n
        var State:u32=23
        func sum(a:u32,b:u32,c:u32,d:u32,e:u32,f:u32,g:u32)->u32 {return a+b+c+d+e+f+g+State}
        func padding(){var buf:[2000]u8;buf[1999]=1}
        func main(){var direct=sum(1,2,3,4,5,6,7);var indirect= sum;
            store32(4294934608,direct+indirect(7,6,5,4,3,2,1))}",
    );
    let project = fixture.core();
    let objects = codegen_objects(&project).unwrap();
    let symbols = &objects[0].symbols;
    let at = |name: &str| symbols.iter().find(|s| s.name == name).unwrap().offset;
    assert!(at("main.main") - at("main.sum") > 131072);
    let bytes = crate::test_support::build_image(&project, large_config()).unwrap();
    assert_eq!(execute(&bytes), 102);
}

#[test]
fn large_loop_and_conditional_edges_execute_from_source() {
    for flag in ["false", "true"] {
        let fixture = Fixture::new(&format!(
            "package main\n
            func loop(flag:bool)->u32 {{var n:u32;for n<2 {{
                if flag {{var buf:[1600]u8;buf[1599]=1}}
                n=n+1
            }};return n}}
            func main(){{store32(4294934608,loop({flag}))}}"
        ));
        let project = fixture.core();
        let objects = codegen_objects(&project).unwrap();
        assert!(
            objects[0]
                .symbols
                .iter()
                .find(|s| s.name == "main.loop")
                .unwrap()
                .size
                > 131072
        );
        assert_eq!(
            execute_steps(
                &crate::test_support::build_image(&project, large_config()).unwrap(),
                1_000_000
            ),
            2
        );
    }
}
