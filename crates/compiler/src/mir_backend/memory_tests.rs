//! CPU execution tests for target layout, aliasing, and the Indirect ABI.
use super::{
    tests::{Fixture, machine, run},
    *,
};
use kagura::Bus;

#[test]
fn pointer_locals_narrow_accesses_and_offsets_execute() {
    run(
        "func calc() -> u32 { var a:u8 = 1; var p = &a; *p = 255; return a as u32 }",
        "calc()",
        255,
    );
    run(
        "func calc() -> i32 { var a:i16 = -1; var p = &a; *p = -123; return a as i32 }",
        "calc()",
        (-123i32) as u32,
    );
    run(
        "func calc() -> u32 { var a = [3]u16{10,20,30}; var p = &a[0]; p[1] = 500; return a[0] as u32 + a[1] as u32 + p[2] as u32 }",
        "calc()",
        540,
    );
    run(
        "func calc() -> u32 { var a:u8=3; var p=&a; var pp=&p; **pp=7; return a as u32 }",
        "calc()",
        7,
    );
    run(
        "func calc() -> bool { var p:*u8=nil; return p==nil }\nfunc test() -> u32 { if calc() { return 1 }; return 0 }",
        "test()",
        1,
    );
}

#[test]
fn natural_struct_layout_nested_arrays_and_byte_literals_execute() {
    run(
        "type S struct { a:u8; b:u16; c:u8; }\nfunc calc() -> u32 { var a:[2]S; a[1].b=1234; var p=&a[0]; var q=&a[1]; return (q as u32 - p as u32)*10000 + a[1].b as u32 }",
        "calc()",
        61234,
    );
    run(
        "type S struct { a:u8; b:u16; c:u8; }\nfunc calc() -> u32 { var s:S; return (((&s.b) as u32)-((&s.a) as u32))*10+(((&s.c) as u32)-((&s.a) as u32)) }",
        "calc()",
        24,
    );
    run(
        "func calc() -> u32 { var text = \"abc\"; var p=&text[0]; p[1]=90; return text[0] as u32 + text[1] as u32 + text[2] as u32 }",
        "calc()",
        286,
    );
    run(
        "type S struct { a:[2][2]u16; }\nfunc calc() -> u32 { var s:S; s.a[1][0]=300; return s.a[1][0] as u32 + s.a[0][1] as u32 }",
        "calc()",
        300,
    );
}

#[test]
fn aggregate_arguments_returns_and_snapshots_execute() {
    run(
        "func change(a:[2]i32) -> [2]i32 { a[0]=99; return a }\nfunc calc() -> i32 { var a=[2]i32{1,2}; var b=change(a); return a[0]*100+b[0]+b[1] }",
        "calc()",
        201,
    );
    run(
        "func tick(p:*i32) -> i32 { *p=*p+1; return *p }\nfunc take(a:[2]i32,x:i32) -> i32 { return a[0]*100+x }\nfunc calc() -> i32 { var a=[2]i32{5,6}; return take(a,tick(&a[0])) }",
        "calc()",
        506,
    );
    run(
        "func make(a:i32,b:i32,c:i32,d:i32,e:i32,f:i32,g:[2]i32) -> [2]i32 { return [2]i32{a+b+c+d+e+f,g[1]} }\nfunc calc() -> i32 { var a=make(1,2,3,4,5,6,[2]i32{7,8}); return a[0]*10+a[1] }",
        "calc()",
        218,
    );
    run(
        "func recurse(a:[2]i32,n:i32) -> [2]i32 { if n==0 { return a }; a[0]=a[0]+1; return recurse(a,n-1) }\nfunc calc() -> i32 { var a=recurse([2]i32{4,5},3); return a[0]*10+a[1] }",
        "calc()",
        75,
    );
    run(
        "func calc() -> i32 { var a=[2]i32{1,2}; a[0],a[1]=a[1],a[0]; return a[0]*10+a[1] }",
        "calc()",
        21,
    );
}

