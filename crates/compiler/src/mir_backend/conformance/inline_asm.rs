//! specs/ond/conformance/cases/inline-assembly.md の完全な入力。
pub const ASM_01: &str = r#"package main
var State:u32
func first() -> u32 { State = State * 10 + 1; return 3 }
func second() -> u32 { State = State * 10 + 2; return 4 }
func main() {
    var result:u32
    asm (r1 <- first(), r2 <- second(), r3 -> result,) {
        ADD r3, r1, r2, 0
    }
    *(536871168 as u32 as *u32) = State * 100 + result
}
"#;

pub const ASM_02: &str = r#"package main
func main() {
    var r1:u32 = 9
    var result:u32
    asm (r2 <- r1, r3 -> result) { ADD r3, r2, r0, 1 }
    *(536871168 as u32 as *u32) = result
}
"#;

pub const ASM_03: &str = r#"package main
func signed() -> i8 {
    var value:i8 = -2
    asm (r1 <-> value) { ADD r1, r1, r0, 0 }
    return value
}
func truth() -> bool {
    var value:bool
    asm (r1 -> value) { ADD r1, r0, r0, 2 }
    return value
}
func floating() -> f32 {
    var value:f32 = 1.5
    var result:f32
    asm (r1 <- value, r2 -> result) { ADD r2, r1, r0, 0 }
    return result
}
func main() {
    *(536871168 as u32 as *u32) = signed() as u32
    var truthBits:u32
    if truth() { truthBits = 1 }
    *(536871172 as u32 as *u32) = truthBits
    var floatBits:u32
    asm (r1 <- floating(), r2 -> floatBits) { ADD r2, r1, r0, 0 }
    *(536871176 as u32 as *u32) = floatBits
}
"#;

pub const ASM_04: &str = r#"package main
func main() {
    var value:u32 = 8
    asm (r1 <- value) { ADD r1, r1, r0, 2 }
    asm { ADD r7, r0, r0, 123 }
    *(536871168 as u32 as *u32) = value
}
"#;

pub const ASM_05: &str = r#"package main
func main() {
    var address:u32 = 536870952
    var pointer = address as *u32
    *pointer = 5
    var result:u32
    asm (r1 <- 11, r2 <- address, r3 -> result) {
        STW r1, r2, r0, 0
        LDW r3, r2, r0, 0
    }
    *(536871168 as u32 as *u32) = result + *pointer
}
"#;

pub const ASM_06: &str = r#"package main
func main() {
    var address:u32 = 4294934608
    asm (r1 <- 1, r2 <- address, r3 <- 2) {
        STW r1, r2, r0, 0
        STW r3, r2, r0, 0
    }
}
"#;

pub const ASM_07: &str = r#"package main
func boot() {
    *(536871168 as u32 as *u32) = 2
    for {}
}
func main() {
    defer { *(536871172 as u32 as *u32) = 1 }
    asm noreturn (r1 <- boot, r2 <- 536871176, r14 <- 8388608) {
        STW r14, r2, r0, 0
        JZ r0, r0, r1, 0
    }
}
"#;

pub const ASM_08: &str = r#"package main
var Global:u32 = 9
func main() {
    var faultAddress:u32 = 305419896
    var outputAddress:u32 = 536871168
    asm (r1 <- faultAddress, r2 -> Global, r3 <- 77, r4 <- outputAddress) {
        STW r3, r4, r0, 0
        LDB r2, r1, r0, 0
    }
}
"#;

use super::super::tests::machine_with_output;
use super::runner::{Case, End, INPUT, OUTPUT, SENTINEL, compile, run};
use kagura::{AccessOperation, AccessWidth, Bus, BusFault, Device, Fault};
use ram::Ram;
use std::any::Any;

const STEP_LIMIT: usize = 2_000_000;
const EXIT: u32 = 0xffff8040;
const TRACE: u32 = 0xffff8050;

fn normal(id: &str, specs: &[&str], source: &str, words: &[u32]) {
    // 未指定wordも全64個のSENTINEL保持を照合する。
    let mut output = [SENTINEL; 64];
    output[..words.len()].copy_from_slice(words);
    run(
        &compile(&[("main.ond", source)]),
        Case {
            id,
            specs,
            inputs: &[],
            output: &output,
            end: End::Return,
            steps: STEP_LIMIT,
        },
    );
}

#[test]
fn asm_01_input_snapshot_order() {
    normal(
        "ASM-01.input-snapshot-order",
        &["ASM-01", "ASM-02"],
        ASM_01,
        &[1207],
    );
}

