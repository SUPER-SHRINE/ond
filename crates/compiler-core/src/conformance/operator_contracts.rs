//! 仕様由来の固定入力と期待。operator-contracts.mdの各caseに対応する。
use crate::{diagnostic::Severity, test_support::compile};
fn accept(id: &str, source: &str) {
    compile(source, None).unwrap_or_else(|e| panic!("{id}: {e}"));
}
fn reject(id: &str, source: &str, cause: &str) {
    reject_cause(id, source, cause, None);
}
fn reject_operand(id: &str, source: &str, expression: &str, operand: &str) {
    reject_cause(id, source, expression, Some(operand));
}
fn reject_cause(id: &str, source: &str, cause: &str, operand: Option<&str>) {
    crate::ond::parse_source_file(crate::source::FileId(0), source)
        .expect("operator case must parse");
    let error = match compile(source, None) {
        Err(error) => error,
        Ok(_) => panic!("{id}: expected semantic rejection, compilation succeeded"),
    };
    let start = source.find(cause).unwrap();
    let end = start + cause.len();
    assert!(
        error
            .diagnostics()
            .iter()
            .any(|d| d.severity == Severity::Error
                && !d.primary.span.is_synthetic()
                && d.primary.span.start < d.primary.span.end
                && d.primary.span.start >= start
                && d.primary.span.end <= end
                && operand.is_none_or(|operand| {
                    let at = start + cause.find(operand).unwrap();
                    d.primary.span.start < at + operand.len() && d.primary.span.end > at
                })),
        "{id}: {error}"
    );
}

#[test]
fn op_01_custom_before_inherited_builtin() {
    accept(
        "OP-01.custom-before-inherited-builtin",
        r###"package main
type Number i32
operator (self: Number) +(rhs: Number) -> bool { return true }
func observe() -> bool { return (1 as Number) + (2 as Number) }
func main() {}
"###,
    );
}

#[test]
fn op_01_custom_result_context_reject() {
    reject(
        "OP-01.custom-result-context-reject",
        r###"package main
type Number i32
operator (self: Number) +(rhs: Number) -> bool { return true }
func observe() -> Number { return (1 as Number) + (2 as Number) }
func main() {}
"###,
        r###"(1 as Number) + (2 as Number)"###,
    );
}

#[test]
fn op_01_custom_one_result_declaration() {
    reject(
        "OP-01.custom-one-result-declaration",
        r###"package main
type Number i32
operator (self: Number) +(rhs: Number) {}
func main() {}
"###,
        r###"operator (self: Number) +(rhs: Number) {}"###,
    );
}

#[test]
fn bi_02_custom_array_len_priority() {
    accept(
        "BI-02.custom-array-len-priority",
        r###"package main
type Buffer [2]u8
operator (self: Buffer) len() -> u32 { return 99 }
func observe() -> u32 { var b: Buffer; return len(b) }
func main() {}
"###,
    );
}

#[test]
fn bi_02_custom_len_constant_reject() {
    reject(
        "BI-02.custom-len-constant-reject",
        r###"package main
type Buffer [2]u8
operator (self: Buffer) len() -> u32 { return 99 }
const X = len(Buffer{})
func main() {}
"###,
        r###"const X = len(Buffer{})"###,
    );
}

#[test]
fn eval_02_setter_only_simple_assignment() {
    accept(
        "EVAL-02.setter-only-simple-assignment",
        r###"package main
type Table struct { value: u32 }
var T: Table
var Trace: u32
func base() -> *Table { Trace = Trace * 10 + 1; return &T }
func index() -> u32 { Trace = Trace * 10 + 2; return 1 }
func rhs() -> u32 { Trace = Trace * 10 + 3; return 9 }
operator (self: *Table) []=(i: u32, value: u32) { Trace = Trace * 10 + 4; self.value = value + i }
func observe() -> (u32, u32) { base()[index()] = rhs(); return Trace, T.value }
func main() {}
"###,
    );
}