#[test]
fn mixed_memory_returns_and_register_returns_execute() {
    run(
        "func values() -> ([1]u8,u8,u16,i32) { return [1]u8{9},7,500,-10 }\nfunc calc() -> i32 { var a,b,c,d=values(); return a[0] as i32+b as i32+c as i32+d }",
        "calc()",
        506,
    );
    run(
        "func values() -> (u8,[3]u8,u16) { return 7,\"abc\",500 }\nfunc calc() -> u32 { var a,b,c=values(); return a as u32+b[1] as u32+c as u32 }",
        "calc()",
        605,
    );
    run(
        "func values() -> (i8,u16,bool,i32) { return -1,500,true,-10 }\nfunc calc() -> i32 { var a,b,c,d=values(); if c { return a as i32+b as i32+d }; return 0 }",
        "calc()",
        489,
    );
    run(
        "func values() -> (u8,u8,u8,u8,u8) { return 1,2,3,4,5 }\nfunc calc() -> u32 { var a,b,c,d,e=values(); return a as u32+b as u32+c as u32+d as u32+e as u32 }",
        "calc()",
        15,
    );
    run(
        "func make(a:[2]u8) -> [2]u8 { a[1]=9; return a }\nfunc calc() -> u32 { var f:func([2]u8)->[2]u8=make; var a=f([2]u8{4,5}); return a[0] as u32+a[1] as u32 }",
        "calc()",
        13,
    );
}

#[test]
fn zero_size_storage_is_non_nil_and_layout_remains_zero() {
    run(
        "type Z struct {}\nfunc pass(z:Z) -> Z { return z }\nfunc calc() -> u32 { var z:Z; var a=pass(z); if &a!=nil { return 1 }; return 0 }",
        "calc()",
        1,
    );
    run(
        "func pass(z:[0]u16) -> [0]u16 { return z }\nfunc calc() -> u32 { var a:[0]u16; var b=pass(a); if &b!=nil { return 1 }; return 0 }",
        "calc()",
        1,
    );
    let fixture =
        Fixture::new("package main\ntype Z struct {}\nfunc main(){var z:Z;var a:[0]u16;}");
    let p = fixture.core();
    let f = &p.packages[0].functions[0];
    for l in &f.locals {
        let d = layout::data(&p.types, l.ty, l.span).unwrap();
        assert_eq!(d.size, 0);
        assert_eq!(
            d.alignment,
            if matches!(
                p.types.underlying_kind(l.ty),
                Some(mir::TypeKind::Struct(_))
            ) {
                1
            } else {
                2
            }
        );
    }
}

#[test]
fn bounds_fault_precedes_destination_write() {
    let f = Fixture::new(
        "package main\nfunc set(i:u32){var a=[2]u8{1,2};a[i]=99;*(4294934608 as u32 as *u32) = 123}\nfunc main(){set(2)}",
    );
    let bytes = crate::test_support::compile_image(&f.0).unwrap();
    let (mut cpu, mut bus, _) = machine(&bytes);
    let fault = (0..20000)
        .find_map(|_| cpu.step(&mut bus).err())
        .expect("bounds fault");
    assert!(format!("{fault:?}").contains("InvalidInstruction"));
    assert_eq!(bus.read32(0xffff8050).unwrap(), 0);
}

fn assignment_fault(image: &crate::linker::LinkedImage) -> (kagura::DefaultBus, u32) {
    let (mut cpu, mut bus, _) = machine(image);
    for _ in 0..20000 {
        let pc = cpu.pc();
        if let Err(fault) = cpu.step(&mut bus) {
            assert!(format!("{fault:?}").contains("InvalidInstruction"));
            return (bus, pc);
        }
    }
    panic!("expected assignment fault");
}