#[test]
fn asm_02_right_identifier_r1() {
    normal(
        "ASM-02.right-identifier-r1",
        &["ASM-01", "ASM-02"],
        ASM_02,
        &[10],
    );
}

#[test]
fn asm_03_inout_narrow_bool_f32() {
    normal(
        "ASM-03.inout-narrow-bool-f32",
        &["ASM-02"],
        ASM_03,
        &[0xfffffffe, 1, 0x3fc00000],
    );
}

#[test]
fn asm_04_binding_register_and_callee_save() {
    normal(
        "ASM-04.binding-register-and-callee-save",
        &["ASM-02", "ASM-04"],
        ASM_04,
        &[8],
    );
}

#[test]
fn asm_05_memory_alias_and_order() {
    normal("ASM-05.memory-alias-and-order", &["ASM-04"], ASM_05, &[22]);
}

#[derive(Default)]
struct TraceDevice {
    writes: Vec<(u32, u32, u32)>,
}

impl Device for TraceDevice {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn read8(&mut self, _: u32) -> Result<u8, BusFault> {
        Ok(0)
    }
    fn read16(&mut self, _: u32) -> Result<u16, BusFault> {
        Ok(0)
    }
    fn read32(&mut self, _: u32) -> Result<u32, BusFault> {
        Ok(0)
    }
    fn write8(&mut self, offset: u32, value: u8) -> Result<(), BusFault> {
        self.writes.push((TRACE + offset, 1, value.into()));
        Ok(())
    }
    fn write16(&mut self, offset: u32, value: u16) -> Result<(), BusFault> {
        self.writes.push((TRACE + offset, 2, value.into()));
        Ok(())
    }
    fn write32(&mut self, offset: u32, value: u32) -> Result<(), BusFault> {
        self.writes.push((TRACE + offset, 4, value));
        Ok(())
    }
}

fn observed_machine(image: &crate::linker::LinkedImage) -> (kagura::Cpu, kagura::DefaultBus, u32) {
    let (cpu, mut bus, top) = machine_with_output(image, TraceDevice::default());
    bus.map_device(INPUT, 512, Ram::new(512)).unwrap();
    for index in 0..64 {
        bus.write32(OUTPUT + index * 4, SENTINEL).unwrap();
    }
    (cpu, bus, top)
}

fn output(bus: &mut impl Bus, words: &[u32]) {
    for index in 0..64 {
        assert_eq!(
            bus.read32(OUTPUT + index * 4).unwrap(),
            words.get(index as usize).copied().unwrap_or(SENTINEL),
            "OUTPUT[{index}]"
        );
    }
}

#[test]
fn asm_06_mmio_write_order() {
    // ASM-06.mmio-write-order / ASM-04: deviceは成功storeの幅も記録する。
    let image = compile(&[("main.ond", ASM_06)]);
    let (mut cpu, mut bus, top) = observed_machine(&image);
    let mut exited = false;
    for _ in 0..STEP_LIMIT {
        cpu.step(&mut bus).unwrap();
        if bus.read32(EXIT).unwrap() != SENTINEL {
            exited = true;
            break;
        }
    }
    assert!(exited, "ASM-06: instruction limit");
    assert_eq!(bus.read32(EXIT).unwrap(), 0);
    assert_eq!(cpu.reg(14), top);
    for register in 7..=11 {
        assert_eq!(cpu.reg(register), 0xabcd0000 + register as u32);
    }
    assert_eq!(
        bus.device_ref::<TraceDevice>(TRACE).unwrap().writes,
        [(TRACE, 4, 1), (TRACE, 4, 2)]
    );
    output(&mut bus, &[]);
}

#[test]
fn asm_07_noreturn_transfer_skips_defer() {
    // ASM-07.noreturn-transfer-skips-defer / ASM-04
    let image = compile(&[("main.ond", ASM_07)]);
    let (mut cpu, mut bus, _) = observed_machine(&image);
    for _ in 0..STEP_LIMIT {
        cpu.step(&mut bus).unwrap();
        assert_eq!(
            bus.read32(EXIT).unwrap(),
            SENTINEL,
            "ASM-07 must not return"
        );
    }
    output(&mut bus, &[2, SENTINEL, 0x00800000]);
}

