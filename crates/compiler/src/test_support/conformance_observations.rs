use super::{
    compile_image,
    machine::{LinkedMachine, MachineStatus},
};
use std::{fs, path::PathBuf, time::SystemTime};

struct Fixture(PathBuf);

impl Fixture {
    fn new(source: String) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ond-conformance-observation-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("ond.toml"), "[project]\nname = \"test\"\n").unwrap();
        fs::write(
            root.join("main.ond"),
            format!(
                "{source}\nconst DebugIoData:u32=0xFFFF8050 as u32\nfunc writeByte(value:u8){{var io:*u8=DebugIoData as *u8;io[0 as u32]=value}}\nfunc writeWord(value:u32){{writeByte(value as u8);writeByte((value>>8) as u8);writeByte((value>>16) as u8);writeByte((value>>24) as u8)}}\n"
            ),
        )
        .unwrap();
        Self(root)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run_observation(source: &str, main_body: &str, expected: &[u8]) {
    const EMPTY_MAIN: &str = "func main(){}";
    let source = source.replace("func main() {}", EMPTY_MAIN);
    assert_eq!(source.matches(EMPTY_MAIN).count(), 1);
    let source = source.replace(EMPTY_MAIN, &format!("func main(){{{main_body}}}"));
    let root = Fixture::new(source);
    let image = compile_image(&root.0).unwrap();
    let mut machine = LinkedMachine::from_image(&image).unwrap();
    assert_eq!(
        machine.run_steps(2_000_000).unwrap(),
        MachineStatus::Exited(0)
    );
    assert_eq!(machine.debug_output(), expected);
}

#[test]
fn num02_u8_runtime_add_reduced_kagura_observation() {
    run_observation(
        "package main\nfunc observe(x:u8)->u8{return x+(256-1)}\nfunc main(){}\n",
        "writeWord(observe(1) as u32)",
        &[0, 0, 0, 0],
    );
}

#[test]
fn kagura_observation_word_output_preserves_all_bytes() {
    run_observation(
        "package main\nfunc main(){}\n",
        "writeWord(0x78563412)",
        &[0x12, 0x34, 0x56, 0x78],
    );
}

#[test]
fn op_01_custom_before_inherited_builtin_kagura_observation() {
    run_observation(
        r###"package main
type Number i32
operator (self: Number) +(rhs: Number) -> bool { return true }
func observe() -> bool { return (1 as Number) + (2 as Number) }
func main() {}
"###,
        r###"if observe(){writeByte(1)}else{writeByte(0)}"###,
        &[1],
    );
}

#[test]
fn bi_02_custom_array_len_priority_kagura_observation() {
    run_observation(
        r###"package main
type Buffer [2]u8
operator (self: Buffer) len() -> u32 { return 99 }
func observe() -> u32 { var b: Buffer; return len(b) }
func main() {}
"###,
        r###"writeWord(observe() as u32)"###,
        &[99, 0, 0, 0],
    );
}

#[test]
fn eval_02_setter_only_simple_assignment_kagura_observation() {
    run_observation(
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
        r###"var a,b=observe();writeWord(a);writeWord(b)"###,
        &[210, 4, 0, 0, 10, 0, 0, 0],
    );
}

#[test]
fn eval_02_custom_compound_stage_order_kagura_observation() {
    run_observation(
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
        r###"var a,b=observe();writeWord(a);writeWord(b)"###,
        &[64, 226, 1, 0, 4, 0, 0, 0],
    );
}

#[test]
fn op_01_literal_parameter_kagura_observation() {
    run_observation(
        r###"package main
type N i32
operator (self:N) +(rhs:u8)->bool{return true}
func observe()->bool{n:=2 as N;return n+1}
func main(){}
"###,
        r###"if observe(){writeByte(1)}else{writeByte(0)}"###,
        &[1],
    );
}

#[test]
fn op_01_literal_parenthesized_kagura_observation() {
    run_observation(
        r###"package main
type N i32
operator (self:N) +(rhs:u8)->bool{return true}
func observe()->bool{n:=2 as N;return n+(1)}
func main(){}
"###,
        r###"if observe(){writeByte(1)}else{writeByte(0)}"###,
        &[1],
    );
}