#[test]
fn eval_02_custom_compound_stage_order() {
    accept(
        "EVAL-02.custom-compound-stage-order",
        r###"package main
type Table struct { value: u32 }
type Getter u32
type Written struct { value: u32 }
var T: Table
var Trace: u32
func base() -> *Table { Trace = Trace * 10 + 1; return &T }
func index() -> u32 { Trace = Trace * 10 + 2; return 0 }
operator (self: *Table) [](i: u32) -> Getter { Trace = Trace * 10 + 3; return self.value as Getter }
func rhs() -> u32 { Trace = Trace * 10 + 4; return 4 }
operator (self: Getter) +(rhs: u32) -> Written { Trace = Trace * 10 + 5; return Written{value: (self as u32) + rhs} }
operator (self: *Table) []=(i: u32, value: Written) { Trace = Trace * 10 + 6; self.value = value.value }
func observe() -> (u32, u32) { base()[index()] += rhs(); return Trace, T.value }
func main() {}
"###,
    );
}

#[test]
fn op_01_literal_parameter() {
    accept(
        "OP-01.literal-parameter",
        r###"package main
type N i32
operator (self:N) +(rhs:u8)->bool{return true}
func observe()->bool{n:=2 as N;return n+1}
func main(){}
"###,
    );
}

#[test]
fn op_01_literal_parenthesized() {
    accept(
        "OP-01.literal-parenthesized",
        r###"package main
type N i32
operator (self:N) +(rhs:u8)->bool{return true}
func observe()->bool{n:=2 as N;return n+(1)}
func main(){}
"###,
    );
}

#[test]
fn op_01_literal_expression_context() {
    accept(
        "OP-01.literal-expression-context",
        r###"package main
type N i32
operator (self:N) +(rhs:u8)->bool{return true}
func observe()->bool{n:=2 as N;return n+(1+2)}
func main(){}
"###,
    );
}

#[test]
fn op_01_nil_parameter_context() {
    accept(
        "OP-01.nil-parameter-context",
        r###"package main
type N i32
operator (self:N) +(rhs:*u8)->bool{return true}
func observe()->bool{n:=2 as N;return n+nil}
func main(){}
"###,
    );
}

#[test]
fn op_01_literal_overflow() {
    reject_operand(
        "OP-01.literal-overflow",
        r###"package main
type N i32
operator (self:N) +(rhs:u8)->bool{return true}
func observe()->bool{n:=2 as N;return n+256}
func main(){}
"###,
        r###"n+256"###,
        r###"256"###,
    );
}

#[test]
fn op_01_literal_negative() {
    reject_operand(
        "OP-01.literal-negative",
        r###"package main
type N i32
operator (self:N) +(rhs:u8)->bool{return true}
func observe()->bool{n:=2 as N;return n+(-1)}
func main(){}
"###,
        r###"n+(-1)"###,
        r###"(-1)"###,
    );
}

#[test]
fn op_01_literal_result_context() {
    reject(
        "OP-01.literal-result-context",
        r###"package main
type N i32
operator (self:N) +(rhs:u8)->bool{return true}
func observe()->N{n:=2 as N;return n+1}
func main(){}
"###,
        r###"n+1"###,
    );
}

#[test]
fn op_01_explicit_operand_builtin() {
    accept(
        "OP-01.explicit-operand-builtin",
        r###"package main
type N i32
operator (self:N) +(rhs:u8)->bool{return true}
func observe()->N{n:=2 as N;return n+(1 as N)}
func main(){}
"###,
    );
}

#[test]
fn op_01_literal_kind_reject() {
    reject_operand(
        "OP-01.literal-kind-reject",
        r###"package main
type N i32
operator (self:N) +(rhs:bool)->bool{return true}
func observe()->bool{n:=2 as N;return n+1}
func main(){}
"###,
        r###"n+1"###,
        r###"1"###,
    );
}

#[test]
fn op_01_literal_kind_accept() {
    accept(
        "OP-01.literal-kind-accept",
        r###"package main
type N i32
operator (self:N) +(rhs:bool)->bool{return true}
func observe()->bool{n:=2 as N;return n+true}
func main(){}
"###,
    );
}

#[test]
fn eval_02_custom_read_builtin_write() {
    accept(
        "EVAL-02.custom-read-builtin-write",
        r###"package main
type Buffer [1]u32
var Trace:u32
operator (self:*Buffer) [](i:u32)->u32{Trace=Trace*10+1;return 100}
func rhs()->u32{Trace=Trace*10+2;return 1}
func observe(i:u32)->(u32,u32){var b:Buffer;b[i]+=rhs();return Trace,((&b) as u32 as *u32)[0]}
func main(){}
"###,
    );
}

