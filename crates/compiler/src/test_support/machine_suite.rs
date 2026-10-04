// Standard MIR compilation through direct LinkedImage loading and test-only devices.
// Expectations are independent goldens, not results of another backend.
use super::{MachineStatus, LinkedMachine};
use kagura::Fault;
use std::env;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir() -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    env::temp_dir().join(format!("ond-linked-machine-{nanos}"))
}

#[test]
fn runs_minimal_linked_image() {
    let root = temp_dir();
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("ond.toml"), "[project]\nname=\"sample\"\n").unwrap();
    fs::write(root.join("main.ond"), "package main\n\nfunc main() {\n}\n").unwrap();

    let bytes = compile(&root).unwrap();
    let mut machine = LinkedMachine::from_image(&bytes).unwrap();
    let status = machine.run_steps(STEP_LIMIT).unwrap();
    assert_eq!(status, MachineStatus::Exited(0));
}

#[test]
fn counts_only_successfully_retired_instructions() {
    let root = temp_dir();
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("ond.toml"), "[project]\nname=\"counter\"\n").unwrap();
    fs::write(root.join("main.ond"), "package main\n\nfunc main() {\n}\n").unwrap();

    let bytes = compile(&root).unwrap();
    let mut machine = LinkedMachine::from_image(&bytes).unwrap();
    assert_eq!(machine.retired_instructions(), 0);

    machine.step().unwrap();
    assert_eq!(machine.retired_instructions(), 1);

    let status = machine.run_steps(STEP_LIMIT).unwrap();
    assert_eq!(status, MachineStatus::Exited(0));
    assert!(machine.retired_instructions() > 1);
    assert!(machine.retired_instructions() <= (STEP_LIMIT + 1) as u64);
}

#[test]
fn fibonacci_example_exits_and_writes_debug_output() {
    let root = temp_dir();
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("ond.toml"), "[project]\nname=\"fibonacci\"\n").unwrap();
    fs::write(
        root.join("main.ond"),
        "package main\n\nconst DebugIoData: u32 = 0xFFFF8050 as u32\n\nfunc writeByte(value: u8) {\n    var io: *u8 = DebugIoData as *u8\n    io[0 as u32] = value\n}\n\nfunc main() {\n    var a: u32 = 0 as u32\n    var b: u32 = 1 as u32\n    var index: u32 = 0 as u32\n    for index < 6 as u32 {\n        writeByte((a + (48 as u32)) as u8)\n        writeByte(10 as u8)\n        var next: u32 = a + b\n        a = b\n        b = next\n        index = index + (1 as u32)\n    }\n}\n",
    )
    .unwrap();
    let bytes = compile(&root).unwrap();
    let mut machine = LinkedMachine::from_image(&bytes).unwrap();
    let status = machine.run_steps(STEP_LIMIT).unwrap();
    let output = machine.take_debug_output();
    assert_eq!(
        status,
        MachineStatus::Exited(0),
        "pc=0x{:08X} r1=0x{:08X} r2=0x{:08X} r3=0x{:08X}",
        machine.pc(),
        machine.reg(1),
        machine.reg(2),
        machine.reg(3),
    );
    assert_eq!(output, b"0\n1\n1\n2\n3\n5\n");
}

#[test]
fn nil_function_pointer_call_faults() {
    let root = temp_dir();
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("ond.toml"), "[project]\nname=\"nil-fn\"\n").unwrap();
    fs::write(
        root.join("main.ond"),
        "package main\n\nfunc main() {\n    var callback: func(value: u32) -> u32 = nil\n    callback(1 as u32)\n}\n",
    )
    .unwrap();

    let bytes = compile(&root).unwrap();
    let mut machine = LinkedMachine::from_image(&bytes).unwrap();
    let fault = (0..STEP_LIMIT)
        .find_map(|_| {
            let before = machine.retired_instructions();
            match machine.step() {
                Ok(MachineStatus::Running) => {
                    assert_eq!(machine.retired_instructions(), before + 1);
                }
                Ok(status) => panic!("unexpected status: {status:?}"),
                Err(fault) => {
                    assert_eq!(machine.retired_instructions(), before);
                    return Some(fault);
                }
            }
            None
        })
        .expect("nil call must fault within step budget");
    assert_eq!(machine.status(), MachineStatus::Running);
    assert!(matches!(fault, Fault::BusFault { addr: 0, .. }));
    assert!(machine.debug_output().is_empty());
}

