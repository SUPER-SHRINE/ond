use super::{
    bounds::proven_checks,
    tests::{Fixture, machine},
    *,
};
use kagura::{Bus, FaultCode};
use mir::{InstructionKind as I, TerminatorKind as T};
use ond_protocol::debug::DebugOperation;
use ram::Ram;

const INPUT: u32 = 0x2000_0000;
const OUTPUT: u32 = INPUT + 0x100;
const END: u32 = 0xffff_8040;
const SENTINEL: u32 = 0xdead_beef;

fn counts(p: &mir::Project, f: &mir::Function) -> (usize, usize) {
    let checks = f
        .blocks
        .iter()
        .flat_map(|b| &b.instructions)
        .filter(|i| matches!(i.kind, I::Check(mir::Check::Bounds { .. })))
        .count();
    (checks, proven_checks(&p.types, f).len())
}

#[test]
fn range_bounds_ordinary_condition_and_loop_keep_unproven_fault_and_earlier_effects() {
    let fixture = Fixture::new(include_str!("bounds_fixture.ond"));
    let p = fixture.core();
    let result = codegen_raw(&p).unwrap();
    let debug = &result.debug[0];
    for (name, removed) in [("main.select", 1), ("main.scan", 1), ("main.changed", 0)] {
        let f = p.packages[0]
            .functions
            .iter()
            .find(|f| f.name == name)
            .unwrap();
        assert_eq!(counts(&p, f), (1, removed), "{name}");
        let retained = debug
            .ranges
            .iter()
            .filter(|range| {
                range.function == name && range.operation == DebugOperation::BoundsCheck
            })
            .count();
        assert_eq!(retained, 1 - removed, "{name}");
    }
    let image = crate::test_support::build_image(&p, crate::ProjectConfig::default()).unwrap();
    let changed = debug
        .functions
        .iter()
        .find(|f| f.symbol == "main.changed")
        .unwrap();
    for (index, expected) in [
        (0, [3, 26, 5]),
        (1, [5, 23, 7]),
        (3, [11, 11, SENTINEL]),
        (4, [99, 0, 99]),
        (u32::MAX, [99, 0, 99]),
    ] {
        let (mut cpu, mut bus, top) = machine(&image);
        bus.map_device(INPUT, 512, Ram::new(512)).unwrap();
        bus.write32(INPUT, index).unwrap();
        for n in 0..3 {
            bus.write32(OUTPUT + 4 * n, SENTINEL).unwrap();
        }
        let finished = (0..40_000).any(|_| {
            if let Err(fault) = cpu.step(&mut bus) {
                assert_eq!(index, 3);
                assert_eq!(fault.code(), FaultCode::InvalidInstruction);
                let pc = fault.faulting_pc() - image.symbols["main.changed"] + changed.offset;
                assert!(debug.ranges.iter().any(|r| {
                    r.start <= pc && pc < r.end && r.operation == DebugOperation::BoundsCheck
                }));
                return true;
            }
            if bus.read32(END).unwrap() == 0 {
                assert_ne!(index, 3);
                assert_eq!(cpu.reg(14), top);
                return true;
            }
            false
        });
        assert!(finished, "{index}");
        assert_eq!(
            [0, 1, 2].map(|n| bus.read32(OUTPUT + 4 * n).unwrap()),
            expected
        );
    }
}

#[test]
fn range_bounds_proofs_cover_projections_and_require_every_incoming_path() {
    let fixture = Fixture::new(
        r#"package main
type Pair struct { x: u32; }
func projects(i: u32) -> u32 {
    var items = [2]Pair{Pair{x: 8}, Pair{x: 9}}
    if i < 2 {
        var old = items[i]
        items[i] = old
        items[i].x = old.x + 1
        var address = &items[i].x
        return *address
    }
    return 0
}
func joins(i: u32, flag: bool) -> u32 {
    var a: [2]u32
    var n: u32
    if i < 2 {
        if flag { n = 1 } else { n = 2 }
        return n + a[i]
    }
    return 0
}
func false_edge(i: u32) -> u32 {
    var a: [2]u32
    if i < 2 { return 0 } else { return a[i] }
}
func missing_path(i: u32) -> u32 {
    var a: [2]u32
    var n: u32
    if i < 2 { n = 1 }
    return n + a[i]
}
func first_unproven(i: u32) -> u32 {
    var a: [2]u32
    var n: u32
    for n < 2 {
        n += a[i]
        if i < 2 { n += 1 } else { return n }
    }
    return n
}
func backedge_writes(i: u32, other: u32) -> u32 {
    var a: [2]u32
    var n: u32
    if i < 2 {
        for n < 2 {
            n += a[i] + 1
            i = other
        }
    }
    return n
}
func main() {}"#,
    );
    let p = fixture.core();
    for (name, want) in [
        ("main.projects", (4, 4)),
        ("main.joins", (1, 1)),
        ("main.false_edge", (1, 0)),
        ("main.missing_path", (1, 0)),
        ("main.first_unproven", (1, 0)),
        ("main.backedge_writes", (1, 0)),
    ] {
        let f = p.packages[0]
            .functions
            .iter()
            .find(|f| f.name == name)
            .unwrap();
        assert_eq!(counts(&p, f), want, "{name}");
    }
}