#[test]
fn eval_02_custom_read_write_bounds() {
    accept(
        "EVAL-02.custom-read-write-bounds",
        r###"package main
type Buffer [1]u32
var Trace:u32
operator (self:*Buffer) [](i:u32)->u32{Trace=Trace*10+1;return 100}
func rhs()->u32{Trace=Trace*10+2;return 1}
func observe(i:u32)->(u32,u32){var b:Buffer;b[i]+=rhs();return Trace,((&b) as u32 as *u32)[0]}
func main(){}
"###,
    );
}

#[test]
fn eval_02_builtin_read_custom_write() {
    accept(
        "EVAL-02.builtin-read-custom-write",
        r###"package main
type Buffer [1]u32
var Trace:u32
operator (self:*Buffer) []=(i:u32,value:u32){Trace=Trace*10+2;(self as u32 as *u32)[i]=value+10}
func rhs()->u32{Trace=Trace*10+1;return 1}
func observe()->(u32,u32){var b:Buffer;((&b) as u32 as *u32)[0]=7;b[0]+=rhs();return Trace,((&b) as u32 as *u32)[0]}
func main(){}
"###,
    );
}

#[test]
fn op_01_shared_index_context_reject() {
    reject(
        "OP-01.shared-index-context-reject",
        r###"package main
type Buffer [1]u32
operator (self:*Buffer) [](i:u8)->u32{return 100}
operator (self:*Buffer) []=(i:u16,value:u32){(self as u32 as *u32)[i as u32]=value+10}
func observe()->u32{var b:Buffer;b[0]+=1;return ((&b) as u32 as *u32)[0]}
func main(){}
"###,
        r###"b[0]+=1"###,
    );
}

#[test]
fn op_01_typed_index_independent() {
    accept(
        "OP-01.typed-index-independent",
        r###"package main
type Buffer [1]u32
operator (self:*Buffer) [](i:u8)->u32{return 100}
operator (self:*Buffer) []=(i:u16,value:u32){(self as u32 as *u32)[i as u32]=value+10}
func observe()->u32{var b:Buffer;b[0 as u8]+=1;return ((&b) as u32 as *u32)[0]}
func main(){}
"###,
    );
}

#[test]
fn op_01_typed_index_expression() {
    accept(
        "OP-01.typed-index-expression",
        r###"package main
type Buffer [1]u32
operator (self:*Buffer) [](i:u8)->u32{return 100}
operator (self:*Buffer) []=(i:u16,value:u32){(self as u32 as *u32)[i as u32]=value+10}
func index()->u8{return 0}
func observe()->u32{var b:Buffer;var i:u8=0;b[i+0]+=1;return ((&b) as u32 as *u32)[0]}
func main(){}
"###,
    );
}

#[test]
fn op_01_call_index_expression() {
    accept(
        "OP-01.call-index-expression",
        r###"package main
type Buffer [1]u32
operator (self:*Buffer) [](i:u8)->u32{return 100}
operator (self:*Buffer) []=(i:u16,value:u32){(self as u32 as *u32)[i as u32]=value+10}
func index()->u8{return 0}
func observe()->u32{var b:Buffer;var i:u8=0;b[index()+0]+=1;return ((&b) as u32 as *u32)[0]}
func main(){}
"###,
    );
}

#[test]
fn op_01_untyped_index_expression() {
    reject(
        "OP-01.untyped-index-expression",
        r###"package main
type Buffer [1]u32
operator (self:*Buffer) [](i:u8)->u32{return 100}
operator (self:*Buffer) []=(i:u16,value:u32){(self as u32 as *u32)[i as u32]=value+10}
func index()->u8{return 0}
func observe()->u32{var b:Buffer;var i:u8=0;b[0+0]+=1;return ((&b) as u32 as *u32)[0]}
func main(){}
"###,
        r###"b[0+0]+=1"###,
    );
}

#[test]
fn op_01_cast_index_expression() {
    accept(
        "OP-01.cast-index-expression",
        r###"package main
type Buffer [1]u32
operator (self:*Buffer) [](i:u8)->u32{return 100}
operator (self:*Buffer) []=(i:u16,value:u32){(self as u32 as *u32)[i as u32]=value+10}
func index()->u8{return 0}
func observe()->u32{var b:Buffer;var i:u8=0;b[(i+0) as u8]+=1;return ((&b) as u32 as *u32)[0]}
func main(){}
"###,
    );
}
