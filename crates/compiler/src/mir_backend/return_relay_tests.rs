use super::{frame::Frame, tests::Fixture, *};
use crate::linker::{LinkPlan, LinkedOutput, link_objects_with_debug};
use kagura::Bus;
use ond_compiler_core::Compilation;
use ond_protocol::debug::{DebugOperation, FunctionDebug, ReturnAddressLocation};
use std::ops::Range;

const RELAY: &str = "main.relay";
const EXIT: u32 = 0xffff8040;
const INPUT: u32 = 0x2000_0000;

fn image(frontend: &Compilation, stack_size: u32, relay: bool) -> LinkedOutput {
    let paths = frontend.loaded.packages[0]
        .files
        .iter()
        .map(|id| (*id, "main.ond".into()))
        .collect();
    let code = codegen_selected_using(
        &frontend.mir,
        &frontend
            .mir
            .packages
            .iter()
            .map(|package| package.logical_path.clone())
            .collect(),
        Some((&frontend.loaded.sources, &paths)),
        |project, globals, function, emitter| {
            if !relay && function.name == RELAY {
                function::lower_without_return_relay(project, function, emitter)
            } else {
                raw_induction::lower(project, globals, function, emitter)
            }
        },
    )
    .unwrap();
    let code = runtime::supply_with_debug(code.objects, code.debug).unwrap();
    let metadata = crate::program::ProgramMetadata::from_mir(&frontend.mir).unwrap();
    let mut objects = vec![crate::test_support::startup_object(
        &metadata.initializers,
        &metadata.entry_symbol,
    )];
    objects.extend(code.objects);
    link_objects_with_debug(
        &objects,
        &code.debug,
        &LinkPlan {
            read_only: None,
            memory_base: 0x1000,
            memory_size: 1 << 20,
            stack_size,
            entry_symbol: "__ond.entry".into(),
            heap_base_symbol: "__ond_heap_base".into(),
            stack_bottom_symbol: "__ond_stack_bottom".into(),
        },
    )
    .unwrap()
}

struct RelayFrame {
    child: Range<u32>,
    values: Vec<Range<u32>>,
}

fn layout(frontend: &Compilation) -> RelayFrame {
    let p = &frontend.mir;
    let f = p.packages[0]
        .functions
        .iter()
        .find(|f| f.name == RELAY)
        .unwrap();
    let block = &f.blocks[0];
    let call = block.instructions.last().unwrap();
    let mir::InstructionKind::Call { signature, .. } = &call.kind else {
        panic!("expected immediate call");
    };
    let abi = layout::abi(
        &p.types,
        &mir::FunctionType {
            parameters: f
                .parameters
                .iter()
                .map(|id| f.locals[id.0 as usize].ty)
                .collect(),
            returns: f.returns.clone(),
        },
        f.span,
    )
    .unwrap();
    let frame = Frame::new(f, &p.types, &abi, &private_array_copy::Plan::default()).unwrap();
    let returns = frame.calls[&call.id].returns;
    let size = layout::abi(&p.types, signature, call.span)
        .unwrap()
        .return_bytes;
    assert!(returns + size.max(1) <= frame.size);
    RelayFrame {
        child: returns..returns + size,
        values: call
            .results
            .iter()
            .map(|&v| {
                let size = layout::data(&p.types, f.values[v.0 as usize].ty, call.span)
                    .unwrap()
                    .size;
                frame.value(v)..frame.value(v) + size.max(4)
            })
            .collect(),
    }
}