#[test]
fn op_01_literal_expression_context_kagura_observation() {
    run_observation(
        r###"package main
type N i32
operator (self:N) +(rhs:u8)->bool{return true}
func observe()->bool{n:=2 as N;return n+(1+2)}
func main(){}
"###,
        r###"if observe(){writeByte(1)}else{writeByte(0)}"###,
        &[1],
    );
}

#[test]
fn op_01_nil_parameter_context_kagura_observation() {
    run_observation(
        r###"package main
type N i32
operator (self:N) +(rhs:*u8)->bool{return true}
func observe()->bool{n:=2 as N;return n+nil}
func main(){}
"###,
        r###"if observe(){writeByte(1)}else{writeByte(0)}"###,
        &[1],
    );
}

#[test]
fn op_01_explicit_operand_builtin_kagura_observation() {
    run_observation(
        r###"package main
type N i32
operator (self:N) +(rhs:u8)->bool{return true}
func observe()->N{n:=2 as N;return n+(1 as N)}
func main(){}
"###,
        r###"writeWord(observe() as u32)"###,
        &[3, 0, 0, 0],
    );
}

#[test]
fn op_01_literal_kind_accept_kagura_observation() {
    run_observation(
        r###"package main
type N i32
operator (self:N) +(rhs:bool)->bool{return true}
func observe()->bool{n:=2 as N;return n+true}
func main(){}
"###,
        r###"if observe(){writeByte(1)}else{writeByte(0)}"###,
        &[1],
    );
}

#[test]
fn eval_02_custom_read_builtin_write_kagura_observation() {
    run_observation(
        r###"package main
type Buffer [1]u32
var Trace:u32
operator (self:*Buffer) [](i:u32)->u32{Trace=Trace*10+1;return 100}
func rhs()->u32{Trace=Trace*10+2;return 1}
func observe(i:u32)->(u32,u32){var b:Buffer;b[i]+=rhs();return Trace,((&b) as u32 as *u32)[0]}
func main(){}
"###,
        r###"var a,b=observe(0);writeWord(a);writeWord(b)"###,
        &[12, 0, 0, 0, 101, 0, 0, 0],
    );
}

#[test]
fn eval_02_custom_read_write_bounds_kagura_observation() {
    let root = Fixture::new(
        r###"package main
type Buffer [1]u32
var Trace:u32
operator (self:*Buffer) [](i:u32)->u32{Trace=Trace*10+1;return 100}
func rhs()->u32{Trace=Trace*10+2;return 1}
func observe(i:u32)->(u32,u32){var b:Buffer;b[i]+=rhs();return Trace,((&b) as u32 as *u32)[0]}
func main(){}
"###
        .replace("func main(){}", "func main(){observe(1)}"),
    );
    let image = compile_image(&root.0).unwrap();
    let mut machine = LinkedMachine::from_image(&image).unwrap();
    assert!(
        matches!(
            machine.run_steps(2_000_000),
            Err(kagura::Fault::InvalidInstruction { .. })
        ),
        "EVAL-02.custom-read-write-bounds"
    );
    assert_eq!(
        machine.read_word(image.symbols["main.Trace"]),
        12,
        "EVAL-02.custom-read-write-bounds"
    );
    assert_eq!(machine.status(), MachineStatus::Running);
}

#[test]
fn eval_02_builtin_read_custom_write_kagura_observation() {
    run_observation(
        r###"package main
type Buffer [1]u32
var Trace:u32
operator (self:*Buffer) []=(i:u32,value:u32){Trace=Trace*10+2;(self as u32 as *u32)[i]=value+10}
func rhs()->u32{Trace=Trace*10+1;return 1}
func observe()->(u32,u32){var b:Buffer;((&b) as u32 as *u32)[0]=7;b[0]+=rhs();return Trace,((&b) as u32 as *u32)[0]}
func main(){}
"###,
        r###"var a,b=observe();writeWord(a);writeWord(b)"###,
        &[12, 0, 0, 0, 18, 0, 0, 0],
    );
}

