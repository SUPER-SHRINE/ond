#![forbid(unsafe_code)]

use core::any::Any;

use crate::{ir::object::SectionKind, linker::LinkedImage};
use debug_io::DebugIo;
use kagura::{Bus, BusFault, Cpu, DefaultBus, Device, Fault};
use ram::Ram;

pub const RAM_BASE: u32 = 0x0000_1000;
pub const EXIT_CODE_BASE: u32 = 0xFFFF_8040;
pub const EXIT_CODE_SIZE: u32 = 0x0000_0004;
pub const DEBUG_IO_BASE: u32 = 0xFFFF_8050;
pub const DEBUG_IO_SIZE: u32 = 0x0000_0010;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineStatus {
    Running,
    Exited(u32),
}

pub struct LinkedMachine {
    cpu: Cpu,
    bus: DefaultBus,
    retired_instructions: u64,
}

impl LinkedMachine {
    pub fn from_image(executable: &LinkedImage) -> Result<Self, String> {
        let entry_point = executable.entry_point;
        let ram_size = executable.ram_size;
        let mut bus = DefaultBus::new();
        bus.map_device(RAM_BASE, ram_size, Ram::new(ram_size as usize))
            .map_err(|_| "failed to map RAM".to_string())?;
        bus.map_device(DEBUG_IO_BASE, DEBUG_IO_SIZE, DebugIo::new())
            .map_err(|_| "failed to map DebugIo".to_string())?;
        bus.map_device(EXIT_CODE_BASE, EXIT_CODE_SIZE, ExitDevice::new())
            .map_err(|_| "failed to map EXIT_CODE".to_string())?;

        for segment in &executable.sections {
            match segment.kind {
                SectionKind::Text | SectionKind::Rodata | SectionKind::Data => {
                    for (offset, byte) in segment.data.iter().copied().enumerate() {
                        bus.write8(segment.load_address + offset as u32, byte)
                            .map_err(|_| "failed to load segment bytes".to_string())?;
                    }
                    if segment.kind == SectionKind::Data
                        && segment.memory_size as usize > segment.data.len()
                    {
                        for offset in segment.data.len()..segment.memory_size as usize {
                            bus.write8(segment.load_address + offset as u32, 0)
                                .map_err(|_| "failed to zero data tail".to_string())?;
                        }
                    }
                }
                SectionKind::Bss => {
                    for offset in 0..segment.memory_size {
                        bus.write8(segment.load_address + offset, 0)
                            .map_err(|_| "failed to zero bss".to_string())?;
                    }
                }
            }
        }

        let mut cpu = Cpu::new();
        cpu.set_reg(14, RAM_BASE + ram_size);
        cpu.set_pc(entry_point);

        Ok(Self {
            cpu,
            bus,
            retired_instructions: 0,
        })
    }

    pub fn pc(&self) -> u32 {
        self.cpu.pc()
    }

    pub fn reg(&self, index: usize) -> u32 {
        self.cpu.reg(index)
    }

    /// Returns the number of CPU instructions successfully retired by this machine.
    pub fn retired_instructions(&self) -> u64 {
        self.retired_instructions
    }

    pub fn step(&mut self) -> Result<MachineStatus, Fault> {
        self.cpu.step(&mut self.bus)?;
        self.retired_instructions += 1;
        Ok(self.status())
    }

    pub fn run_steps(&mut self, max_steps: usize) -> Result<MachineStatus, Fault> {
        for _ in 0..max_steps {
            let status = self.step()?;
            if status != MachineStatus::Running {
                return Ok(status);
            }
        }
        Ok(MachineStatus::Running)
    }

    pub fn status(&self) -> MachineStatus {
        let device = self
            .bus
            .device_ref::<ExitDevice>(EXIT_CODE_BASE)
            .expect("EXIT_CODE device must be present");
        match device.code {
            Some(code) => MachineStatus::Exited(code),
            None => MachineStatus::Running,
        }
    }

    pub fn debug_output(&self) -> &[u8] {
        self.bus
            .device_ref::<DebugIo>(DEBUG_IO_BASE)
            .expect("DebugIo must be present")
            .output()
    }

    pub fn take_debug_output(&mut self) -> Vec<u8> {
        self.bus
            .device_mut::<DebugIo>(DEBUG_IO_BASE)
            .expect("DebugIo must be present")
            .take_output()
    }
}

#[derive(Default)]
struct ExitDevice {
    code: Option<u32>,
}

impl ExitDevice {
    fn new() -> Self {
        Self::default()
    }
}

impl Device for ExitDevice {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn read8(&mut self, _: u32) -> Result<u8, BusFault> {
        Err(BusFault)
    }

    fn read16(&mut self, _: u32) -> Result<u16, BusFault> {
        Err(BusFault)
    }

    fn read32(&mut self, _: u32) -> Result<u32, BusFault> {
        Err(BusFault)
    }

    fn write8(&mut self, _: u32, _: u8) -> Result<(), BusFault> {
        Err(BusFault)
    }

    fn write16(&mut self, _: u32, _: u16) -> Result<(), BusFault> {
        Err(BusFault)
    }

    fn write32(&mut self, _: u32, value: u32) -> Result<(), BusFault> {
        self.code = Some(value);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::compile_image as compile;
    // These are correctness budgets, not instruction-count performance goldens.
    const STEP_LIMIT: usize = 2_000_000;
    include!("machine_suite.rs");
}