#[test]
fn assignment_bounds_check_follows_rhs_effects() {
    for index in ["2", "-1"] {
        let f = Fixture::new(&format!(
            "package main\nfunc rhs()->u32{{*(4294934608 as u32 as *u32)=123;return 99}}\nfunc main(){{var a:[2]u32;var i:i32={index};a[i]=rhs()}}"
        ));
        let image = crate::test_support::compile_image(&f.0).unwrap();
        let (mut bus, _) = assignment_fault(&image);
        assert_eq!(bus.read32(0xffff8050).unwrap(), 123, "index {index}");
    }
}

#[test]
fn assignment_bounds_check_preserves_earlier_stores() {
    let f = Fixture::new(
        "package main\nvar A:[2]u32\nvar Later:u32\nfunc rhs()->u32{*(4294934608 as u32 as *u32)=123;return 99}\nfunc main(){var i:u32=2;A[0],A[i],Later=7,rhs(),8}",
    );
    let image = crate::test_support::compile_image(&f.0).unwrap();
    let (mut bus, _) = assignment_fault(&image);
    assert_eq!(bus.read32(0xffff8050).unwrap(), 123);
    assert_eq!(bus.read32(image.symbols["main.A"]).unwrap(), 7);
    assert_eq!(bus.read32(image.symbols["main.Later"]).unwrap(), 0);
}

#[test]
fn assignment_bounds_check_follows_all_lhs_operands_and_rhs_values() {
    let f = Fixture::new(
        "package main\nvar Trace:u32\nvar A:[2]u32\nfunc index(tag:u32,i:u32)->u32{Trace=Trace*10+tag;return i}\nfunc pointer()->*u32{Trace=Trace*10+2;return &A[0]}\nfunc rhs(tag:u32)->u32{Trace=Trace*10+tag;*(4294934608 as u32 as *u32)=Trace;return tag}\nfunc main(){A[index(1,2)],*pointer()=rhs(3),rhs(4)}",
    );
    let image = crate::test_support::compile_image(&f.0).unwrap();
    let (mut bus, _) = assignment_fault(&image);
    assert_eq!(bus.read32(0xffff8050).unwrap(), 1234);
    assert_eq!(bus.read32(image.symbols["main.A"]).unwrap(), 0);
}

#[test]
fn assignment_bounds_check_defers_nested_destination_projections() {
    for (outer, inner) in [(2, 1), (1, 2)] {
        let f = Fixture::new(&format!(
            "package main\ntype S struct{{values:[2]u32}}\nvar Trace:u32\nfunc index(tag:u32,i:u32)->u32{{Trace=Trace*10+tag;return i}}\nfunc rhs()->u32{{Trace=Trace*10+3;*(4294934608 as u32 as *u32)=Trace;return 99}}\nfunc main(){{var a:[2]S;a[index(1,{outer})].values[index(2,{inner})]=rhs()}}"
        ));
        let image = crate::test_support::compile_image(&f.0).unwrap();
        let (mut bus, _) = assignment_fault(&image);
        assert_eq!(bus.read32(0xffff8050).unwrap(), 123);
    }
}

#[test]
fn assignment_bounds_check_precedes_rhs_for_compound_assignment() {
    let f = Fixture::new(
        "package main\nfunc rhs()->u32{*(4294934608 as u32 as *u32)=123;return 99}\nfunc main(){var a:[2]u32;var i:u32=2;a[i]+=rhs()}",
    );
    let image = crate::test_support::compile_image(&f.0).unwrap();
    let (mut bus, _) = assignment_fault(&image);
    assert_eq!(bus.read32(0xffff8050).unwrap(), 0);
}

#[test]
fn assignment_bounds_check_remains_immediate_in_lhs_operand_reads() {
    for body in [
        "var a:[2]u32;var p=[1]*u32{&a[0]};var i:u32=1;*p[i]=rhs()",
        "var a:[2]u32;var i:u32=2;*(&a[i])=rhs()",
        "var a:[2]u32;var indices:[1]u32;var i:u32=1;a[indices[i]]=rhs()",
        "var a:[2]u32;var i:u32=2;a[0],a[1]=a[i],rhs()",
    ] {
        let f = Fixture::new(&format!(
            "package main\nfunc rhs()->u32{{*(4294934608 as u32 as *u32)=123;return 99}}\nfunc main(){{{body}}}"
        ));
        let image = crate::test_support::compile_image(&f.0).unwrap();
        let (mut bus, _) = assignment_fault(&image);
        assert_eq!(bus.read32(0xffff8050).unwrap(), 0, "{body}");
    }
}