#[test]
fn op_01_typed_index_independent_kagura_observation() {
    run_observation(
        r###"package main
type Buffer [1]u32
operator (self:*Buffer) [](i:u8)->u32{return 100}
operator (self:*Buffer) []=(i:u16,value:u32){(self as u32 as *u32)[i as u32]=value+10}
func observe()->u32{var b:Buffer;b[0 as u8]+=1;return ((&b) as u32 as *u32)[0]}
func main(){}
"###,
        r###"writeWord(observe() as u32)"###,
        &[101, 0, 0, 0],
    );
}

#[test]
fn op_01_typed_index_expression_kagura_observation() {
    run_observation(
        r###"package main
type Buffer [1]u32
operator (self:*Buffer) [](i:u8)->u32{return 100}
operator (self:*Buffer) []=(i:u16,value:u32){(self as u32 as *u32)[i as u32]=value+10}
func index()->u8{return 0}
func observe()->u32{var b:Buffer;var i:u8=0;b[i+0]+=1;return ((&b) as u32 as *u32)[0]}
func main(){}
"###,
        r###"writeWord(observe() as u32)"###,
        &[101, 0, 0, 0],
    );
}

#[test]
fn op_01_call_index_expression_kagura_observation() {
    run_observation(
        r###"package main
type Buffer [1]u32
operator (self:*Buffer) [](i:u8)->u32{return 100}
operator (self:*Buffer) []=(i:u16,value:u32){(self as u32 as *u32)[i as u32]=value+10}
func index()->u8{return 0}
func observe()->u32{var b:Buffer;var i:u8=0;b[index()+0]+=1;return ((&b) as u32 as *u32)[0]}
func main(){}
"###,
        r###"writeWord(observe() as u32)"###,
        &[101, 0, 0, 0],
    );
}

#[test]
fn op_01_cast_index_expression_kagura_observation() {
    run_observation(
        r###"package main
type Buffer [1]u32
operator (self:*Buffer) [](i:u8)->u32{return 100}
operator (self:*Buffer) []=(i:u16,value:u32){(self as u32 as *u32)[i as u32]=value+10}
func index()->u8{return 0}
func observe()->u32{var b:Buffer;var i:u8=0;b[(i+0) as u8]+=1;return ((&b) as u32 as *u32)[0]}
func main(){}
"###,
        r###"writeWord(observe() as u32)"###,
        &[101, 0, 0, 0],
    );
}

#[test]
fn art01_generic_value_kagura_observation() {
    let source = r###"package main
import "p/lib"
func observe()->u32{var b:lib.Box[u8];var f:func(u8)->u8=lib.Identity[u8];return (lib.Identity[u8](1) as u32)+(f(2) as u32)+lib.Identity[u32](4)+(b.Get() as u32)+((b+3) as u32)}
func main(){}
"###;
    let library = r###"package lib
type Box[T] struct{Value:T}
type Holder struct{Small:Box[u8];Nested:Box[Box[u8]]}
func Identity[T](x:T)->T{return x}
func Unused[T](x:T)->T{return x}
func (b:Box[T]) Get()->T{return b.Value}
operator (b:Box[T]) +(rhs:T)->T{return b.Value+rhs}
"###;

    let root = Fixture::new(source.replace("func main(){}", "func main(){writeWord(observe())}"));
    fs::write(root.0.join("ond.toml"), "[project]\nname = \"p\"\n").unwrap();
    fs::create_dir(root.0.join("lib")).unwrap();
    fs::write(root.0.join("lib/lib.ond"), library).unwrap();
    let image = compile_image(&root.0).expect("ART-01.generic-value compile precondition");
    let mut machine = LinkedMachine::from_image(&image).unwrap();
    assert_eq!(
        machine.run_steps(2_000_000).unwrap(),
        MachineStatus::Exited(0)
    );
    assert_eq!(
        machine.debug_output(),
        [10, 0, 0, 0],
        "ART-01.generic-value"
    );
}