#[test]
fn array_bounds_faults() {
    let root = temp_dir();
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("ond.toml"), "[project]\nname=\"bounds\"\n").unwrap();
    fs::write(
        root.join("main.ond"),
        "package main\n\nfunc main() {\n    var local: [4 as u32]u8\n    var index: u32 = 4\n    local[index] = 1 as u8\n}\n",
    )
    .unwrap();

    let bytes = compile(&root).unwrap();
    let mut machine = LinkedMachine::from_image(&bytes).unwrap();
    let fault = machine.run_steps(STEP_LIMIT).unwrap_err();
    assert!(matches!(fault, Fault::InvalidInstruction { .. }));
    assert_eq!(machine.status(), MachineStatus::Running);
    assert!(machine.debug_output().is_empty());
}

#[test]
fn array_bounds_fixture_does_not_depend_on_legacy_string_lowering() {
    let root = temp_dir();
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("ond.toml"),
        "[project]\nname=\"byte-array-bounds\"\n",
    )
    .unwrap();
    fs::write(
        root.join("main.ond"),
        "package main\n\nfunc main() {\n    text := [1]u8{97 as u8}\n    var index: u32 = 1 as u32\n    var value: u8 = text[index]\n    if value == 0 as u8 {\n        return\n    }\n}\n",
    )
    .unwrap();

    let bytes = compile(&root).unwrap();
    let mut machine = LinkedMachine::from_image(&bytes).unwrap();
    let fault = machine.run_steps(STEP_LIMIT).unwrap_err();
    assert!(matches!(fault, Fault::InvalidInstruction { .. }));
    assert_eq!(machine.status(), MachineStatus::Running);
    assert!(machine.debug_output().is_empty());
}

#[test]
fn nested_call_arguments_preserve_previous_argument_values() {
    let root = temp_dir();
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("ond.toml"), "[project]\nname=\"nested-call\"\n").unwrap();
    fs::write(
        root.join("main.ond"),
        "package main\n\nfunc id(value: u32) -> u32 {\n    return value\n}\n\nfunc add(a: u32, b: u32, c: u32, d: u32, e: u32, f: u32) -> u32 {\n    return a + b + c + d + e + f\n}\n\nfunc fail() {\n    var local: [1 as u32]u8\n    var index: u32 = 1\n    local[index] = 1 as u8\n}\n\nfunc main() {\n    var result: u32 = add(1 as u32, id(2 as u32), 3 as u32, id(4 as u32), 5 as u32, id(6 as u32))\n    if result != 21 as u32 {\n        fail()\n    }\n}\n",
    )
    .unwrap();

    let bytes = compile(&root).unwrap();
    let mut machine = LinkedMachine::from_image(&bytes).unwrap();
    let status = machine.run_steps(STEP_LIMIT).unwrap();
    assert_eq!(status, MachineStatus::Exited(0));
}

#[test]
fn invalid_float_to_integer_cast_faults() {
    let root = temp_dir();
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("ond.toml"),
        "[project]\nname=\"float-cast-fault\"\n",
    )
    .unwrap();
    fs::write(
        root.join("main.ond"),
        "package main\n\nfunc main() {\n    var bad: f32 = 0.0 / 0.0\n    var value: u32 = bad as u32\n    if value == 0 as u32 {\n        return\n    }\n}\n",
    )
    .unwrap();

    let bytes = compile(&root).unwrap();
    let mut machine = LinkedMachine::from_image(&bytes).unwrap();
    let fault = machine.run_steps(STEP_LIMIT).unwrap_err();
    assert!(matches!(fault, Fault::InvalidInstruction { .. }));
    assert_eq!(machine.status(), MachineStatus::Running);
    assert!(machine.debug_output().is_empty());
}

