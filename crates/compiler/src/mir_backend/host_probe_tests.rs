//! F20 feasibility fixtures. Provisional test adapters, not a host ABI.
use kagura::{AccessOperation, AccessWidth, Bus, BusFault, Cpu, Device, Fault};
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

const RAM_BASE: u32 = 0x1000;
const RAM_SIZE: u32 = 0x100;
const GPU0_REGS: u32 = 0x8000;
const GPU0_VRAM: u32 = 0x8100;
const GPU1_REGS: u32 = 0x9000;
const GPU1_VRAM: u32 = 0x9100;
const WINDOW_SIZE: u32 = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Window {
    Gpu0Regs,
    Gpu0Vram,
    Gpu1Regs,
    Gpu1Vram,
}
impl Window {
    fn spec(self) -> (usize, u32, u8) {
        match self {
            Self::Gpu0Regs => (0, GPU0_REGS, 4),
            Self::Gpu0Vram => (0, GPU0_VRAM, 1 | 2 | 4),
            Self::Gpu1Regs => (1, GPU1_REGS, 4),
            Self::Gpu1Vram => (1, GPU1_VRAM, 1 | 2 | 4),
        }
    }
    fn at(addr: u32) -> Option<Self> {
        [
            Self::Gpu0Regs,
            Self::Gpu0Vram,
            Self::Gpu1Regs,
            Self::Gpu1Vram,
        ]
        .into_iter()
        .find(|w| {
            let (_, base, _) = w.spec();
            addr >= base && addr < base + WINDOW_SIZE
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Event {
    instance: usize,
    window: Window,
    offset: u32,
    width: u8,
    write: bool,
    value: u32,
}

struct Probe {
    value: u32,
    clear_on_read: bool,
    drops: Rc<RefCell<Vec<u32>>>,
    id: u32,
}
impl Probe {
    fn new(id: u32, drops: Rc<RefCell<Vec<u32>>>) -> Self {
        Self {
            value: 0,
            clear_on_read: false,
            drops,
            id,
        }
    }
    fn access(&mut self, offset: u32, width: u8, write: Option<u32>) -> Result<u32, BusFault> {
        if !matches!(width, 1 | 2 | 4) {
            return Err(BusFault);
        }
        if offset
            .checked_add(u32::from(width))
            .is_none_or(|end| end > WINDOW_SIZE)
            || !offset.is_multiple_of(u32::from(width))
        {
            return Err(BusFault);
        }
        let mask = match width {
            1 => 0xff,
            2 => 0xffff,
            4 => u32::MAX,
            _ => return Err(BusFault),
        };
        if let Some(v) = write {
            self.value = v & mask;
            Ok(self.value)
        } else {
            let v = self.value & mask;
            if self.clear_on_read {
                self.value = 0;
            }
            Ok(v)
        }
    }
}
impl Drop for Probe {
    fn drop(&mut self) {
        self.drops.borrow_mut().push(self.id);
    }
}
impl Device for Probe {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn read8(&mut self, o: u32) -> Result<u8, BusFault> {
        Ok(self.access(o, 1, None)? as u8)
    }
    fn read16(&mut self, o: u32) -> Result<u16, BusFault> {
        Ok(self.access(o, 2, None)? as u16)
    }
    fn read32(&mut self, o: u32) -> Result<u32, BusFault> {
        self.access(o, 4, None)
    }
    fn write8(&mut self, o: u32, v: u8) -> Result<(), BusFault> {
        self.access(o, 1, Some(u32::from(v))).map(|_| ())
    }
    fn write16(&mut self, o: u32, v: u16) -> Result<(), BusFault> {
        self.access(o, 2, Some(u32::from(v))).map(|_| ())
    }
    fn write32(&mut self, o: u32, v: u32) -> Result<(), BusFault> {
        self.access(o, 4, Some(v)).map(|_| ())
    }
}

/// One routing implementation for CPU, fixture checks, and fetch guard.
struct Router {
    ram: Vec<u8>,
    instances: [Probe; 2],
    events: Vec<Event>,
}
impl Router {
    fn new() -> Self {
        let drops = Rc::new(RefCell::new(Vec::new()));
        Self {
            ram: vec![0; RAM_SIZE as usize],
            instances: [Probe::new(1, drops.clone()), Probe::new(2, drops.clone())],
            events: Vec::new(),
        }
    }
    fn read(&mut self, addr: u32, width: u8) -> Result<u32, BusFault> {
        if (RAM_BASE..RAM_BASE + RAM_SIZE).contains(&addr) {
            return self.ram_access(addr, width, None);
        }
        self.window_access(addr, width, None)
    }
    fn write(&mut self, addr: u32, width: u8, value: u32) -> Result<(), BusFault> {
        if (RAM_BASE..RAM_BASE + RAM_SIZE).contains(&addr) {
            self.ram_access(addr, width, Some(value)).map(|_| ())
        } else {
            self.window_access(addr, width, Some(value)).map(|_| ())
        }
    }
    fn ram_access(&mut self, addr: u32, width: u8, write: Option<u32>) -> Result<u32, BusFault> {
        if !matches!(width, 1 | 2 | 4) {
            return Err(BusFault);
        }
        let off = addr.checked_sub(RAM_BASE).ok_or(BusFault)?;
        if off % u32::from(width) != 0
            || off
                .checked_add(u32::from(width))
                .is_none_or(|end| end > RAM_SIZE)
        {
            return Err(BusFault);
        }
        let i = off as usize;
        if let Some(v) = write {
            for n in 0..width {
                self.ram[i + n as usize] = (v >> (8 * n)) as u8;
            }
        }
        let mut result = 0;
        for n in 0..width {
            result |= u32::from(self.ram[i + n as usize]) << (8 * n);
        }
        Ok(result)
    }
    fn window_access(&mut self, addr: u32, width: u8, write: Option<u32>) -> Result<u32, BusFault> {
        let window = Window::at(addr).ok_or(BusFault)?;
        let (instance, base, allowed_width) = window.spec();
        let offset = addr.checked_sub(base).ok_or(BusFault)?;
        if !matches!(width, 1 | 2 | 4)
            || allowed_width & width == 0
            || offset % u32::from(width) != 0
            || offset
                .checked_add(u32::from(width))
                .is_none_or(|end| end > WINDOW_SIZE)
        {
            return Err(BusFault);
        }
        let value = self.instances[instance].access(offset, width, write)?;
        self.events.push(Event {
            instance,
            window,
            offset,
            width,
            write: write.is_some(),
            value: write.unwrap_or(value),
        });
        Ok(value)
    }
}
impl Bus for Router {
    fn read8(&mut self, a: u32) -> Result<u8, BusFault> {
        Ok(self.read(a, 1)? as u8)
    }
    fn read16(&mut self, a: u32) -> Result<u16, BusFault> {
        Ok(self.read(a, 2)? as u16)
    }
    fn read32(&mut self, a: u32) -> Result<u32, BusFault> {
        self.read(a, 4)
    }
    fn write8(&mut self, a: u32, v: u8) -> Result<(), BusFault> {
        self.write(a, 1, u32::from(v))
    }
    fn write16(&mut self, a: u32, v: u16) -> Result<(), BusFault> {
        self.write(a, 2, u32::from(v))
    }
    fn write32(&mut self, a: u32, v: u32) -> Result<(), BusFault> {
        self.write(a, 4, v)
    }
}

/// A fresh facade is constructed per Cpu::step; its first read32 is the fetch.
struct FetchGuard<'a> {
    bus: &'a mut Router,
    fetch_first: bool,
    executable: std::ops::Range<u32>,
}
impl<'a> FetchGuard<'a> {
    fn for_step(bus: &'a mut Router) -> Self {
        Self {
            bus,
            fetch_first: true,
            executable: RAM_BASE..RAM_BASE + RAM_SIZE,
        }
    }
}
impl Bus for FetchGuard<'_> {
    fn read8(&mut self, a: u32) -> Result<u8, BusFault> {
        self.bus.read8(a)
    }
    fn read16(&mut self, a: u32) -> Result<u16, BusFault> {
        self.bus.read16(a)
    }
    fn read32(&mut self, a: u32) -> Result<u32, BusFault> {
        if self.fetch_first {
            self.fetch_first = false;
            if !a
                .checked_add(4)
                .is_some_and(|end| self.executable.start <= a && end <= self.executable.end)
            {
                return Err(BusFault);
            }
        }
        self.bus.read32(a)
    }
    fn write8(&mut self, a: u32, v: u8) -> Result<(), BusFault> {
        self.bus.write8(a, v)
    }
    fn write16(&mut self, a: u32, v: u16) -> Result<(), BusFault> {
        self.bus.write16(a, v)
    }
    fn write32(&mut self, a: u32, v: u32) -> Result<(), BusFault> {
        self.bus.write32(a, v)
    }
}
fn step_guarded(cpu: &mut Cpu, bus: &mut Router) -> Result<(), Fault> {
    cpu.step(&mut FetchGuard::for_step(bus))
}

#[test]
fn one_instance_serves_two_windows_and_same_type_instances_are_independent() {
    let mut bus = Router::new();
    bus.write32(GPU0_REGS, 0x11223344).unwrap();
    assert_eq!(bus.read32(GPU0_VRAM).unwrap(), 0x11223344);
    assert_eq!(bus.read32(GPU1_REGS).unwrap(), 0);
    bus.write32(GPU1_VRAM, 0x55667788).unwrap();
    assert_eq!(bus.read32(GPU1_REGS).unwrap(), 0x55667788);
    assert_eq!(bus.read32(GPU0_REGS).unwrap(), 0x11223344);
    assert_eq!(
        bus.events,
        [
            (0, Window::Gpu0Regs, 0, 4, true, 0x11223344),
            (0, Window::Gpu0Vram, 0, 4, false, 0x11223344),
            (1, Window::Gpu1Regs, 0, 4, false, 0),
            (1, Window::Gpu1Vram, 0, 4, true, 0x55667788),
            (1, Window::Gpu1Regs, 0, 4, false, 0x55667788),
            (0, Window::Gpu0Regs, 0, 4, false, 0x11223344)
        ]
        .map(|(instance, window, offset, width, write, value)| Event {
            instance,
            window,
            offset,
            width,
            write,
            value
        })
    );
}

#[test]
fn router_rejects_bad_width_alignment_and_full_extent_without_events() {
    let mut bus = Router::new();
    bus.instances[0].value = 0x11223344;
    bus.instances[1].value = 0x55667788;
    for (regs, vram) in [(GPU0_REGS, GPU0_VRAM), (GPU1_REGS, GPU1_VRAM)] {
        assert!(bus.read8(regs).is_err());
        assert!(bus.read16(regs + 1).is_err());
        assert!(bus.write8(regs, 7).is_err());
        assert!(bus.read16(vram + 1).is_err());
        assert!(bus.read32(vram + 4).is_err());
        assert!(bus.write32(vram + 4, 7).is_err());
    }
    assert!(bus.events.is_empty());
    assert_eq!(
        bus.instances.map(|probe| probe.value),
        [0x11223344, 0x55667788]
    );
}

fn instruction(op: u32, rd: u32, base: u32, offset: u32) -> u32 {
    (op << 28) | (rd << 24) | (base << 20) | offset
}
fn run_one(bus: &mut Router, word: u32, initial: u32) -> Result<Cpu, Fault> {
    bus.write32(RAM_BASE, word).unwrap();
    let mut cpu = Cpu::new();
    cpu.set_pc(RAM_BASE);
    cpu.set_reg(2, GPU0_VRAM);
    cpu.set_reg(3, initial);
    step_guarded(&mut cpu, bus).map(|_| cpu)
}
#[test]
fn cpu_executes_each_scalar_load_store_width_as_one_matching_access() {
    let mut bus = Router::new();
    // ldb, ldh, ldw use r2 as base; each reaches the same register address once.
    for (op, width) in [(4, 1), (5, 2), (7, 4)] {
        bus.instances[0].value = 0x89abcdef;
        bus.events.clear();
        let cpu = run_one(&mut bus, instruction(op, 1, 2, 0), 0).unwrap();
        assert_eq!(
            cpu.reg(1),
            match width {
                1 => 0xef,
                2 => 0xcdef,
                _ => 0x89abcdef,
            }
        );
        assert_eq!(
            bus.events,
            [Event {
                instance: 0,
                window: Window::Gpu0Vram,
                offset: 0,
                width,
                write: false,
                value: cpu.reg(1)
            }]
        );
    }
    // stb, sth, stw use r3 as value and r2 as base.
    for (op, width, value) in [(8, 1, 0xef), (9, 2, 0xcdef), (11, 4, 0x89abcdef)] {
        bus.events.clear();
        run_one(&mut bus, instruction(op, 3, 2, 0), value).unwrap();
        assert_eq!(
            bus.events,
            [Event {
                instance: 0,
                window: Window::Gpu0Vram,
                offset: 0,
                width,
                write: true,
                value
            }]
        );
        assert_eq!(bus.instances[0].value, value);
    }
}

#[test]
fn fetch_guard_rejects_mmio_and_unaligned_pc_without_callbacks_and_repeats_per_step() {
    let mut bus = Router::new();
    bus.instances[0].value = u32::MAX;
    bus.instances[0].clear_on_read = true;
    let mut cpu = Cpu::new();
    cpu.set_pc(GPU0_REGS);
    let before = cpu.clone();
    assert!(
        matches!(step_guarded(&mut cpu,&mut bus),Err(Fault::BusFault{faulting_pc,addr,width,operation:AccessOperation::Fetch}) if faulting_pc==GPU0_REGS&&addr==GPU0_REGS&&width==AccessWidth::Word)
    );
    assert_eq!(cpu, before);
    assert_eq!(bus.instances[0].value, u32::MAX);
    assert!(bus.events.is_empty());
    // Fresh facade on each call means a second MMIO fetch is rejected too.
    assert!(step_guarded(&mut cpu, &mut bus).is_err());
    assert!(bus.events.is_empty());
    bus.write32(RAM_BASE, 0).unwrap();
    cpu.set_pc(RAM_BASE);
    step_guarded(&mut cpu, &mut bus).unwrap();
    cpu.set_pc(GPU0_REGS);
    let before = cpu.clone();
    assert!(step_guarded(&mut cpu, &mut bus).is_err());
    assert_eq!(cpu, before);
    assert!(bus.events.is_empty());
    cpu.set_pc(RAM_BASE + 1);
    let before = cpu.clone();
    assert!(
        matches!(step_guarded(&mut cpu,&mut bus),Err(Fault::UnalignedPc{faulting_pc,addr}) if faulting_pc==RAM_BASE+1&&addr==RAM_BASE+1)
    );
    assert_eq!(cpu, before);
    assert!(bus.events.is_empty());
}

#[test]
fn fetch_guard_allows_read_clear_mmio_data_load_but_unguarded_invalid_fetch_commits() {
    let mut bus = Router::new();
    bus.instances[0].value = 0xfeedbeef;
    bus.instances[0].clear_on_read = true;
    bus.write32(RAM_BASE, instruction(7, 1, 2, 0)).unwrap();
    let mut cpu = Cpu::new();
    cpu.set_pc(RAM_BASE);
    cpu.set_reg(2, GPU0_VRAM);
    step_guarded(&mut cpu, &mut bus).unwrap();
    assert_eq!(cpu.reg(1), 0xfeedbeef);
    assert_eq!(bus.instances[0].value, 0);
    assert_eq!(bus.events.len(), 1);
    bus.events.clear();
    bus.instances[0].value = u32::MAX;
    bus.instances[0].clear_on_read = true;
    let mut unguarded = Cpu::new();
    unguarded.set_pc(GPU0_REGS);
    let before_unguarded = unguarded.clone();
    assert!(
        matches!(unguarded.step(&mut bus),Err(Fault::InvalidInstruction{faulting_pc}) if faulting_pc==GPU0_REGS)
    );
    assert_eq!(unguarded, before_unguarded);
    assert_eq!(bus.instances[0].value, 0);
    assert_eq!(
        bus.events,
        [Event {
            instance: 0,
            window: Window::Gpu0Regs,
            offset: 0,
            width: 4,
            write: false,
            value: u32::MAX
        }]
    );
}

struct Setup {
    instances: Vec<Probe>,
    drops: Rc<RefCell<Vec<u32>>>,
}
impl Setup {
    fn new() -> Self {
        Self {
            instances: Vec::new(),
            drops: Rc::new(RefCell::new(Vec::new())),
        }
    }
    fn create(&mut self, id: u32, fail: bool) -> Result<(), ()> {
        if fail {
            return Err(());
        }
        self.instances.push(Probe::new(id, self.drops.clone()));
        Ok(())
    }
}
impl Drop for Setup {
    fn drop(&mut self) {
        while let Some(instance) = self.instances.pop() {
            drop(instance);
        }
    }
}
#[test]
fn setup_success_teardown_and_second_creation_failure_drop_in_reverse_order() {
    let mut ok = Setup::new();
    let log = ok.drops.clone();
    ok.create(1, false).unwrap();
    ok.create(2, false).unwrap();
    drop(ok);
    assert_eq!(*log.borrow(), [2, 1]);
    let mut failed = Setup::new();
    let log = failed.drops.clone();
    failed.create(1, false).unwrap();
    assert!(failed.create(2, true).is_err());
    drop(failed);
    assert_eq!(*log.borrow(), [1]);
}
