//! Fixed-footprint branch islands: no link-time insertion or layout iteration.
pub fn word(op: u32, d: u8, a: u8, b: u8, imm: i16) -> u32 {
    (op << 28) | ((d as u32) << 24) | ((a as u32) << 20) | ((b as u32) << 16) | imm as u16 as u32
}

pub(crate) const LINK_SLOT_WORDS: usize = 6;
pub(crate) const LOCAL_SLOT_WORDS: usize = 7;

pub(crate) fn link_slot(call: bool, condition: u8) -> [u32; LINK_SLOT_WORDS] {
    [
        word(12, if call { 15 } else { 0 }, condition, 0, 0),
        word(12, 0, 0, 0, 4),
        0,
        0,
        0,
        0,
    ]
}

pub(crate) fn local_slot(condition: u8) -> [u32; LOCAL_SLOT_WORDS] {
    [
        word(12, 0, condition, 0, 0),
        word(12, 0, 0, 0, 5),
        0,
        0,
        0,
        0,
        0,
    ]
}

/// Captures the address of the following word, then loads the nearby literal.
pub fn literal_load(reg: u8, offset: i16) -> [u32; 2] {
    [word(12, reg, 0, 0, 0), word(7, reg, reg, 0, offset)]
}

/// Tail jump: r15 is the original caller's return address, not a new link.
pub fn absolute_thunk(target: u32) -> [u32; 4] {
    let [capture, load] = literal_load(12, 8);
    [capture, load, word(12, 0, 0, 12, 0), target]
}

/// Position independent; delta is relative to the second instruction here.
pub(crate) fn relative_thunk(delta: u32) -> [u32; 5] {
    [
        word(12, 12, 0, 0, 0),
        word(7, 13, 12, 0, 12),
        word(0, 12, 12, 13, 0),
        word(12, 0, 0, 12, 0),
        delta,
    ]
}