#[test]
fn float_arithmetic_pipeline_exits_zero() {
    let root = temp_dir();
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("ond.toml"),
        "[project]\nname=\"float-pipeline\"\n",
    )
    .unwrap();
    fs::write(
        root.join("main.ond"),
        "package main\n\nfunc fail(addr: u32) {\n    var ptr: *u8 = addr as *u8\n    ptr[0 as u32] = 1 as u8\n}\n\nfunc main() {\n    var literal: f32 = 3.75\n    var literalPtr: *u8 = ((&literal) as u32) as *u8\n    var literalBits: u32 = (literalPtr[0 as u32] as u32) | ((literalPtr[1 as u32] as u32) << (8 as u32)) | ((literalPtr[2 as u32] as u32) << (16 as u32)) | ((literalPtr[3 as u32] as u32) << (24 as u32))\n    if literalBits != 0x4070_0000 as u32 {\n        fail(0x1000_0000 as u32)\n    }\n\n    var sum: f32 = 1.5 + 2.25\n    var sumPtr: *u8 = ((&sum) as u32) as *u8\n    var sumBits: u32 = (sumPtr[0 as u32] as u32) | ((sumPtr[1 as u32] as u32) << (8 as u32)) | ((sumPtr[2 as u32] as u32) << (16 as u32)) | ((sumPtr[3 as u32] as u32) << (24 as u32))\n    if sumBits != 0x4070_0000 as u32 {\n        fail(0x1000_0010 as u32)\n    }\n\n    var product: f32 = sum * 2.0\n    var productPtr: *u8 = ((&product) as u32) as *u8\n    var productBits: u32 = (productPtr[0 as u32] as u32) | ((productPtr[1 as u32] as u32) << (8 as u32)) | ((productPtr[2 as u32] as u32) << (16 as u32)) | ((productPtr[3 as u32] as u32) << (24 as u32))\n    if productBits != 0x40F0_0000 as u32 {\n        fail(0x1000_0002 as u32)\n    }\n\n    var difference: f32 = product - 1.25\n    var differencePtr: *u8 = ((&difference) as u32) as *u8\n    var differenceBits: u32 = (differencePtr[0 as u32] as u32) | ((differencePtr[1 as u32] as u32) << (8 as u32)) | ((differencePtr[2 as u32] as u32) << (16 as u32)) | ((differencePtr[3 as u32] as u32) << (24 as u32))\n    if differenceBits != 0x40C8_0000 as u32 {\n        fail(0x1000_0003 as u32)\n    }\n\n    var quotient: f32 = difference / 2.5\n    var quotientPtr: *u8 = ((&quotient) as u32) as *u8\n    var quotientBits: u32 = (quotientPtr[0 as u32] as u32) | ((quotientPtr[1 as u32] as u32) << (8 as u32)) | ((quotientPtr[2 as u32] as u32) << (16 as u32)) | ((quotientPtr[3 as u32] as u32) << (24 as u32))\n    if quotientBits != 0x4020_0000 as u32 {\n        fail(0x1000_0004 as u32)\n    }\n}\n",
    )
    .unwrap();

    let bytes = compile(&root).unwrap();
    let mut machine = LinkedMachine::from_image(&bytes).unwrap();
    let status = machine.run_steps(STEP_LIMIT).unwrap();
    assert_eq!(status, MachineStatus::Exited(0));
}

