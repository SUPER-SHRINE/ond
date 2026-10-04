//! Observe actual Kagura bus accesses, not only the final aggregate value.
use super::tests::{Fixture, machine};
use kagura::{Bus, BusFault, DefaultBus};
use ram::Ram;
const BASE: u32 = 0x20000000;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Access {
    write: bool,
    offset: u32,
    width: u32,
}
struct TraceBus {
    inner: DefaultBus,
    trace: Vec<Access>,
    fault: Option<Access>,
}
impl TraceBus {
    fn access(&mut self, write: bool, addr: u32, width: u32) -> Result<(), BusFault> {
        if (BASE..BASE + 64).contains(&addr) {
            let a = Access {
                write,
                offset: addr - BASE,
                width,
            };
            self.trace.push(a);
            if self.fault == Some(a) {
                return Err(BusFault);
            }
        }
        Ok(())
    }
}
macro_rules! read {
    ($name:ident,$ty:ty,$width:expr) => {
        fn $name(&mut self, addr: u32) -> Result<$ty, BusFault> {
            self.access(false, addr, $width)?;
            self.inner.$name(addr)
        }
    };
}
macro_rules! write {
    ($name:ident,$ty:ty,$width:expr) => {
        fn $name(&mut self, addr: u32, v: $ty) -> Result<(), BusFault> {
            self.access(true, addr, $width)?;
            self.inner.$name(addr, v)
        }
    };
}
impl Bus for TraceBus {
    read!(read8, u8, 1);
    read!(read16, u16, 2);
    read!(read32, u32, 4);
    write!(write8, u8, 1);
    write!(write16, u16, 2);
    write!(write32, u32, 4);
}
fn run(source: &str, fault: Option<Access>) -> (TraceBus, bool) {
    let f = Fixture::new(source);
    let bytes = crate::test_support::compile_image(&f.0).unwrap();
    let (mut cpu, mut inner, _) = machine(&bytes);
    inner.map_device(BASE, 64, Ram::new(64)).unwrap();
    for n in 0..64 {
        inner.write8(BASE + n, 0xcc).unwrap();
    }
    inner.write8(BASE, 11).unwrap();
    inner.write16(BASE + 2, 1234).unwrap();
    inner.write8(BASE + 4, 22).unwrap();
    let mut bus = TraceBus {
        inner,
        trace: vec![],
        fault,
    };
    for _ in 0..200000 {
        if cpu.step(&mut bus).is_err() {
            return (bus, true);
        }
        if bus.inner.read32(0xffff8040).unwrap() != 0xdeadbeef {
            return (bus, false);
        }
    }
    panic!("CPU step limit");
}
const COPY: &str = "package main\ntype S struct{a:u8;b:u16;c:u8;}\nfunc main(){var p=536870912 as *S;var q=536870928 as *S;*q=*p;}";
fn accesses() -> Vec<Access> {
    vec![
        Access {
            write: false,
            offset: 0,
            width: 1,
        },
        Access {
            write: false,
            offset: 2,
            width: 2,
        },
        Access {
            write: false,
            offset: 4,
            width: 1,
        },
        Access {
            write: true,
            offset: 16,
            width: 1,
        },
        Access {
            write: true,
            offset: 18,
            width: 2,
        },
        Access {
            write: true,
            offset: 20,
            width: 1,
        },
    ]
}
#[test]
fn ordered_copy_reads_snapshot_before_writes_and_ignores_padding() {
    let (mut bus, fault) = run(COPY, None);
    assert!(!fault);
    assert_eq!(bus.trace, accesses());
    assert_eq!(bus.inner.read8(BASE + 16).unwrap(), 11);
    assert_eq!(bus.inner.read16(BASE + 18).unwrap(), 1234);
    assert_eq!(bus.inner.read8(BASE + 20).unwrap(), 22);
    assert_eq!(bus.inner.read8(BASE + 17).unwrap(), 0xcc);
    assert_eq!(bus.inner.read8(BASE + 21).unwrap(), 0xcc);
}
#[test]
fn source_fault_never_starts_destination_and_write_fault_does_not_rollback() {
    let all = accesses();
    let (mut bus, fault) = run(COPY, Some(all[1]));
    assert!(fault);
    assert_eq!(bus.trace, all[..2]);
    for i in 16..22 {
        assert_eq!(bus.inner.read8(BASE + i).unwrap(), 0xcc);
    }
    let (mut bus, fault) = run(COPY, Some(all[4]));
    assert!(fault);
    assert_eq!(bus.trace, all[..5]);
    assert_eq!(bus.inner.read8(BASE + 16).unwrap(), 11);
    assert_eq!(bus.inner.read8(BASE + 18).unwrap(), 0xcc);
    assert_eq!(bus.inner.read8(BASE + 20).unwrap(), 0xcc);
}
#[test]
fn overlapping_aggregate_copy_uses_pre_write_snapshot() {
    let (mut bus, fault) = run(
        "package main\nfunc main(){var p=536870912 as *[3]u8;var q=536870913 as *[3]u8;*q=*p;}",
        None,
    );
    assert!(!fault);
    assert_eq!(
        bus.trace,
        vec![
            Access {
                write: false,
                offset: 0,
                width: 1
            },
            Access {
                write: false,
                offset: 1,
                width: 1
            },
            Access {
                write: false,
                offset: 2,
                width: 1
            },
            Access {
                write: true,
                offset: 1,
                width: 1
            },
            Access {
                write: true,
                offset: 2,
                width: 1
            },
            Access {
                write: true,
                offset: 3,
                width: 1
            }
        ]
    );
    assert_eq!(bus.inner.read8(BASE + 1).unwrap(), 11);
    assert_eq!(bus.inner.read8(BASE + 2).unwrap(), 0xcc);
    assert_eq!(bus.inner.read8(BASE + 3).unwrap(), 1234u16 as u8);
}
#[test]
fn bool_load_normalizes_and_bool_store_uses_one_byte() {
    let (mut bus, fault) = run(
        "package main\nfunc main(){var p=536870912 as *bool;var q=536870928 as *bool;*q=*p;}",
        None,
    );
    assert!(!fault);
    assert_eq!(
        bus.trace,
        vec![
            Access {
                write: false,
                offset: 0,
                width: 1
            },
            Access {
                write: true,
                offset: 16,
                width: 1
            }
        ]
    );
    assert_eq!(bus.inner.read8(BASE + 16).unwrap(), 1);
    assert_eq!(bus.inner.read8(BASE + 17).unwrap(), 0xcc);
}

#[test]
fn nested_array_order_raw_offset_wrap_and_alignment_fault() {
    let (_, fault) = run(
        "package main\nfunc main(){var p=536870913 as *u16;var x=*p;}",
        None,
    );
    assert!(fault);
    let (bus, fault) = run(
        "package main\nfunc main(){var p=536870913 as *u8;var x=p[4294967295];}",
        None,
    );
    assert!(!fault);
    assert_eq!(
        bus.trace,
        vec![Access {
            write: false,
            offset: 0,
            width: 1
        }]
    );
    let (bus, fault) = run(
        "package main\ntype S struct{a:u8;b:u16;c:u8;}\nfunc main(){var p=536870912 as *[2]S;var q=536870928 as *[2]S;*q=*p;}",
        None,
    );
    assert!(!fault);
    let expected = [(0, 1), (2, 2), (4, 1), (6, 1), (8, 2), (10, 1)];
    let reads = expected.iter().map(|(o, w)| Access {
        write: false,
        offset: *o,
        width: *w,
    });
    let writes = expected.iter().map(|(o, w)| Access {
        write: true,
        offset: 16 + o,
        width: *w,
    });
    assert_eq!(bus.trace, reads.chain(writes).collect::<Vec<_>>());
}