#[test]
fn asm_08_fault_preserves_completed_store() {
    // ASM-08.fault-preserves-completed-store / ASM-02, ASM-04
    let image = compile(&[("main.ond", ASM_08)]);
    let global = image.symbols["main.Global"];
    let (mut cpu, mut bus, _) = observed_machine(&image);
    let mut fault = None;
    for _ in 0..STEP_LIMIT {
        if let Err(observed) = cpu.step(&mut bus) {
            fault = Some(observed);
            break;
        }
        assert_eq!(bus.read32(EXIT).unwrap(), SENTINEL, "ASM-08 must fault");
    }
    assert!(
        matches!(
            fault,
            Some(Fault::BusFault {
                addr: 0x12345678,
                width: AccessWidth::Byte,
                operation: AccessOperation::Load,
                ..
            })
        ),
        "ASM-08: {fault:?}"
    );
    assert_eq!(bus.read32(EXIT).unwrap(), SENTINEL);
    assert_eq!(
        bus.read32(global).unwrap(),
        9,
        "output binding must not be committed after fault"
    );
    output(&mut bus, &[77]);
}

#[test]
fn asm_normal_loop_all_memory_widths_and_narrow_input_bits() {
    normal(
        "ASM.extra.loop-memory-widths",
        &["ASM-02", "ASM-03", "ASM-04"],
        r#"package main
func main(){
 var address:u32=536870912
 var value:u32=305419896
 var a:u32;var b:u32;var c:u32
 asm(r1<-value,r2<-address,r3->a,r4->b,r5->c){
 STB r1,r2,r0,0
 STH r1,r2,r0,2
 STW r1,r2,r0,4
 LDB r3,r2,r0,0
 LDH r4,r2,r0,2
 LDW r5,r2,r0,4
 ADD r6,r0,r0,3
 loop:
 ADD r6,r6,r0,-1
 JZ r0,r6,done
 JZ r0,r0,loop
 done:
 }
 *(536871168 as u32 as *u32)=a
 *(536871172 as u32 as *u32)=b
 *(536871176 as u32 as *u32)=c
 var x:i8=-2;var y:i16=-2
 asm(r1<-x,r2->a,r3<-y,r4->b){ADD r2,r1,r0,0;ADD r4,r3,r0,0}
 *(536871180 as u32 as *u32)=a
 *(536871184 as u32 as *u32)=b
}
"#,
        &[0x78, 0x5678, 0x12345678, 0xfe, 0xfffe],
    );
}

#[test]
fn asm_all_registers_keep_live_values_and_capture_outputs_before_writeback() {
    let variables = (1..=13)
        .map(|r| format!("var v{r}:u32={r};"))
        .collect::<String>();
    let bindings = (1..=13)
        .map(|r| format!("r{r}<->v{r}"))
        .collect::<Vec<_>>()
        .join(",");
    let body = (1..=13)
        .map(|r| format!("ADD r{r},r{r},r0,10;"))
        .collect::<String>();
    let sum = (1..=13)
        .map(|r| format!("v{r}"))
        .collect::<Vec<_>>()
        .join("+");
    let source = format!(
        "package main\nfunc main(){{var live:u32=42;{variables}asm({bindings}){{{body}}};*(536871168 as u32 as *u32)={sum}+live}}\n"
    );
    normal(
        "ASM.extra.all-registers",
        &["ASM-02", "ASM-04"],
        &source,
        &[263],
    );
}

#[test]
fn asm_large_frame_keeps_r12_r13_input_and_output_values() {
    normal(
        "ASM.extra.large-frame",
        &["ASM-02", "ASM-04"],
        r#"package main
func main(){
 var padding:[9000]u32
 padding[0]=7
 var a:u32=5;var b:u32=9
 asm(r12<->a,r13<->b){ADD r12,r12,r13,0;ADD r13,r13,r0,1}
 *(536871168 as u32 as *u32)=a*100+b+(padding[0] as u32)
}
"#,
        &[1417],
    );
}

#[test]
fn asm_defined_types_raw_and_function_pointers_use_scalar_bits() {
    normal(
        "ASM.extra.defined-pointers",
        &["ASM-01", "ASM-02"],
        r#"package main
type Number u16
type Pointer *u32
func answer()->u32{return 23}
func main(){
 var value:Number=65535 as Number
 var result:Number
 var pointer:Pointer=536871168 as u32 as *u32 as Pointer
 var callable:func()->u32
 asm(r1<-value,r2->result,r3<->pointer,r4<-answer,r5->callable){ADD r2,r1,r0,2;ADD r3,r3,r0,0;ADD r5,r4,r0,0}
 *(pointer as *u32)=(result as u32)+callable()
}
"#,
        &[24],
    );
}