#[test]
fn float_add_outputs_raw_bytes() {
    let root = temp_dir();
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("ond.toml"),
        "[project]\nname=\"float-add-bytes\"\n",
    )
    .unwrap();
    fs::write(
        root.join("main.ond"),
        "package main\n\nconst DebugIoData: u32 = 0xFFFF8050 as u32\n\nfunc writeByte(value: u8) {\n    var io: *u8 = DebugIoData as *u8\n    io[0 as u32] = value\n}\n\nfunc main() {\n    var sum: f32 = 1.5 + 2.25\n    var ptr: *u8 = ((&sum) as u32) as *u8\n    writeByte(ptr[0 as u32])\n    writeByte(ptr[1 as u32])\n    writeByte(ptr[2 as u32])\n    writeByte(ptr[3 as u32])\n}\n",
    )
    .unwrap();

    let bytes = compile(&root).unwrap();
    let mut machine = LinkedMachine::from_image(&bytes).unwrap();
    let status = machine.run_steps(STEP_LIMIT).unwrap();
    let output = machine.take_debug_output();
    assert_eq!(status, MachineStatus::Exited(0));
    assert_eq!(output, vec![0x00, 0x00, 0x70, 0x40]);
}

#[test]
fn float_neg_outputs_expected_bytes() {
    let root = temp_dir();
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("ond.toml"),
        "[project]\nname=\"float-neg-bytes\"\n",
    )
    .unwrap();
    fs::write(
        root.join("main.ond"),
        "package main\n\nconst DebugIoData: u32 = 0xFFFF8050 as u32\n\nfunc writeByte(value: u8) {\n    var io: *u8 = DebugIoData as *u8\n    io[0 as u32] = value\n}\n\nfunc main() {\n    var value: f32 = -(1.5)\n    var ptr: *u8 = ((&value) as u32) as *u8\n    writeByte(ptr[0 as u32])\n    writeByte(ptr[1 as u32])\n    writeByte(ptr[2 as u32])\n    writeByte(ptr[3 as u32])\n}\n",
    )
    .unwrap();

    let bytes = compile(&root).unwrap();
    let mut machine = LinkedMachine::from_image(&bytes).unwrap();
    let status = machine.run_steps(STEP_LIMIT).unwrap();
    let output = machine.take_debug_output();
    assert_eq!(status, MachineStatus::Exited(0));
    assert_eq!(output, vec![0x00, 0x00, 0xC0, 0xBF]);
}

#[test]
fn float_from_i32_outputs_expected_bytes() {
    let root = temp_dir();
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("ond.toml"),
        "[project]\nname=\"float-from-i32-bytes\"\n",
    )
    .unwrap();
    fs::write(
        root.join("main.ond"),
        "package main\n\nconst DebugIoData: u32 = 0xFFFF8050 as u32\n\nfunc writeByte(value: u8) {\n    var io: *u8 = DebugIoData as *u8\n    io[0 as u32] = value\n}\n\nfunc main() {\n    var value: f32 = 3 as f32\n    var ptr: *u8 = ((&value) as u32) as *u8\n    writeByte(ptr[0 as u32])\n    writeByte(ptr[1 as u32])\n    writeByte(ptr[2 as u32])\n    writeByte(ptr[3 as u32])\n}\n",
    )
    .unwrap();

    let bytes = compile(&root).unwrap();
    let mut machine = LinkedMachine::from_image(&bytes).unwrap();
    let status = machine.run_steps(STEP_LIMIT).unwrap();
    let output = machine.take_debug_output();
    assert_eq!(status, MachineStatus::Exited(0));
    assert_eq!(output, vec![0x00, 0x00, 0x40, 0x40]);
}

