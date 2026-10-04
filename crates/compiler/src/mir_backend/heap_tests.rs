use super::tests::run;
use super::tests::{Fixture, machine};
use kagura::Bus;
#[test]
fn allocation_zeroing_reuse_and_explicit_initialization_execute() {
    run(
        "type S struct{a:u8;b:u16;c:[2]u32;}\nvar G:*S\nfunc calc()->u32{G=new(S);if G==nil{return 999};var value=G.a as u32+G.b as u32+G.c[1];G.c[0]=77;value=value+G.c[0];free(G);return value}",
        "calc()",
        77,
    );
    run(
        "func calc()->u32{var a=alloc[u8](16);a[0]=99;free(a);var b=new([16]u8);var value=(*b)[0] as u32;if (b as u32)==(a as u32){value=value+1};free(b);return value}",
        "calc()",
        1,
    );
    run(
        "type Z struct{}\nfunc calc()->u32{var a=new(Z);var b=new([0]u16);var c=alloc[u8](0);free(c);var ok:u32;if a!=nil && b!=nil && c==nil{ok=1};free(a);free(b);return ok}",
        "calc()",
        1,
    );
}
#[test]
fn first_fit_splitting_coalescing_and_tail_reclamation_execute() {
    run(
        "func calc()->u32{var a=alloc[u8](32);var b=alloc[u8](8);free(a);var c=alloc[u8](8);var d=alloc[u8](8);var score:u32;if c==a{score=score+1};if (d as u32)==(c as u32)+12{score=score+2};free(c);free(d);var e=alloc[u8](32);if e==a{score=score+4};free(e);free(b);var f=alloc[u8](48);if f==a{score=score+8};free(f);return score}",
        "calc()",
        15,
    );
    run(
        "func calc()->u32{var a=alloc[u8](8);var b=alloc[u8](8);var c=alloc[u8](8);var d=alloc[u8](8);free(a);free(c);free(b);var e=alloc[u8](32);var score:u32;if e==a{score=1};free(e);free(d);return score}",
        "calc()",
        1,
    );
}
#[test]
fn failed_allocation_and_overflow_preserve_existing_memory() {
    run(
        "func calc()->u32{var a=alloc[u8](8);a[0]=42;var b=alloc[u8](4294967295);var c=alloc[u8](65536);var d=new([65536]u8);var result=a[0] as u32;if b==nil && c==nil && d==nil{result=result+1};free(a);return result}",
        "calc()",
        43,
    );
    run(
        "func calc()->u32{var a=alloc[u8](4);var b=alloc[u8](4);var c=alloc[u8](4);var value=((a as u32)|(b as u32)|(c as u32))&3;free(a);free(b);free(c);return value}",
        "calc()",
        0,
    );
}

#[test]
fn heap_can_fill_exactly_to_stack_bottom_and_recover_after_oom() {
    let f = Fixture::new(
        "package main\ntype Z struct{}\nfunc main(){var first=alloc[u8](1);free(first);var n:u32=61440-(first as u32);var full=alloc[u8](n);full[n-1]=165;var empty=new(Z);var score:u32;if empty==nil{score=1};free(full);var again=new(Z);if again!=nil{score=score+2};free(again);store32(4294934608,score)}",
    );
    let bytes = crate::test_support::compile_image(&f.0).unwrap();
    let (mut cpu, mut bus, top) = machine(&bytes);
    let bottom = top - 8192;
    bus.write32(bottom, 0xfeedcafe).unwrap();
    let mut done = false;
    for _ in 0..200000 {
        cpu.step(&mut bus).unwrap();
        if bus.read32(0xffff8040).unwrap() != 0xdeadbeef {
            done = true;
            break;
        }
    }
    assert!(done);
    assert_eq!(bus.read32(0xffff8050).unwrap(), 3);
    assert_eq!(bus.read32(bottom).unwrap(), 0xfeedcafe);
    assert_eq!(bus.read8(bottom - 1).unwrap(), 165);
    assert_eq!(cpu.reg(14), top);
}
