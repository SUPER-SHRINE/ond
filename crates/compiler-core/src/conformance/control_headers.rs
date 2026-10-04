//! Empty control bodies and composite literals remain distinct.
use super::*;
pub(super) const CASES: &[Case] = &[
    Case {
        id: "FLOW-02.empty_if_identifier",
        specs: &["FLOW-02"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test(flag: bool) { if flag {} }\n",
        }],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "FLOW-02.empty_for_identifier",
        specs: &["FLOW-02"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test(flag: bool) { for flag {} }\n",
        }],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "FLOW-02.header_if_init_identifier",
        specs: &["FLOW-02", "SYN-12"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\ntype S struct { Flag: bool; }\ntype Outer struct { Inner: S; }\ntype Flags [1]bool\nfunc yes(s: S) -> bool { return s.Flag; }\nfunc slot(s: S) -> u32 { return 0; }\nfunc main() {}\nfunc test() { if flag := true; flag {}; }\n",
        }],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "FLOW-02.header_if_identifier_else",
        specs: &["FLOW-02", "SYN-12"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\ntype S struct { Flag: bool; }\ntype Outer struct { Inner: S; }\ntype Flags [1]bool\nfunc yes(s: S) -> bool { return s.Flag; }\nfunc slot(s: S) -> u32 { return 0; }\nfunc main() {}\nfunc test() { var flag = false; if flag {} else {}; }\n",
        }],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "FLOW-02.header_multiline_if",
        specs: &["FLOW-02", "SYN-12"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\ntype S struct { Flag: bool; }\ntype Outer struct { Inner: S; }\ntype Flags [1]bool\nfunc yes(s: S) -> bool { return s.Flag; }\nfunc slot(s: S) -> u32 { return 0; }\nfunc main() {}\nfunc test() { var flag = true; if flag {\n// empty body\n}; }\n",
        }],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "FLOW-02.header_multiline_for",
        specs: &["FLOW-02", "SYN-12"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\ntype S struct { Flag: bool; }\ntype Outer struct { Inner: S; }\ntype Flags [1]bool\nfunc yes(s: S) -> bool { return s.Flag; }\nfunc slot(s: S) -> u32 { return 0; }\nfunc main() {}\nfunc test() { var flag = false; for flag {\n// empty body\n}; }\n",
        }],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "FLOW-02.header_for_condition_clause",
        specs: &["FLOW-02", "SYN-12"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\ntype S struct { Flag: bool; }\ntype Outer struct { Inner: S; }\ntype Flags [1]bool\nfunc yes(s: S) -> bool { return s.Flag; }\nfunc slot(s: S) -> u32 { return 0; }\nfunc main() {}\nfunc test() { var flag = false; for ; flag; {}; }\n",
        }],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "FLOW-02.header_for_post_identifier",
        specs: &["FLOW-02", "SYN-12"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\ntype S struct { Flag: bool; }\ntype Outer struct { Inner: S; }\ntype Flags [1]bool\nfunc yes(s: S) -> bool { return s.Flag; }\nfunc slot(s: S) -> u32 { return 0; }\nfunc main() {}\nfunc test() { var next = 1; for i := 0; i < 1; i = next {}; }\n",
        }],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "FLOW-02.header_for_post_expression",
        specs: &["FLOW-02", "SYN-12"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\ntype S struct { Flag: bool; }\ntype Outer struct { Inner: S; }\ntype Flags [1]bool\nfunc yes(s: S) -> bool { return s.Flag; }\nfunc slot(s: S) -> u32 { return 0; }\nfunc main() {}\nfunc test() { var next = 1; for i := 0; i < 1; i = i + next {}; }\n",
        }],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "FLOW-02.header_if_init_composite",
        specs: &["FLOW-02", "SYN-12"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\ntype S struct { Flag: bool; }\ntype Outer struct { Inner: S; }\ntype Flags [1]bool\nfunc yes(s: S) -> bool { return s.Flag; }\nfunc slot(s: S) -> u32 { return 0; }\nfunc main() {}\nfunc test() { if s := S{}; s.Flag {}; }\n",
        }],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "FLOW-02.header_if_assignment_composite",
        specs: &["FLOW-02", "SYN-12"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\ntype S struct { Flag: bool; }\ntype Outer struct { Inner: S; }\ntype Flags [1]bool\nfunc yes(s: S) -> bool { return s.Flag; }\nfunc slot(s: S) -> u32 { return 0; }\nfunc main() {}\nfunc test() { var s: S; if s = S{}; s.Flag {}; }\n",
        }],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "FLOW-02.header_if_parenthesized_composite",
        specs: &["FLOW-02", "SYN-12"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\ntype S struct { Flag: bool; }\ntype Outer struct { Inner: S; }\ntype Flags [1]bool\nfunc yes(s: S) -> bool { return s.Flag; }\nfunc slot(s: S) -> u32 { return 0; }\nfunc main() {}\nfunc test() { if (S{}).Flag {}; }\n",
        }],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "FLOW-02.header_if_call_composite",
        specs: &["FLOW-02", "SYN-12"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\ntype S struct { Flag: bool; }\ntype Outer struct { Inner: S; }\ntype Flags [1]bool\nfunc yes(s: S) -> bool { return s.Flag; }\nfunc slot(s: S) -> u32 { return 0; }\nfunc main() {}\nfunc test() { if yes(S{}) {}; }\n",
        }],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "FLOW-02.header_if_index_composite",
        specs: &["FLOW-02", "SYN-12"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\ntype S struct { Flag: bool; }\ntype Outer struct { Inner: S; }\ntype Flags [1]bool\nfunc yes(s: S) -> bool { return s.Flag; }\nfunc slot(s: S) -> u32 { return 0; }\nfunc main() {}\nfunc test() { var flags: [1]bool; if flags[slot(S{})] {}; }\n",
        }],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "FLOW-02.header_if_array_elements",
        specs: &["FLOW-02", "SYN-12"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\ntype S struct { Flag: bool; }\ntype Outer struct { Inner: S; }\ntype Flags [1]bool\nfunc yes(s: S) -> bool { return s.Flag; }\nfunc slot(s: S) -> u32 { return 0; }\nfunc main() {}\nfunc test() { if [1]S{S{}}[0].Flag {}; }\n",
        }],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "FLOW-02.header_if_keyed_composite",
        specs: &["FLOW-02", "SYN-12"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\ntype S struct { Flag: bool; }\ntype Outer struct { Inner: S; }\ntype Flags [1]bool\nfunc yes(s: S) -> bool { return s.Flag; }\nfunc slot(s: S) -> u32 { return 0; }\nfunc main() {}\nfunc test() { if S{Flag: true}.Flag {}; }\n",
        }],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "FLOW-02.header_if_nested_composite",
        specs: &["FLOW-02", "SYN-12"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\ntype S struct { Flag: bool; }\ntype Outer struct { Inner: S; }\ntype Flags [1]bool\nfunc yes(s: S) -> bool { return s.Flag; }\nfunc slot(s: S) -> u32 { return 0; }\nfunc main() {}\nfunc test() { if Outer{Inner: S{}}.Inner.Flag {}; }\n",
        }],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "FLOW-02.header_if_composite_selector",
        specs: &["FLOW-02", "SYN-12"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\ntype S struct { Flag: bool; }\ntype Outer struct { Inner: S; }\ntype Flags [1]bool\nfunc yes(s: S) -> bool { return s.Flag; }\nfunc slot(s: S) -> u32 { return 0; }\nfunc main() {}\nfunc test() { if S{}.Flag {}; }\n",
        }],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "FLOW-02.header_if_named_array_index",
        specs: &["FLOW-02", "SYN-12"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\ntype S struct { Flag: bool; }\ntype Outer struct { Inner: S; }\ntype Flags [1]bool\nfunc yes(s: S) -> bool { return s.Flag; }\nfunc slot(s: S) -> u32 { return 0; }\nfunc main() {}\nfunc test() { if Flags{}[0] {}; }\n",
        }],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "FLOW-02.header_for_init_composite",
        specs: &["FLOW-02", "SYN-12"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\ntype S struct { Flag: bool; }\ntype Outer struct { Inner: S; }\ntype Flags [1]bool\nfunc yes(s: S) -> bool { return s.Flag; }\nfunc slot(s: S) -> u32 { return 0; }\nfunc main() {}\nfunc test() { for s := S{}; s.Flag; {}; }\n",
        }],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "FLOW-02.header_for_condition_composite",
        specs: &["FLOW-02", "SYN-12"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\ntype S struct { Flag: bool; }\ntype Outer struct { Inner: S; }\ntype Flags [1]bool\nfunc yes(s: S) -> bool { return s.Flag; }\nfunc slot(s: S) -> u32 { return 0; }\nfunc main() {}\nfunc test() { for S{}.Flag {}; }\n",
        }],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "FLOW-02.header_for_post_composite",
        specs: &["FLOW-02", "SYN-12"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\ntype S struct { Flag: bool; }\ntype Outer struct { Inner: S; }\ntype Flags [1]bool\nfunc yes(s: S) -> bool { return s.Flag; }\nfunc slot(s: S) -> u32 { return 0; }\nfunc main() {}\nfunc test() { var s: S; for i := 0; i < 1; s = S{} { i = i + 1; }; }\n",
        }],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "FLOW-02.header_nested_header_restoration",
        specs: &["FLOW-02", "SYN-12"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\ntype S struct { Flag: bool; }\ntype Outer struct { Inner: S; }\ntype Flags [1]bool\nfunc yes(s: S) -> bool { return s.Flag; }\nfunc slot(s: S) -> u32 { return 0; }\nfunc main() {}\nfunc test() { var flag = false; if flag {} else { if S{}.Flag {}; var s = S{}; _ = s; }; var s = S{}; _ = s; }\n",
        }],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "FLOW-02.header_body_composite",
        specs: &["FLOW-02", "SYN-12"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\ntype S struct { Flag: bool; }\ntype Outer struct { Inner: S; }\ntype Flags [1]bool\nfunc yes(s: S) -> bool { return s.Flag; }\nfunc slot(s: S) -> u32 { return 0; }\nfunc main() {}\nfunc test() { var flag = true; if flag { var s = S{}; _ = s; }; }\n",
        }],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "FLOW-02.header_for_post_parenthesized",
        specs: &["FLOW-02", "SYN-12"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\ntype S struct { Flag: bool; }\ntype Outer struct { Inner: S; }\ntype Flags [1]bool\nfunc yes(s: S) -> bool { return s.Flag; }\nfunc slot(s: S) -> u32 { return 0; }\nfunc main() {}\nfunc test() { var s: S; for i := 0; i < 1; s = (S{}) { i = i + 1; }; }\n",
        }],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "FLOW-02.empty_bodies_execution",
        specs: &["FLOW-02"],
        layer: Layer::MemoryVm,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test(flag: bool) -> (i32, bool) { var n = 0; if flag {} else { n = 1; }; flag = false; for flag {}; for ; flag == false; flag = true {}; return n, flag; }\n",
        }],
        expected: Expected::Accept(&[
            Check::MemoryExecution {
                description: "false branch and empty loop post",
                function: "main.test",
                input: &[0],
                expected: &[1, 1],
            },
            Check::MemoryExecution {
                description: "empty true branch and empty loop post",
                function: "main.test",
                input: &[1],
                expected: &[0, 1],
            },
        ]),
    },
];

