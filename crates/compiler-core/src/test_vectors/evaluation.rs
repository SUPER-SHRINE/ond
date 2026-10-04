//! Shared source-level trace and final-value fixtures; entry is main.test().
pub struct File {
    pub path: &'static str,
    pub text: &'static str,
}
pub struct Evaluation {
    pub id: &'static str,
    pub specs: &'static [&'static str],
    pub files: &'static [File],
    pub expected: &'static [u64],
}
pub const CASES: &[Evaluation] = &[
    Evaluation {
        id: "EVAL-02.operands",
        specs: &["EVAL-02"],
        files: &[File {
            path: "main.ond",
            text: "package main\nvar Trace: i32\nvar A: [2]i32\nfunc main() {}\nfunc value(tag: i32, n: i32) -> i32 { Trace = Trace * 10 + tag; return n; }\nfunc index(tag: i32, n: u32) -> u32 { Trace = Trace * 10 + tag; return n; }\nfunc ptr(tag: i32, n: u32) -> *i32 { Trace = Trace * 10 + tag; return &A[n]; }\nfunc flag(tag: i32, b: bool) -> bool { Trace = Trace * 10 + tag; return b; }\nfunc test() -> (i32, i32) { var n = value(1, 2) + value(2, 3) * value(3, 4); return Trace, n; }\n",
        }],
        expected: &[123, 14],
    },
    Evaluation {
        id: "EVAL-02.arguments",
        specs: &["EVAL-02"],
        files: &[File {
            path: "main.ond",
            text: "package main\nvar Trace: i32\nvar A: [2]i32\nfunc main() {}\nfunc value(tag: i32, n: i32) -> i32 { Trace = Trace * 10 + tag; return n; }\nfunc index(tag: i32, n: u32) -> u32 { Trace = Trace * 10 + tag; return n; }\nfunc ptr(tag: i32, n: u32) -> *i32 { Trace = Trace * 10 + tag; return &A[n]; }\nfunc flag(tag: i32, b: bool) -> bool { Trace = Trace * 10 + tag; return b; }\nfunc combine(a: i32, b: i32) -> i32 { return value(3, a * 10 + b); }\nfunc test() -> (i32, i32) { var n = combine(value(1, 2), value(2, 4)); return Trace, n; }\n",
        }],
        expected: &[123, 24],
    },
    Evaluation {
        id: "EVAL-02.indices_distinct",
        specs: &["EVAL-02"],
        files: &[File {
            path: "main.ond",
            text: "package main\nvar Trace: i32\nvar A: [2]i32\nfunc main() {}\nfunc value(tag: i32, n: i32) -> i32 { Trace = Trace * 10 + tag; return n; }\nfunc index(tag: i32, n: u32) -> u32 { Trace = Trace * 10 + tag; return n; }\nfunc ptr(tag: i32, n: u32) -> *i32 { Trace = Trace * 10 + tag; return &A[n]; }\nfunc flag(tag: i32, b: bool) -> bool { Trace = Trace * 10 + tag; return b; }\nfunc test() -> (i32, i32, i32) { A[index(1, 0)], A[index(2, 1)] = value(3, 10), value(4, 20); return Trace, A[0], A[1]; }\n",
        }],
        expected: &[1234, 10, 20],
    },
    Evaluation {
        id: "EVAL-02.indices_alias",
        specs: &["EVAL-02"],
        files: &[File {
            path: "main.ond",
            text: "package main\nvar Trace: i32\nvar A: [2]i32\nfunc main() {}\nfunc value(tag: i32, n: i32) -> i32 { Trace = Trace * 10 + tag; return n; }\nfunc index(tag: i32, n: u32) -> u32 { Trace = Trace * 10 + tag; return n; }\nfunc ptr(tag: i32, n: u32) -> *i32 { Trace = Trace * 10 + tag; return &A[n]; }\nfunc flag(tag: i32, b: bool) -> bool { Trace = Trace * 10 + tag; return b; }\nfunc test() -> (i32, i32, i32) { A[index(1, 0)], A[index(2, 0)] = value(3, 10), value(4, 20); return Trace, A[0], A[1]; }\n",
        }],
        expected: &[1234, 20, 0],
    },
    Evaluation {
        id: "EVAL-02.pointers_alias",
        specs: &["EVAL-02"],
        files: &[File {
            path: "main.ond",
            text: "package main\nvar Trace: i32\nvar A: [2]i32\nfunc main() {}\nfunc value(tag: i32, n: i32) -> i32 { Trace = Trace * 10 + tag; return n; }\nfunc index(tag: i32, n: u32) -> u32 { Trace = Trace * 10 + tag; return n; }\nfunc ptr(tag: i32, n: u32) -> *i32 { Trace = Trace * 10 + tag; return &A[n]; }\nfunc flag(tag: i32, b: bool) -> bool { Trace = Trace * 10 + tag; return b; }\nfunc test() -> (i32, i32, i32) { *ptr(1, 0), *ptr(2, 0) = value(3, 10), value(4, 20); return Trace, A[0], A[1]; }\n",
        }],
        expected: &[1234, 20, 0],
    },
    Evaluation {
        id: "EVAL-02.index_snapshot",
        specs: &["EVAL-02"],
        files: &[File {
            path: "main.ond",
            text: "package main\nvar Trace: i32\nvar A: [2]i32\nfunc main() {}\nfunc value(tag: i32, n: i32) -> i32 { Trace = Trace * 10 + tag; return n; }\nfunc index(tag: i32, n: u32) -> u32 { Trace = Trace * 10 + tag; return n; }\nfunc ptr(tag: i32, n: u32) -> *i32 { Trace = Trace * 10 + tag; return &A[n]; }\nfunc flag(tag: i32, b: bool) -> bool { Trace = Trace * 10 + tag; return b; }\nfunc test() -> (i32, u32, i32, i32) { var i: u32 = 0; i, A[i] = index(1, 1), value(2, 7); return Trace, i, A[0], A[1]; }\n",
        }],
        expected: &[12, 1, 7, 0],
    },
    Evaluation {
        id: "EVAL-02.pointer_snapshot",
        specs: &["EVAL-02"],
        files: &[File {
            path: "main.ond",
            text: "package main\nvar Trace: i32\nvar A: [2]i32\nfunc main() {}\nfunc value(tag: i32, n: i32) -> i32 { Trace = Trace * 10 + tag; return n; }\nfunc index(tag: i32, n: u32) -> u32 { Trace = Trace * 10 + tag; return n; }\nfunc ptr(tag: i32, n: u32) -> *i32 { Trace = Trace * 10 + tag; return &A[n]; }\nfunc flag(tag: i32, b: bool) -> bool { Trace = Trace * 10 + tag; return b; }\nfunc test() -> (i32, i32, i32, i32) { var p = &A[0]; p, *p = ptr(1, 1), value(2, 9); return Trace, A[0], A[1], *p; }\n",
        }],
        expected: &[12, 9, 0, 0],
    },
    Evaluation {
        id: "EVAL-02.rhs_snapshot",
        specs: &["EVAL-02"],
        files: &[File {
            path: "main.ond",
            text: "package main\nvar Trace: i32\nvar A: [2]i32\nfunc main() {}\nfunc value(tag: i32, n: i32) -> i32 { Trace = Trace * 10 + tag; return n; }\nfunc index(tag: i32, n: u32) -> u32 { Trace = Trace * 10 + tag; return n; }\nfunc ptr(tag: i32, n: u32) -> *i32 { Trace = Trace * 10 + tag; return &A[n]; }\nfunc flag(tag: i32, b: bool) -> bool { Trace = Trace * 10 + tag; return b; }\nfunc test() -> (i32, i32, i32) { A[0], A[1] = 10, 20; A[0], A[1] = value(1, A[1]), value(2, A[0]); return Trace, A[0], A[1]; }\n",
        }],
        expected: &[12, 20, 10],
    },
    Evaluation {
        id: "EVAL-02.rhs_mutates_index",
        specs: &["EVAL-02"],
        files: &[File {
            path: "main.ond",
            text: "package main\nvar Trace: i32\nvar A: [2]i32\nfunc main() {}\nfunc value(tag: i32, n: i32) -> i32 { Trace = Trace * 10 + tag; return n; }\nfunc index(tag: i32, n: u32) -> u32 { Trace = Trace * 10 + tag; return n; }\nfunc ptr(tag: i32, n: u32) -> *i32 { Trace = Trace * 10 + tag; return &A[n]; }\nfunc flag(tag: i32, b: bool) -> bool { Trace = Trace * 10 + tag; return b; }\nvar Position: u32\nfunc mutate() -> i32 { Position = 1; A[0] = 99; return value(2, 7); }\nfunc test() -> (i32, u32, i32, i32) { A[index(1, Position)] = mutate(); return Trace, Position, A[0], A[1]; }\n",
        }],
        expected: &[12, 1, 7, 0],
    },
    Evaluation {
        id: "EVAL-02.aggregate_then_field",
        specs: &["EVAL-02"],
        files: &[File {
            path: "main.ond",
            text: "package main\nvar Trace: i32\nvar A: [2]i32\nfunc main() {}\nfunc value(tag: i32, n: i32) -> i32 { Trace = Trace * 10 + tag; return n; }\nfunc index(tag: i32, n: u32) -> u32 { Trace = Trace * 10 + tag; return n; }\nfunc ptr(tag: i32, n: u32) -> *i32 { Trace = Trace * 10 + tag; return &A[n]; }\nfunc flag(tag: i32, b: bool) -> bool { Trace = Trace * 10 + tag; return b; }\nfunc build() -> [2]i32 { _ = value(1, 0); return [2]i32{3, 4}; }\nfunc test() -> (i32, i32, i32) { A, A[0] = build(), value(2, 9); return Trace, A[0], A[1]; }\n",
        }],
        expected: &[12, 9, 4],
    },
    Evaluation {
        id: "EVAL-02.field_then_aggregate",
        specs: &["EVAL-02"],
        files: &[File {
            path: "main.ond",
            text: "package main\nvar Trace: i32\nvar A: [2]i32\nfunc main() {}\nfunc value(tag: i32, n: i32) -> i32 { Trace = Trace * 10 + tag; return n; }\nfunc index(tag: i32, n: u32) -> u32 { Trace = Trace * 10 + tag; return n; }\nfunc ptr(tag: i32, n: u32) -> *i32 { Trace = Trace * 10 + tag; return &A[n]; }\nfunc flag(tag: i32, b: bool) -> bool { Trace = Trace * 10 + tag; return b; }\nfunc build() -> [2]i32 { _ = value(2, 0); return [2]i32{3, 4}; }\nfunc test() -> (i32, i32, i32) { A[0], A = value(1, 9), build(); return Trace, A[0], A[1]; }\n",
        }],
        expected: &[12, 3, 4],
    },
    Evaluation {
        id: "EVAL-02.multi_return_alias",
        specs: &["EVAL-02"],
        files: &[File {
            path: "main.ond",
            text: "package main\nvar Trace: i32\nvar A: [2]i32\nfunc main() {}\nfunc value(tag: i32, n: i32) -> i32 { Trace = Trace * 10 + tag; return n; }\nfunc index(tag: i32, n: u32) -> u32 { Trace = Trace * 10 + tag; return n; }\nfunc ptr(tag: i32, n: u32) -> *i32 { Trace = Trace * 10 + tag; return &A[n]; }\nfunc flag(tag: i32, b: bool) -> bool { Trace = Trace * 10 + tag; return b; }\nfunc pair() -> (i32, i32) { _ = value(3, 0); return 10, 20; }\nfunc test() -> (i32, i32, i32) { A[index(1, 0)], *ptr(2, 0) = pair(); return Trace, A[0], A[1]; }\n",
        }],
        expected: &[123, 20, 0],
    },
    Evaluation {
        id: "FLOW-02.branch_short_circuit",
        specs: &["FLOW-02"],
        files: &[File {
            path: "main.ond",
            text: "package main\nvar Trace: i32\nvar A: [2]i32\nfunc main() {}\nfunc value(tag: i32, n: i32) -> i32 { Trace = Trace * 10 + tag; return n; }\nfunc index(tag: i32, n: u32) -> u32 { Trace = Trace * 10 + tag; return n; }\nfunc ptr(tag: i32, n: u32) -> *i32 { Trace = Trace * 10 + tag; return &A[n]; }\nfunc flag(tag: i32, b: bool) -> bool { Trace = Trace * 10 + tag; return b; }\nfunc test() -> (i32, i32, i32) {\nif flag(1, false) && flag(2, true) { A[0] = 99; } else { A[0] = 7; }\nif flag(3, true) || flag(4, false) { A[1] = 8; }\nreturn Trace, A[0], A[1];\n}\n",
        }],
        expected: &[13, 7, 8],
    },
    Evaluation {
        id: "FLOW-02.continue_post_break",
        specs: &["FLOW-02"],
        files: &[File {
            path: "main.ond",
            text: "package main\nvar Trace: i32\nvar A: [2]i32\nfunc main() {}\nfunc value(tag: i32, n: i32) -> i32 { Trace = Trace * 10 + tag; return n; }\nfunc index(tag: i32, n: u32) -> u32 { Trace = Trace * 10 + tag; return n; }\nfunc ptr(tag: i32, n: u32) -> *i32 { Trace = Trace * 10 + tag; return &A[n]; }\nfunc flag(tag: i32, b: bool) -> bool { Trace = Trace * 10 + tag; return b; }\nfunc test() -> (i32, i32) {\nvar n = 0\nfor i := value(1, 0); flag(2, i < 3); i = value(4, i + 1) {\nif i == 0 { n = value(3, n + 1); continue; }\nn = value(5, n + 10)\nbreak\n}\nreturn Trace, n;\n}\n",
        }],
        expected: &[123425, 11],
    },
    Evaluation {
        id: "FLOW-02.nested_break_continue",
        specs: &["FLOW-02"],
        files: &[File {
            path: "main.ond",
            text: "package main\nvar Trace: i32\nvar A: [2]i32\nfunc main() {}\nfunc value(tag: i32, n: i32) -> i32 { Trace = Trace * 10 + tag; return n; }\nfunc index(tag: i32, n: u32) -> u32 { Trace = Trace * 10 + tag; return n; }\nfunc ptr(tag: i32, n: u32) -> *i32 { Trace = Trace * 10 + tag; return &A[n]; }\nfunc flag(tag: i32, b: bool) -> bool { Trace = Trace * 10 + tag; return b; }\nfunc test() -> (i32, i32) {\nvar n = 0\nfor i := 0; i < 2; i = i + 1 {\nfor j := 0; j < 3; j = j + 1 {\nif j == 0 { _ = value(1, 0); continue; }\nn = value(2, n + 1)\nbreak\n}\n_ = value(3, 0)\n}\nreturn Trace, n;\n}\n",
        }],
        expected: &[123123, 2],
    },
    Evaluation {
        id: "NAME-02.declaration_shadow",
        specs: &["NAME-02"],
        files: &[File {
            path: "main.ond",
            text: "package main\nvar Trace: i32\nvar A: [2]i32\nfunc main() {}\nfunc value(tag: i32, n: i32) -> i32 { Trace = Trace * 10 + tag; return n; }\nfunc index(tag: i32, n: u32) -> u32 { Trace = Trace * 10 + tag; return n; }\nfunc ptr(tag: i32, n: u32) -> *i32 { Trace = Trace * 10 + tag; return &A[n]; }\nfunc flag(tag: i32, b: bool) -> bool { Trace = Trace * 10 + tag; return b; }\nfunc inner(x: i32) -> i32 {\n{ var x, y = value(1, x + 1), value(2, x + 2); A[0], A[1] = x, y; }\nreturn x;\n}\nfunc test() -> (i32, i32, i32, i32) { var n = inner(5); return Trace, n, A[0], A[1]; }\n",
        }],
        expected: &[12, 5, 6, 7],
    },
    Evaluation {
        id: "NAME-02.if_scope_both_branches",
        specs: &["NAME-02"],
        files: &[File {
            path: "main.ond",
            text: "package main\nvar Trace: i32\nvar A: [2]i32\nfunc main() {}\nfunc value(tag: i32, n: i32) -> i32 { Trace = Trace * 10 + tag; return n; }\nfunc index(tag: i32, n: u32) -> u32 { Trace = Trace * 10 + tag; return n; }\nfunc ptr(tag: i32, n: u32) -> *i32 { Trace = Trace * 10 + tag; return &A[n]; }\nfunc flag(tag: i32, b: bool) -> bool { Trace = Trace * 10 + tag; return b; }\nfunc branch(b: bool) {\nvar x = 5\nif x := value(1, x + 1); b { A[0] = value(2, x); } else { A[0] = value(3, x); }\nA[1] = x\n}\nfunc test() -> (i32, i32, i32) { branch(false); return Trace, A[0], A[1]; }\n",
        }],
        expected: &[13, 6, 5],
    },
    Evaluation {
        id: "NAME-02.for_scope_restore",
        specs: &["NAME-02"],
        files: &[File {
            path: "main.ond",
            text: "package main\nvar Trace: i32\nvar A: [2]i32\nfunc main() {}\nfunc value(tag: i32, n: i32) -> i32 { Trace = Trace * 10 + tag; return n; }\nfunc index(tag: i32, n: u32) -> u32 { Trace = Trace * 10 + tag; return n; }\nfunc ptr(tag: i32, n: u32) -> *i32 { Trace = Trace * 10 + tag; return &A[n]; }\nfunc flag(tag: i32, b: bool) -> bool { Trace = Trace * 10 + tag; return b; }\nfunc test() -> (i32, i32, i32) {\nvar i = 9\nfor i := value(1, 0); i < 1; i = value(3, i + 1) { A[0] = value(2, i + 5); }\nreturn Trace, i, A[0];\n}\n",
        }],
        expected: &[123, 9, 5],
    },
    Evaluation {
        id: "EVAL-02.pointer_base_then_index",
        specs: &["EVAL-02"],
        files: &[File {
            path: "main.ond",
            text: "package main\nvar Trace: i32\nvar A: [2]i32\nfunc main() {}\nfunc value(tag: i32, n: i32) -> i32 { Trace = Trace * 10 + tag; return n; }\nfunc index(tag: i32, n: u32) -> u32 { Trace = Trace * 10 + tag; return n; }\nfunc ptr(tag: i32, n: u32) -> *i32 { Trace = Trace * 10 + tag; return &A[n]; }\nfunc test() -> (i32, i32) { ptr(1, 0)[index(2, 0)] = value(3, 7); return Trace, A[0]; }\n",
        }],
        expected: &[123, 7],
    },
    Evaluation {
        id: "EVAL-02.return_values_left_to_right",
        specs: &["EVAL-02"],
        files: &[File {
            path: "main.ond",
            text: "package main\nvar Trace: i32\nvar A: [2]i32\nfunc main() {}\nfunc value(tag: i32, n: i32) -> i32 { Trace = Trace * 10 + tag; return n; }\nfunc index(tag: i32, n: u32) -> u32 { Trace = Trace * 10 + tag; return n; }\nfunc ptr(tag: i32, n: u32) -> *i32 { Trace = Trace * 10 + tag; return &A[n]; }\nfunc pair() -> (i32, i32) { return value(1, 7), value(2, 8); }\nfunc test() -> (i32, i32, i32) { A[0], A[1] = pair(); return Trace, A[0], A[1]; }\n",
        }],
        expected: &[12, 7, 8],
    },
    Evaluation {
        id: "FLOW-03.early_return_stops_effects",
        specs: &["FLOW-03"],
        files: &[File {
            path: "main.ond",
            text: "package main\nvar Trace: i32\nvar A: [2]i32\nfunc main() {}\nfunc value(tag: i32, n: i32) -> i32 { Trace = Trace * 10 + tag; return n; }\nfunc index(tag: i32, n: u32) -> u32 { Trace = Trace * 10 + tag; return n; }\nfunc ptr(tag: i32, n: u32) -> *i32 { Trace = Trace * 10 + tag; return &A[n]; }\nfunc early() -> i32 { _ = value(1, 0); return value(2, 7); _ = value(3, 99); }\nfunc test() -> (i32, i32) { A[0] = early(); return Trace, A[0]; }\n",
        }],
        expected: &[12, 7],
    },
    Evaluation {
        id: "NAME-02.const_shadow_initializers",
        specs: &["NAME-02"],
        files: &[File {
            path: "main.ond",
            text: "package main\nvar Trace: i32\nvar A: [2]i32\nfunc main() {}\nfunc value(tag: i32, n: i32) -> i32 { Trace = Trace * 10 + tag; return n; }\nfunc index(tag: i32, n: u32) -> u32 { Trace = Trace * 10 + tag; return n; }\nfunc ptr(tag: i32, n: u32) -> *i32 { Trace = Trace * 10 + tag; return &A[n]; }\nfunc test() -> (i32, i32, i32, i32) {\nconst x = 5\n{ const x, y = x + 1, x + 2; A[0], A[1] = value(1, x), value(2, y); }\nreturn Trace, x, A[0], A[1];\n}\n",
        }],
        expected: &[12, 5, 6, 7],
    },
    Evaluation {
        id: "NAME-02.short_shadow_initializers",
        specs: &["NAME-02"],
        files: &[File {
            path: "main.ond",
            text: "package main\nvar Trace: i32\nvar A: [2]i32\nfunc main() {}\nfunc value(tag: i32, n: i32) -> i32 { Trace = Trace * 10 + tag; return n; }\nfunc index(tag: i32, n: u32) -> u32 { Trace = Trace * 10 + tag; return n; }\nfunc ptr(tag: i32, n: u32) -> *i32 { Trace = Trace * 10 + tag; return &A[n]; }\nfunc test() -> (i32, i32, i32, i32) {\nvar x = 5\n{ x, y := value(1, x + 1), value(2, x + 2); A[0], A[1] = x, y; }\nreturn Trace, x, A[0], A[1];\n}\n",
        }],
        expected: &[12, 5, 6, 7],
    },
    Evaluation {
        id: "NAME-02.import_shadow_and_restore",
        specs: &["NAME-02"],
        files: &[
            File {
                path: "main.ond",
                text: "package main\nimport \"shared\"\nvar Trace: i32\nvar Result: i32\ntype S struct { Value: i32; }\nfunc main() {}\nfunc value(tag: i32, n: i32) -> i32 { Trace = Trace * 10 + tag; return n; }\nfunc test() -> (i32, i32, i32) {\n{ var shared = S{Value: value(1, shared.Exported + 1)}; Result = value(2, shared.Value); }\nreturn Trace, Result, shared.Exported;\n}\n",
            },
            File {
                path: "shared/a.ond",
                text: "package shared\nconst Exported = 7\n",
            },
        ],
        expected: &[12, 8, 7],
    },
    Evaluation {
        id: "NAME-02.if_scope_then",
        specs: &["NAME-02"],
        files: &[File {
            path: "main.ond",
            text: "package main\nvar Trace: i32\nvar A: [2]i32\nfunc main() {}\nfunc value(tag: i32, n: i32) -> i32 { Trace = Trace * 10 + tag; return n; }\nfunc index(tag: i32, n: u32) -> u32 { Trace = Trace * 10 + tag; return n; }\nfunc ptr(tag: i32, n: u32) -> *i32 { Trace = Trace * 10 + tag; return &A[n]; }\nfunc test() -> (i32, i32, i32) {\nvar x = 5\nif x := value(1, x + 1); true { A[0] = value(2, x); } else { A[0] = value(3, x); }\nreturn Trace, x, A[0];\n}\n",
        }],
        expected: &[12, 5, 6],
    },
];