#[test]
fn mixed_i32_u32_multi_return_exits_zero() {
    let root = temp_dir();
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("ond.toml"), "[project]\nname=\"mixed-ret\"\n").unwrap();
    fs::write(
        root.join("main.ond"),
        "package main\n\nfunc fail(addr: u32) {\n    var ptr: *u8 = addr as *u8\n    ptr[0 as u32] = 1 as u8\n}\n\nfunc decode(value: u32) -> (i32, u32) {\n    var expField: u32 = (value >> (23 as u32)) & (0xFF as u32)\n    return (expField as i32) - (127 as i32), value & (0x007F_FFFF as u32)\n}\n\nfunc main() {\n    var exp: i32\n    var frac: u32\n    exp, frac = decode(0x4010_0000 as u32)\n    if exp != 1 as i32 {\n        fail(0x1000_0101 as u32)\n    }\n    if frac != 0x0010_0000 as u32 {\n        fail(0x1000_0102 as u32)\n    }\n\n    var otherExp: i32\n    var otherFrac: u32\n    otherExp, otherFrac = decode(0x3FC0_0000 as u32)\n    if otherExp != 0 as i32 {\n        fail(0x1000_0103 as u32)\n    }\n    if otherFrac != 0x0040_0000 as u32 {\n        fail(0x1000_0104 as u32)\n    }\n    if otherExp < exp {\n        return\n    }\n    fail(0x1000_0105 as u32)\n}\n",
    )
    .unwrap();

    let bytes = compile(&root).unwrap();
    let mut machine = LinkedMachine::from_image(&bytes).unwrap();
    let status = machine.run_steps(STEP_LIMIT).unwrap();
    assert_eq!(status, MachineStatus::Exited(0));
}

#[test]
fn address_of_local_u32_supports_byte_reads() {
    let root = temp_dir();
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("ond.toml"),
        "[project]\nname=\"addr-local-u32\"\n",
    )
    .unwrap();
    fs::write(
        root.join("main.ond"),
        "package main\n\nfunc fail(addr: u32) {\n    var ptr: *u8 = addr as *u8\n    ptr[0 as u32] = 1 as u8\n}\n\nfunc main() {\n    var value: u32 = 0x1234_5678 as u32\n    var ptr: *u8 = ((&value) as u32) as *u8\n    if ptr[0 as u32] != 0x78 as u8 {\n        fail(0x1000_0201 as u32)\n    }\n    if ptr[1 as u32] != 0x56 as u8 {\n        fail(0x1000_0202 as u32)\n    }\n    if ptr[2 as u32] != 0x34 as u8 {\n        fail(0x1000_0203 as u32)\n    }\n    if ptr[3 as u32] != 0x12 as u8 {\n        fail(0x1000_0204 as u32)\n    }\n}\n",
    )
    .unwrap();

    let bytes = compile(&root).unwrap();
    let mut machine = LinkedMachine::from_image(&bytes).unwrap();
    let status = machine.run_steps(STEP_LIMIT).unwrap();
    assert_eq!(status, MachineStatus::Exited(0));
}

#[test]
fn address_of_local_f32_supports_byte_reads() {
    let root = temp_dir();
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("ond.toml"),
        "[project]\nname=\"addr-local-f32\"\n",
    )
    .unwrap();
    fs::write(
        root.join("main.ond"),
        "package main\n\nfunc fail(addr: u32) {\n    var ptr: *u8 = addr as *u8\n    ptr[0 as u32] = 1 as u8\n}\n\nfunc main() {\n    var value: f32 = 3.75\n    var ptr: *u8 = ((&value) as u32) as *u8\n    if ptr[0 as u32] != 0x00 as u8 {\n        fail(0x1000_0301 as u32)\n    }\n    if ptr[1 as u32] != 0x00 as u8 {\n        fail(0x1000_0302 as u32)\n    }\n    if ptr[2 as u32] != 0x70 as u8 {\n        fail(0x1000_0303 as u32)\n    }\n    if ptr[3 as u32] != 0x40 as u8 {\n        fail(0x1000_0304 as u32)\n    }\n}\n",
    )
    .unwrap();

    let bytes = compile(&root).unwrap();
    let mut machine = LinkedMachine::from_image(&bytes).unwrap();
    let status = machine.run_steps(STEP_LIMIT).unwrap();
    assert_eq!(status, MachineStatus::Exited(0));
}