#[test]
fn empty_bodies_preserve_branch_and_post_execution() {
    run(&CASES[25]).unwrap();
}
#[test]
fn empty_if_identifier() {
    run(&CASES[0]).unwrap();
}
#[test]
fn empty_for_identifier() {
    run(&CASES[1]).unwrap();
}
#[test]
fn if_init_identifier() {
    run(&CASES[2]).unwrap();
}
#[test]
fn if_identifier_else() {
    run(&CASES[3]).unwrap();
}
#[test]
fn multiline_if() {
    run(&CASES[4]).unwrap();
}
#[test]
fn multiline_for() {
    run(&CASES[5]).unwrap();
}
#[test]
fn for_condition_clause() {
    run(&CASES[6]).unwrap();
}
#[test]
fn for_post_identifier() {
    run(&CASES[7]).unwrap();
}
#[test]
fn for_post_expression() {
    run(&CASES[8]).unwrap();
}
#[test]
fn if_init_composite() {
    run(&CASES[9]).unwrap();
}
#[test]
fn if_assignment_composite() {
    run(&CASES[10]).unwrap();
}
#[test]
fn if_parenthesized_composite() {
    run(&CASES[11]).unwrap();
}
#[test]
fn if_call_composite() {
    run(&CASES[12]).unwrap();
}
#[test]
fn if_index_composite() {
    run(&CASES[13]).unwrap();
}
#[test]
fn if_array_elements() {
    run(&CASES[14]).unwrap();
}
#[test]
fn if_keyed_composite() {
    run(&CASES[15]).unwrap();
}
#[test]
fn if_nested_composite() {
    run(&CASES[16]).unwrap();
}
#[test]
fn if_composite_selector() {
    run(&CASES[17]).unwrap();
}
#[test]
fn if_named_array_index() {
    run(&CASES[18]).unwrap();
}
#[test]
fn for_init_composite() {
    run(&CASES[19]).unwrap();
}
#[test]
fn for_condition_composite() {
    run(&CASES[20]).unwrap();
}
#[test]
fn for_post_composite() {
    run(&CASES[21]).unwrap();
}
#[test]
fn nested_header_restoration() {
    run(&CASES[22]).unwrap();
}
#[test]
fn body_composite() {
    run(&CASES[23]).unwrap();
}
#[test]
fn for_post_parenthesized() {
    run(&CASES[24]).unwrap();
}