#[test]
fn assignment_bounds_check_follows_rhs_division_fault() {
    use ond_protocol::debug::DebugOperation;
    let f = Fixture::new(
        "package main\nfunc main(){var a:[2]u32;var i:u32=2;var one:u32=1;var zero:u32=0;a[i]=one/zero}",
    );
    let image = crate::test_support::compile_image(&f.0).unwrap();
    let (_, pc) = assignment_fault(&image);
    let project = f.core();
    let output = codegen_selected_with_debug(
        &project,
        &project
            .packages
            .iter()
            .map(|p| p.logical_path.clone())
            .collect(),
        None,
    )
    .unwrap();
    let debug = &output.debug[0];
    let function = debug
        .functions
        .iter()
        .find(|f| f.symbol == "main.main")
        .unwrap();
    let offset = pc - image.symbols["main.main"] + function.offset;
    assert!(debug.ranges.iter().any(|range| {
        range.start <= offset
            && offset < range.end
            && range.operation == DebugOperation::DivisionByZero
    }));
}

#[test]
fn layout_and_abi_have_independent_fixed_expectations() {
    use layout::Argument::{Register, Stack};
    let mut types = mir::TypeTable::new();
    let span = Span::synthetic();
    let bytes = types.intern(
        mir::TypeKind::Array {
            length: 3,
            element: mir::TypeId::U8,
        },
        span,
    );
    let plan = layout::abi(
        &types,
        &mir::FunctionType {
            parameters: vec![mir::TypeId::U32; 7],
            returns: vec![mir::TypeId::U8, bytes, mir::TypeId::U16],
        },
        span,
    )
    .unwrap();
    assert_eq!(plan.hidden, Some(Register(1)));
    assert_eq!(
        plan.arguments,
        vec![
            Register(2),
            Register(3),
            Register(4),
            Register(5),
            Register(6),
            Stack(0),
            Stack(4)
        ]
    );
    assert_eq!(plan.stack_bytes, 8);
    assert_eq!(plan.return_offsets, vec![0, 4, 8]);
    assert_eq!(plan.return_bytes, 12);
    let byte = types.intern(
        mir::TypeKind::Array {
            length: 1,
            element: mir::TypeId::U8,
        },
        span,
    );
    let plan = layout::abi(
        &types,
        &mir::FunctionType {
            parameters: vec![],
            returns: vec![byte, mir::TypeId::U8, mir::TypeId::U16, mir::TypeId::I32],
        },
        span,
    )
    .unwrap();
    assert_eq!(plan.return_offsets, vec![0, 1, 6, 12]);
    assert_eq!(plan.return_bytes, 16);
    let huge = types.intern(
        mir::TypeKind::Array {
            length: u32::MAX,
            element: mir::TypeId::U32,
        },
        span,
    );
    assert!(
        layout::data(&types, huge, span)
            .unwrap_err()
            .message
            .contains("overflow")
    );
    let f = Fixture::new(
        "package main\ntype S struct{a:u8;b:u16;c:u8;}\ntype Node struct{value:u8;next:*Node;}\nfunc main(){var s:S;var n:Node;}",
    );
    let p = f.core();
    let main = &p.packages[0].functions[0];
    for (name, size, align, fields) in [("s", 6, 2, vec![0, 2, 4]), ("n", 8, 4, vec![0, 4])] {
        let l = main
            .locals
            .iter()
            .find(|l| l.name.as_deref() == Some(name))
            .unwrap();
        let d = layout::data(&p.types, l.ty, l.span).unwrap();
        assert_eq!((d.size, d.alignment, d.fields), (size, align, fields));
    }
}