#[test]
fn range_bounds_retains_checks_after_mutation_and_opaque_effects() {
    let fixture = Fixture::new(
        r#"package main
func call() {}
func reload(i: u32) -> u32 { var a: [2]u32; if i < 2 { i += 1; return a[i] }; return 0 }
func alias(i: u32) -> u32 { var a: [2]u32; var p = &i; if i < 2 { *p = 2; return a[i] }; return 0 }
func raw(i: u32, p: *u32) -> u32 { var a: [2]u32; if i < 2 { *p = 2; return a[i] }; return 0 }
func raw_read(i: u32, p: *u32) -> u32 { var a: [2]u32; if i < 2 { _ = *p; return a[i] }; return 0 }
func callable(i: u32) -> u32 { var a: [2]u32; if i < 2 { call(); return a[i] }; return 0 }
func helper(i: u32, divisor: u32) -> u32 { var a: [2]u32; if i < 2 { var n = i / divisor; return n + a[i] }; return 0 }
func assembly(i: u32) -> u32 { var a: [2]u32; if i < 2 { asm () { ADD r1, r0, r0, 0 }; return a[i] }; return 0 }
func wrapped(i: u32) -> u32 { var a: [2]u32; if i < 2 { return a[i + 4294967295] }; return 0 }
func mismatch(i: u32) -> u32 { var a: [2]u32; if i < 3 { return a[i] }; return 0 }
func zero(i: u32) -> u32 { var a: [0]u32; if i < 0 { return a[i] }; return 0 }
func signed(i: i32) -> u32 { var a: [2]u32; if i < 2 { return a[i] }; return 0 }
func main() {}"#,
    );
    let p = fixture.core();
    for f in &p.packages[0].functions {
        if ["main.call", "main.main"].contains(&f.name.as_str()) {
            continue;
        }
        assert_eq!(counts(&p, f), (1, 0), "{}", f.name);
    }
}

#[test]
fn range_bounds_requires_fresh_comparison_and_does_not_guess_block_arguments() {
    let fixture = Fixture::new(
        r#"package main
func stale(i: u32) -> u32 { var a: [2]u32; if i < 2 { i = 2; return a[i] }; return 0 }
func carried(i: u32) -> u32 { var a: [2]u32; if i < 2 { return a[i] }; return 0 }
func main() {}"#,
    );
    let p = fixture.core();
    let stale = p.packages[0]
        .functions
        .iter()
        .find(|f| f.name == "main.stale")
        .unwrap();
    let mut stale = stale.clone();
    let branch = stale
        .blocks
        .iter()
        .position(|b| matches!(b.terminator.kind, T::Branch { .. }))
        .unwrap();
    let T::Branch { ref then_edge, .. } = stale.blocks[branch].terminator.kind else {
        unreachable!()
    };
    let target = then_edge.target.0 as usize;
    let store = stale.blocks[target]
        .instructions
        .iter()
        .position(|i| matches!(i.kind, I::Store { .. }))
        .unwrap();
    let moved: Vec<_> = stale.blocks[target].instructions.drain(..=store).collect();
    stale.blocks[branch].instructions.extend(moved);
    mir::validate_function(&p.types, &stale).unwrap();
    assert_eq!(counts(&p, &stale), (1, 0));

    let carried = p.packages[0]
        .functions
        .iter()
        .find(|f| f.name == "main.carried")
        .unwrap();
    let mut carried = carried.clone();
    assert_eq!(counts(&p, &carried), (1, 1));
    let passed = mir::ValueId(carried.values.len() as u32);
    carried.values.push(mir::Value {
        id: passed,
        ty: mir::TypeId::U32,
        span: carried.span,
    });
    let branch = carried
        .blocks
        .iter_mut()
        .find(|b| matches!(b.terminator.kind, T::Branch { .. }))
        .unwrap();
    let T::Branch {
        ref mut then_edge,
        condition,
        ..
    } = branch.terminator.kind
    else {
        unreachable!()
    };
    let i = branch
        .instructions
        .iter()
        .find(|i| i.results.contains(&condition))
        .unwrap();
    let I::Binary { lhs, .. } = i.kind else {
        unreachable!()
    };
    then_edge.arguments.push(lhs);
    let target = then_edge.target.0 as usize;
    carried.blocks[target].parameters.push(passed);
    for i in &mut carried.blocks[target].instructions {
        match &mut i.kind {
            I::Check(mir::Check::Bounds { index, .. }) => *index = passed,
            I::Load { place, .. } => {
                for projection in &mut place.projections {
                    if let mir::Projection::Index { index, .. } = projection {
                        *index = passed;
                    }
                }
            }
            _ => {}
        }
    }
    mir::validate_function(&p.types, &carried).unwrap();
    assert_eq!(counts(&p, &carried), (1, 0));
}
