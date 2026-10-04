//! Target-neutral, opt-in test fixtures. Not a stable production API.
#[path = "test_vectors/evaluation.rs"]
pub mod evaluation;
#[path = "test_vectors/floats.rs"]
pub mod floats;
#[path = "test_vectors/integers.rs"]
pub mod integers;

#[derive(Clone, Copy)]
pub struct Integer {
    pub name: &'static str,
    pub bits: u32,
    pub min: i128,
    pub max: i128,
    pub counts: [u32; 5],
}
// Explicit specification boundaries, never queried from compiler helpers.
pub const INTEGERS: &[Integer] = &[
    Integer {
        name: "u8",
        bits: 8,
        min: 0,
        max: 255,
        counts: [0, 7, 8, 9, 4294967295],
    },
    Integer {
        name: "i8",
        bits: 8,
        min: -128,
        max: 127,
        counts: [0, 7, 8, 9, 4294967295],
    },
    Integer {
        name: "u16",
        bits: 16,
        min: 0,
        max: 65535,
        counts: [0, 15, 16, 17, 4294967295],
    },
    Integer {
        name: "i16",
        bits: 16,
        min: -32768,
        max: 32767,
        counts: [0, 15, 16, 17, 4294967295],
    },
    Integer {
        name: "u32",
        bits: 32,
        min: 0,
        max: 4294967295,
        counts: [0, 31, 32, 33, 4294967295],
    },
    Integer {
        name: "i32",
        bits: 32,
        min: -2147483648,
        max: 2147483647,
        counts: [0, 31, 32, 33, 4294967295],
    },
];