fn relay(linked: &LinkedOutput) -> &FunctionDebug {
    linked
        .debug
        .functions
        .iter()
        .find(|f| f.symbol == RELAY)
        .unwrap()
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Transfer {
    write: bool,
    offset: u32,
    bytes: u8,
    bits: u32,
}

struct Run {
    steps: u32,
    peak: u32,
    stack: Vec<Transfer>,
    child: Vec<Transfer>,
    parent: Vec<Transfer>,
    private: Vec<Transfer>,
}

/// Trace only normal execution in RAM. Reading decoded memory operands from
/// the running guest avoids changing emitted code or observing MMIO twice.
fn run(linked: &LinkedOutput, layout: &RelayFrame, input: &[u32], return_line: u32) -> Run {
    let (mut cpu, mut bus, top) = tests::machine(&linked.image);
    bus.map_device(INPUT, 8, ram::Ram::new(8)).unwrap();
    for (n, bits) in input.iter().enumerate() {
        bus.write32(INPUT + n as u32 * 4, *bits).unwrap();
    }
    let f = relay(linked);
    let mut result = Run {
        steps: 0,
        peak: 0,
        stack: Vec::new(),
        child: Vec::new(),
        parent: Vec::new(),
        private: Vec::new(),
    };
    let mut sp = 0;
    let mut parent = 0..0;
    let mut started_writing = false;
    for step in 1..100_000 {
        let pc = cpu.pc();
        if pc == f.address {
            assert_eq!(sp, 0, "test calls the relay exactly once");
            sp = cpu.reg(14) - f.frame_size;
            let address = cpu.reg(1);
            assert!(sp + f.frame_size <= address);
            parent = address..address + (layout.child.end - layout.child.start);
        }
        if (f.address..f.address + f.size).contains(&pc) {
            let word = bus.read32(pc).unwrap();
            let (write, bytes) = match word >> 28 {
                4 => (false, 1),
                5 => (false, 2),
                7 => (false, 4),
                8 => (true, 1),
                9 => (true, 2),
                11 => (true, 4),
                _ => (false, 0),
            };
            if bytes != 0 {
                let reg = |shift| cpu.reg(((word >> shift) & 0xfu32) as usize);
                let address = reg(20)
                    .wrapping_add(reg(16))
                    .wrapping_add_signed(i32::from(word as i16));
                if (sp..top).contains(&address) {
                    let bits = if write {
                        reg(24) & (u32::MAX >> (32 - u32::from(bytes) * 8))
                    } else {
                        match bytes {
                            1 => u32::from(bus.read8(address).unwrap()),
                            2 => u32::from(bus.read16(address).unwrap()),
                            4 => bus.read32(address).unwrap(),
                            _ => unreachable!(),
                        }
                    };
                    let offset = address - sp;
                    // Relaying shrinks Text, so the caller’s link address
                    // shifts while its position within the caller stays fixed.
                    let bits = if offset == f.return_address_offset {
                        bits - linked.image.symbols["main.main"]
                    } else {
                        bits
                    };
                    let access = Transfer {
                        write,
                        offset,
                        bytes,
                        bits,
                    };
                    result.stack.push(access.clone());
                    if layout.child.contains(&offset) || parent.contains(&address) {
                        let (expected, transfers, base) = if layout.child.contains(&offset) {
                            assert!(!write && !started_writing);
                            (
                                DebugOperation::MemoryRead,
                                &mut result.child,
                                layout.child.start,
                            )
                        } else {
                            assert!(write);
                            started_writing = true;
                            (
                                DebugOperation::MemoryWrite,
                                &mut result.parent,
                                parent.start - sp,
                            )
                        };
                        let range = linked
                            .debug
                            .ranges
                            .iter()
                            .filter(|range| range.start <= pc && pc < range.end)
                            .min_by_key(|range| range.end - range.start)
                            .unwrap();
                        assert_eq!(range.operation, expected);
                        assert_eq!(range.function, RELAY);
                        let source = range.source.as_ref().unwrap();
                        assert_eq!(source.path, "main.ond");
                        assert_eq!(
                            (source.line, source.column),
                            (return_line, if write { 5 } else { 12 })
                        );
                        let unwind = f.unwind.iter().rfind(|row| row.address <= pc).unwrap();
                        assert_eq!(unwind.cfa_sp_offset, f.frame_size);
                        assert_eq!(
                            unwind.return_address,
                            ReturnAddressLocation::StackOffset(f.return_address_offset)
                        );
                        transfers.push(Transfer {
                            offset: offset - base,
                            ..access
                        });
                    } else if layout.values.iter().any(|slot| slot.contains(&offset)) {
                        result.private.push(access);
                    }
                }
            }
        }
        cpu.step(&mut bus).unwrap();
        result.peak = result.peak.max(top - cpu.reg(14));
        if bus.read32(EXIT).unwrap() == 0 {
            assert_eq!(cpu.reg(14), top);
            for r in 7..=11 {
                assert_eq!(cpu.reg(r), 0xabcd_0000 + r as u32);
            }
            result.steps = step;
            return result;
        }
    }
    panic!("guest step limit at {:#x}", cpu.pc());
}

fn guards(linked: &LinkedOutput) -> impl Iterator<Item = (&str, &[u8])> {
    linked
        .debug
        .ranges
        .iter()
        .filter(|r| r.operation == DebugOperation::StackOverflow)
        .map(|range| {
            let section = linked
                .image
                .sections
                .iter()
                .find(|s| {
                    s.kind == SectionKind::Text
                        && s.load_address <= range.start
                        && range.end <= s.load_address + s.memory_size
                })
                .unwrap();
            let start = (range.start - section.load_address) as usize;
            let end = (range.end - section.load_address) as usize;
            (range.function.as_str(), &section.data[start..end])
        })
}

fn frame_shape(linked: &LinkedOutput) -> impl Iterator<Item = (&str, u32, u32)> {
    linked
        .debug
        .functions
        .iter()
        .map(|f| (f.symbol.as_str(), f.frame_size, f.return_address_offset))
}

fn assert_only_private_accesses_removed(layout: &RelayFrame, before: &Run, after: &Run) {
    assert_eq!(before.private.len(), 4);
    assert_eq!(before.private.iter().filter(|a| a.write).count(), 2);
    assert!(after.private.is_empty());
    assert_eq!(
        before
            .stack
            .iter()
            .filter(|a| !layout.values.iter().any(|slot| slot.contains(&a.offset)))
            .cloned()
            .collect::<Vec<_>>(),
        after.stack
    );
}

fn transfer(write: bool, offset: u32, bytes: u8, bits: u32) -> Transfer {
    Transfer {
        write,
        offset,
        bytes,
        bits,
    }
}

#[test]
fn private_return_relay_keeps_abi_accesses_source_unwind_and_stack() {
    let fixture = Fixture::new(
        "package main
func child(x:u32,y:u32) -> [2]u32 { return [2]u32{x,y} }
func relay(x:u32,y:u32) -> [2]u32 {
    return child(x,y)
}
func main() { _=relay(7,19) }",
    );
    let frontend = ond_compiler_core::compile_project(&fixture.0).unwrap();
    let layout = layout(&frontend);
    let compare = |stack| {
        let [old, new] = [false, true].map(|enabled| image(&frontend, stack, enabled));
        let [before, after] = [&old, &new].map(|linked| run(linked, &layout, &[], 4));
        assert_eq!(
            before.child,
            vec![transfer(false, 0, 4, 7), transfer(false, 4, 4, 19)]
        );
        assert_eq!(
            before.parent,
            vec![transfer(true, 0, 4, 7), transfer(true, 4, 4, 19)]
        );
        assert_eq!(before.child, after.child);
        assert_eq!(before.parent, after.parent);
        assert_only_private_accesses_removed(&layout, &before, &after);
        let (old_fn, new_fn) = (relay(&old), relay(&new));
        assert_eq!(old_fn.size - new_fn.size, 24);
        assert_eq!(before.steps - after.steps, 6);
        assert_eq!(
            frame_shape(&old).collect::<Vec<_>>(),
            frame_shape(&new).collect::<Vec<_>>()
        );
        assert_eq!(
            guards(&old).collect::<Vec<_>>(),
            guards(&new).collect::<Vec<_>>()
        );
        assert_eq!(before.peak, after.peak);
        let [old_text, new_text] = [&old, &new].map(|linked| {
            linked
                .image
                .sections
                .iter()
                .filter(|s| s.kind == SectionKind::Text)
                .map(|s| s.memory_size)
                .sum::<u32>()
        });
        assert_eq!(old_text - new_text, 24);
        eprintln!(
            "relay [2]u32 text={}→{} B (total {old_text}→{new_text} B) guest={}→{} steps frame={} B peak={} B",
            old_fn.size, new_fn.size, before.steps, after.steps, old_fn.frame_size, after.peak
        );
        after.peak
    };
    let peak = compare(8192);
    // Both images fit at their actual peak. The unchanged guards check
    // new_sp < STACK_BOTTOM in every function, so four bytes less would
    // reject the deepest allocation in both images.
    assert_eq!(compare(peak), peak);
}

#[test]
fn private_return_relay_preserves_typed_slot_widths_and_bits() {
    for (declarations, arguments, input, payload, zeros) in [
        (
            "type Tiny [1]u8
func child(x:u8,y:bool) -> (Tiny,bool) { return Tiny{x},y }
func relay(x:u8,y:bool) -> (Tiny,bool) {
    return child(x,y)
}",
            "167,*(536870912 as u32 as *bool)",
            &[0x80][..],
            vec![(0, 1, 167), (1, 1, 1)],
            2..5,
        ),
        (
            "func child(x:u8,y:u16) -> ([1]u8,u16) { return [1]u8{x},y }
func relay(x:u8,y:u16) -> ([1]u8,u16) {
    return child(x,y)
}",
            "167,57072",
            &[],
            vec![(0, 1, 167), (2, 2, 57072)],
            4..6,
        ),
        (
            "func child(x:f32,y:f32) -> [2]f32 { return [2]f32{x,y} }
func relay(x:f32,y:f32) -> [2]f32 {
    return child(x,y)
}",
            "*(536870912 as u32 as *f32),*(536870916 as u32 as *f32)",
            &[0x7fc1_2345, 0x8000_0000],
            vec![(0, 4, 0x7fc1_2345), (4, 4, 0x8000_0000)],
            0..0,
        ),
        (
            "func child(x:f64) -> [1]f64 { return [1]f64{x} }
func relay(x:f64) -> [1]f64 {
    return child(x)
}",
            "*(536870912 as u32 as *f64)",
            &[0x5678_abcd, 0x7ff8_1234],
            vec![(0, 4, 0x5678_abcd), (4, 4, 0x7ff8_1234)],
            0..0,
        ),
    ] {
        // Keep the return source at the same coordinates in each case.
        let prefix = if declarations.starts_with("type ") {
            "package main\n"
        } else {
            "package main\n\n"
        };
        let fixture = Fixture::new(&format!(
            "{prefix}{declarations}\nfunc main() {{ relay({arguments}) }}"
        ));
        let frontend = ond_compiler_core::compile_project(&fixture.0)
            .unwrap_or_else(|error| panic!("{declarations}: {error:?}"));
        let layout = layout(&frontend);
        let [old, new] = [false, true].map(|enabled| image(&frontend, 8192, enabled));
        let [before, after] = [&old, &new].map(|linked| run(linked, &layout, input, 5));
        let reads = payload
            .iter()
            .map(|&(o, s, v)| transfer(false, o, s, v))
            .collect::<Vec<_>>();
        let writes = payload
            .into_iter()
            .map(|(o, s, v)| transfer(true, o, s, v))
            .chain(zeros.map(|o| transfer(true, o, 1, 0)))
            .collect::<Vec<_>>();
        assert_eq!(before.child, reads);
        assert_eq!(before.parent, writes);
        assert_eq!(before.child, after.child);
        assert_eq!(before.parent, after.parent);
        assert_only_private_accesses_removed(&layout, &before, &after);
        assert!(before.steps > after.steps);
        assert!(relay(&old).size > relay(&new).size);
        assert_eq!(relay(&old).frame_size, relay(&new).frame_size);
        assert_eq!(before.peak, after.peak);
    }
}

#[test]
fn private_return_relay_keeps_the_existing_path_when_unproven() {
    for (declare, returns, body) in [
        (
            "func child()->[0]u32 { return [0]u32{} }",
            "[0]u32",
            "return child()",
        ),
        (
            "func child()->[3]u8 { return [3]u8{1,2,3} }",
            "[3]u8",
            "return child()",
        ),
        (
            "type A u8\ntype B u8\nfunc child()->([1]u8,A) { return [1]u8{1}, 2 as A }",
            "([1]u8,B)",
            "var array, value=child(); return array, value as B",
        ),
        (
            "func child()->[2]u32 { return [2]u32{1,2} }",
            "[2]u32",
            "var result=child(); result[0]=3; return result",
        ),
        (
            "func marker() {}\nfunc child()->[2]u32 { return [2]u32{1,2} }",
            "[2]u32",
            "defer { marker() }; return child()",
        ),
        (
            "func helper()->u32 { return 7 }\nfunc child()->[2]u32 { return [2]u32{helper(),2} }",
            "[2]u32",
            "return child()",
        ),
        (
            "var Sample:u32\nfunc child(p:*u32)->[2]u32 { return [2]u32{*p,2} }",
            "[2]u32",
            "return child(&Sample)",
        ),
        (
            "func child()->[2]u32 { var x:u32; var p=&x; _=p; return [2]u32{1,2} }",
            "[2]u32",
            "return child()",
        ),
        (
            "func child()->[2]u32 { return [2]u32{1,2} }",
            "[2]u32",
            "var pick:func()->[2]u32 = child; return pick()",
        ),
        (
            "func child()->[2]u32 { asm { ADD r1, r0, r0, 0 }; return [2]u32{1,2} }",
            "[2]u32",
            "return child()",
        ),
    ] {
        let fixture = Fixture::new(&format!(
            "package main\n{declare}\nfunc relay()->{returns} {{ {body} }}\nfunc main() {{ relay() }}"
        ));
        let frontend = ond_compiler_core::compile_project(&fixture.0)
            .unwrap_or_else(|error| panic!("{declare}: {body}: {error:?}"));
        if returns == "[0]u32" {
            let frame = layout(&frontend);
            assert!(frame.child.is_empty());
            assert!(
                frame
                    .values
                    .iter()
                    .all(|slot| slot.end <= frame.child.start)
            );
        }
        let fallback = assert_fallback(&frontend, body);
        if returns == "[3]u8" {
            // The existing Call and Source ranges enclose the whole
            // transfer. Non-candidates gain no per-access ranges.
            let operations = fallback
                .debug
                .ranges
                .iter()
                .filter(|r| r.function == RELAY && r.operation != DebugOperation::StackOverflow)
                .map(|r| r.operation)
                .collect::<Vec<_>>();
            assert_eq!(operations, [DebugOperation::Call, DebugOperation::Source]);
        }
    }
}

#[test]
fn private_return_relay_leaves_cross_package_calls_independent() {
    let fixture = Fixture::new(
        "package main
import \"test/shared\"
func relay()->[2]u32 { return shared.Child() }
func main() { relay() }",
    );
    std::fs::create_dir(fixture.0.join("shared")).unwrap();
    std::fs::write(
        fixture.0.join("shared/main.ond"),
        "package shared\nfunc Child()->[2]u32 { return [2]u32{1,2} }",
    )
    .unwrap();
    let frontend = ond_compiler_core::compile_project(&fixture.0).unwrap();
    assert_fallback(&frontend, "cross-package child");
}

fn assert_fallback(frontend: &Compilation, case: &str) -> LinkedOutput {
    let [old, new] = [false, true].map(|enabled| image(frontend, 8192, enabled));
    assert_eq!(old.image.sections.len(), new.image.sections.len(), "{case}");
    for (left, right) in old.image.sections.iter().zip(&new.image.sections) {
        assert_eq!(left.load_address, right.load_address, "{case}");
        assert_eq!(left.data, right.data, "{case}");
    }
    assert_eq!(old.debug, new.debug, "{case}");
    new
}
