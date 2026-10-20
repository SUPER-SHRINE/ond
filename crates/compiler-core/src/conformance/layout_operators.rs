//! Concrete layout and non-conflicting custom operator cases.
use super::*;

pub(super) const CASES: &[Case] = &[
    Case {
        id: "BI-04.layout-padded-struct",
        specs: &["BI-04"],
        layer: Layer::MemoryVm,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: r#"package main
type S struct { a: u8; b: u32; c: u8 }
func observe() -> (u32, u32) { return sizeof(S), alignof(S) }
func main() {}
"#,
        }],
        expected: Expected::Accept(&[Check::MemoryExecution {
            description: "layout or custom operator observable result",
            function: "main.observe",
            input: &[],
            expected: &[12, 4],
        }]),
    },
    Case {
        id: "BI-04.layout-zero-array",
        specs: &["BI-04"],
        layer: Layer::MemoryVm,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: r#"package main
func observe() -> (u32, u32) { return sizeof([0]u32), alignof([0]u32) }
func main() {}
"#,
        }],
        expected: Expected::Accept(&[Check::MemoryExecution {
            description: "layout or custom operator observable result",
            function: "main.observe",
            input: &[],
            expected: &[0, 4],
        }]),
    },
    Case {
        id: "BI-02.operator-len-struct",
        specs: &["BI-02"],
        layer: Layer::MemoryVm,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: r#"package main
type Buffer struct { count: u32 }
operator (self: Buffer) len() -> u32 { return self.count }
func observe() -> u32 { value := Buffer{count: 7}; return len(value) }
func main() {}
"#,
        }],
        expected: Expected::Accept(&[Check::MemoryExecution {
            description: "layout or custom operator observable result",
            function: "main.observe",
            input: &[],
            expected: &[7],
        }]),
    },
    Case {
        id: "OP-01.operator-index-struct",
        specs: &["OP-01"],
        layer: Layer::MemoryVm,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: r#"package main
type Table struct { values: [2]u32 }
operator (self: Table) [](index: u32) -> u32 { return self.values[index] }
func observe() -> u32 { value := Table{values: [2]u32{11, 42}}; return value[1 as u32] }
func main() {}
"#,
        }],
        expected: Expected::Accept(&[Check::MemoryExecution {
            description: "layout or custom operator observable result",
            function: "main.observe",
            input: &[],
            expected: &[42],
        }]),
    },
];

#[test]
fn layout_and_custom_operators() {
    for case in CASES {
        run(case).unwrap();
    }
}
