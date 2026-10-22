//! MEM-02.stack-startup-lifetimes: 実行時のstore完了時SPと公開image範囲を比較する。
use super::*;
use crate::ir::object::SectionKind;
const RESULT: u32 = 0x20000100;
const SOURCE: &str = r###"package main
var Global: u32 = 3
var Escaped: *u32
var Child: *u32
var Trace: u32
func output() -> *u32 { return 536871168 as u32 as *u32 }
func nested(parent: *u32) {
    var child: u32 = 11
    Child = &child
    output()[2] = Child as u32
    defer {
        *Child += 1
        output()[5] = *Child
        Trace = Trace * 10 + 2
    }
    *parent += child
    Trace = Trace * 10 + 1
}
func visit() -> u32 {
    var local: u32 = 7
    Escaped = &local
    output()[1] = Escaped as u32
    defer {
        *Escaped += 1
        output()[4] = *Escaped
        Trace = Trace * 10 + 4
    }
    nested(&local)
    Trace = Trace * 10 + 3
    return local
}
func main() {
    output()[0] = (&Global) as u32
    var result = visit()
    output()[3] = result
    output()[6] = Trace
    output()[7] = Global
}
"###;

fn observe() -> (crate::linker::LinkedImage, [u32; 8], [u32; 2]) {
    let fixture = Fixture::new(SOURCE);
    fs::write(
        fixture.0.join("ond.toml"),
        "[project]\nname = \"test\"\n[target.kagura]\nram = 8388608\nstack = 1048576\n",
    )
    .unwrap();
    let image = crate::test_support::compile_image(&fixture.0).unwrap();
    let (mut cpu, mut bus, top) = machine(&image);
    bus.map_device(0x20000000, 0x200, Ram::new(0x200)).unwrap();
    for offset in 0..64 {
        bus.write32(RESULT + offset * 4, 0xdeadbeef).unwrap();
    }
    let mut stores = [None, None];
    let mut exited = false;
    for _ in 0..200_000 {
        cpu.step(&mut bus).unwrap();
        for (index, store) in stores.iter_mut().enumerate() {
            let pointer = bus.read32(RESULT + 4 + index as u32 * 4).unwrap();
            if store.is_none() && pointer != 0xdeadbeef {
                *store = Some(cpu.reg(14));
            }
        }
        if bus.read32(EXIT).unwrap() != 0xdeadbeef {
            exited = true;
            break;
        }
    }
    assert!(exited, "MEM-02 instruction limit");
    assert_eq!(bus.read32(EXIT).unwrap(), 0);
    assert_eq!(cpu.reg(14), top);
    for register in 7..=11 {
        assert_eq!(cpu.reg(register), 0xabcd0000 + register as u32);
    }
    let words = std::array::from_fn(|index| bus.read32(RESULT + index as u32 * 4).unwrap());
    (
        image,
        words,
        stores.map(|sp| sp.expect("MEM-02 pointer store missing")),
    )
}
#[test]
fn mem02_stack_startup_lifetimes_bss() {
    let (image, words, _) = observe();
    let pointer = words[0];
    assert_eq!(pointer % 4, 0);
    assert!(
        image.sections.iter().any(|s| s.kind == SectionKind::Bss
            && pointer >= s.load_address
            && pointer
                .checked_add(4)
                .is_some_and(|end| end <= s.load_address + s.memory_size)),
        "MEM-02 Global must be in Bss: {pointer:#x}, {:?}",
        image.sections
    );
    assert!(pointer + 4 <= image.stack_bottom || pointer >= image.stack_top);
}
#[test]
fn mem02_stack_startup_lifetimes_local_storage() {
    let (image, words, stores) = observe();
    let l = words[1];
    let c = words[2];
    for (pointer, sp) in [(l, stores[0]), (c, stores[1])] {
        assert_eq!(pointer % 4, 0);
        assert!(
            pointer >= image.stack_bottom
                && pointer >= sp
                && pointer
                    .checked_add(4)
                    .is_some_and(|end| end <= image.stack_top),
            "MEM-02 pointer={pointer:#x}, store SP={sp:#x}"
        );
    }
    assert!(l + 4 <= c || c + 4 <= l);
    assert_eq!(&words[3..], &[18, 19, 12, 1234, 3]);
}