#[test]
fn asm_noreturn_large_frame_sets_sp_last_and_preserves_lr_input() {
    let source = r#"package main
func boot(){*(536871168 as u32 as *u32)=42;for{}}
func stack()->u32{return 8388608}
func main(){
 var padding:[9000]u32
 padding[0]=1
 asm noreturn(r14<-stack(),r2<-536871172,r15<-boot,r12<-12,r13<-13){
 STW r14,r2,r0,0
 STW r12,r2,r0,4
 STW r13,r2,r0,8
 JZ r0,r0,r15,0
 }
}
"#;
    let image = compile(&[("main.ond", source)]);
    let (mut cpu, mut bus, _) = observed_machine(&image);
    for _ in 0..STEP_LIMIT {
        cpu.step(&mut bus).unwrap();
        assert_eq!(bus.read32(EXIT).unwrap(), SENTINEL);
    }
    output(&mut bus, &[42, 0x00800000, 12, 13]);
}

#[test]
fn asm_instruction_debug_ranges_and_noreturn_unwind_match_source() {
    use super::super::tests::Fixture;
    use ond_protocol::debug::DebugOperation;
    let source = "package main\nfunc main(){\n var address:u32=536870912\n asm(r1<-address){\n ADD r2,r0,r0,1\n STW r2,r1,r0,0\n LDW r3,r1,r0,0\n }\n}\n";
    let fixture = Fixture::new(source);
    let compilation = crate::compile_project(&fixture.0).unwrap();
    let ranges = &compilation.debug_objects[0].ranges;
    for (line, operation) in [
        (5, DebugOperation::Source),
        (6, DebugOperation::MemoryWrite),
        (7, DebugOperation::MemoryRead),
    ] {
        let range = ranges
            .iter()
            .find(|range| {
                range
                    .source
                    .as_ref()
                    .is_some_and(|source| source.line == line && source.column == 2)
            })
            .expect("explicit asm instruction must have its own source location");
        assert_eq!(range.end - range.start, 4);
        assert_eq!(range.operation, operation);
        assert!(
            !ranges.iter().any(|other| other.start <= range.start
                && other.end >= range.end
                && other.start != range.start
                && other.operation != operation),
            "asm must not combine different operations in one range"
        );
    }
    let fixture = Fixture::new(ASM_07);
    let compilation = crate::compile_project(&fixture.0).unwrap();
    let debug = compilation
        .debug_objects
        .iter()
        .flat_map(|object| &object.functions)
        .find(|function| function.symbol == "main.main")
        .unwrap();
    assert!(
        debug.unwind.is_empty(),
        "noreturn new SP has no representable CFA rule"
    );
}

#[test]
fn asm_explicit_alias_store_survives_input_only_binding() {
    normal(
        "ASM.extra.input-alias-store",
        &["ASM-02", "ASM-04"],
        r#"package main
func main(){var value:u32=5;asm(r1<-value,r2<-&value,r3<-11){STW r3,r2,r0,0;ADD r1,r1,r0,3};*(536871168 as u32 as *u32)=value}
"#,
        &[11],
    );
}

#[test]
fn asm_noreturn_without_sp_input_keeps_the_current_frame_sp() {
    use super::super::tests::Fixture;
    let source = "package main\nfunc boot(){*(536871168 as u32 as *u32)=42;for{}}\nfunc main(){asm noreturn(r1<-boot,r2<-536871172){STW r14,r2,r0,0;JZ r0,r0,r1,0}}\n";
    let fixture = Fixture::new(source);
    let compilation = crate::compile_project(&fixture.0).unwrap();
    let frame_size = compilation
        .debug_objects
        .iter()
        .flat_map(|object| &object.functions)
        .find(|function| function.symbol == "main.main")
        .unwrap()
        .frame_size;
    let image =
        crate::test_support::build_image(&compilation.mir, super::runner::config()).unwrap();
    let (mut cpu, mut bus, _) = observed_machine(&image);
    let mut entry_sp = None;
    for _ in 0..STEP_LIMIT {
        if cpu.pc() == image.symbols["main.main"] {
            entry_sp = Some(cpu.reg(14));
        }
        cpu.step(&mut bus).unwrap();
        assert_eq!(bus.read32(EXIT).unwrap(), SENTINEL);
    }
    output(
        &mut bus,
        &[42, entry_sp.expect("main entry reached") - frame_size],
    );
}