#[test]
fn dynamic_shift_ops_on_u32_work() {
    let root = temp_dir();
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("ond.toml"), "[project]\nname=\"dynamic-shift\"\n").unwrap();
    fs::write(
        root.join("main.ond"),
        "package main\n\nfunc fail(addr: u32) {\n    var ptr: *u8 = addr as *u8\n    ptr[0 as u32] = 1 as u8\n}\n\nfunc main() {\n    var value: u32 = 0x0600_0000 as u32\n    var amount: u32 = 1 as u32\n    var shiftedRight: u32 = value >> amount\n    if shiftedRight != 0x0300_0000 as u32 {\n        fail(0x1000_0401 as u32)\n    }\n\n    var shiftedLeft: u32 = shiftedRight << amount\n    if shiftedLeft != value {\n        fail(0x1000_0402 as u32)\n    }\n}\n",
    )
    .unwrap();

    let bytes = compile(&root).unwrap();
    let mut machine = LinkedMachine::from_image(&bytes).unwrap();
    let status = machine.run_steps(STEP_LIMIT).unwrap();
    assert_eq!(status, MachineStatus::Exited(0));
}

#[test]
fn nested_bit_expression_on_u32_works() {
    let root = temp_dir();
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("ond.toml"),
        "[project]\nname=\"nested-bit-expr\"\n",
    )
    .unwrap();
    fs::write(
        root.join("main.ond"),
        "package main\n\nfunc fail(addr: u32) {\n    var ptr: *u8 = addr as *u8\n    ptr[0 as u32] = 1 as u8\n}\n\nfunc highestBit(value: u32) -> u32 {\n    var index: u32 = 31 as u32\n    for {\n        if (value & ((1 as u32) << index)) != 0 as u32 {\n            return index\n        }\n        if index == 0 as u32 {\n            return 0 as u32\n        }\n        index = index - (1 as u32)\n    }\n}\n\nfunc main() {\n    if highestBit(3 as u32) != 1 as u32 {\n        fail(0x1000_0501 as u32)\n    }\n    if highestBit(0x0400_0000 as u32) != 26 as u32 {\n        fail(0x1000_0502 as u32)\n    }\n}\n",
    )
    .unwrap();

    let bytes = compile(&root).unwrap();
    let mut machine = LinkedMachine::from_image(&bytes).unwrap();
    let status = machine.run_steps(STEP_LIMIT).unwrap();
    assert_eq!(status, MachineStatus::Exited(0));
}

#[test]
fn byte_reassembly_expression_works() {
    let root = temp_dir();
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("ond.toml"),
        "[project]\nname=\"byte-reassembly\"\n",
    )
    .unwrap();
    fs::write(
        root.join("main.ond"),
        "package main\n\nfunc fail(addr: u32) {\n    var ptr: *u8 = addr as *u8\n    ptr[0 as u32] = 1 as u8\n}\n\nfunc main() {\n    var value: u32 = 0x1234_5678 as u32\n    var ptr: *u8 = ((&value) as u32) as *u8\n    var rebuilt: u32 = (ptr[0 as u32] as u32) | ((ptr[1 as u32] as u32) << (8 as u32)) | ((ptr[2 as u32] as u32) << (16 as u32)) | ((ptr[3 as u32] as u32) << (24 as u32))\n    if rebuilt != value {\n        fail(0x1000_0601 as u32)\n    }\n}\n",
    )
    .unwrap();

    let bytes = compile(&root).unwrap();
    let mut machine = LinkedMachine::from_image(&bytes).unwrap();
    let status = machine.run_steps(STEP_LIMIT).unwrap();
    assert_eq!(status, MachineStatus::Exited(0));
}

#[test]
fn divide_by_zero_faults() {
    let root = temp_dir();
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("ond.toml"), "[project]\nname=\"div-zero\"\n").unwrap();
    fs::write(
        root.join("main.ond"),
        "package main\n\nfunc main() {\n    var zero: u32 = 0\n    var value: u32 = (1 as u32) / zero\n    if value == 0 as u32 {\n        return\n    }\n}\n",
    )
    .unwrap();

    let bytes = compile(&root).unwrap();
    let mut machine = LinkedMachine::from_image(&bytes).unwrap();
    let fault = machine.run_steps(STEP_LIMIT).unwrap_err();
    assert!(matches!(fault, Fault::InvalidInstruction { .. }));
    assert_eq!(machine.status(), MachineStatus::Running);
    assert!(machine.debug_output().is_empty());
}
