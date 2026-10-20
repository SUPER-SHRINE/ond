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
fn source_line_information_uses_runtime_wrapping_and_narrowing() {
    let source = format!(
        "package main\n{}func observe()->(u8,u32){{return thisLine() as u8,thisLine()+4294967295}}\nfunc main(){{}}\n",
        "\n".repeat(260)
    );
    let line = source[..source.find("thisLine()").unwrap()]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count() as u32
        + 1;
    let mut expected = vec![line as u8];
    expected.extend(line.wrapping_sub(1).to_le_bytes());
    run_observation(
        &source,
        "a,b:=observe();writeByte(a);writeWord(b)",
        &expected,
    );
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
fn source_line_information_is_stable_in_generic_defer_and_short_circuit() {
    let source = "package main\nfunc generic[T](x:T)->u32{_=x;return thisLine()}\nfunc delayed(){defer {writeWord(thisLine())}}\nfunc main(){}\n";
    let mut expected = Vec::new();
    expected.extend(2u32.to_le_bytes());
    expected.extend(3u32.to_le_bytes());
    run_observation(
        source,
        "writeWord(generic[u8](1));delayed();_=false&&(thisLine()==0)",
        &expected,
    );
}

#[test]
fn scalar_compound_custom_operators_preserve_operand_order_and_single_evaluation() {
    let source = "package main\ntype N u32\ntype Record struct{Value:N}\nvar Storage:Record\nvar Trace:u32\noperator(n:N)+(x:N)->N{Trace=Trace*10+3;return ((n as u32)+(x as u32)+10) as N}\nfunc base()->*Record{Trace=Trace*10+1;return &Storage}\nfunc pointer()->*N{Trace=Trace*10+1;return &Storage.Value}\nfunc rhs()->N{Trace=Trace*10+2;Storage.Value=100 as N;return 2 as N}\nfunc observe()->(u32,u32){Storage.Value=1 as N;Trace=0;base().Value+=rhs();var a=Trace;Trace=0;*pointer()+=rhs();return Storage.Value as u32,a*1000+Trace}\nfunc main(){}\n";
    let mut expected = Vec::new();
    expected.extend(25u32.to_le_bytes());
    expected.extend(123123u32.to_le_bytes());
    run_observation(
        source,
        "value,trace:=observe();writeWord(value);writeWord(trace)",
        &expected,
    );
}

#[test]
fn simple_setter_uses_typed_builtin_fallback_and_custom_literal_context() {
    let source = "package main\ntype B [1]u32\nvar Trace:u32\noperator(b:*B) []=(i:u16,x:u16){Trace+=1;(b as u32 as *u32)[i as u32]=x as u32}\nfunc observe()->(u32,u32){var b:B;var i:u8;var x:u32=5;b[i]=1;b[0 as u8]=2;b[0 as u16]=x;b[0]=3;return b[0],Trace}\nfunc main(){}\n";
    let mut expected = Vec::new();
    expected.extend(3u32.to_le_bytes());
    expected.extend(1u32.to_le_bytes());
    run_observation(
        source,
        "value,trace:=observe();writeWord(value);writeWord(trace)",
        &expected,
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

#[test]
fn compound_index_updates_preserve_effect_order_and_fallback() {
    run_observation(
        r###"package main
var Trace:u32
type B struct{Value:u32}
operator(b:*B) [](i:u8)->u32{Trace=Trace*10+2;return b.Value}
operator(b:*B) []=(i:u8,v:u32){Trace=Trace*10+4;b.Value=v}
func index()->u8{Trace=Trace*10+1;return 0}
func rhs()->u32{Trace=Trace*10+3;return 1}
func observe()->(u32,u32){var b:B;b[index()]+=rhs();b[0]++;b[0]--;return Trace,b.Value}
func main(){}
"###,
        r###"var t,v=observe();writeWord(t);writeWord(v)"###,
        &[152, 84, 188, 0, 1, 0, 0, 0],
    );
    run_observation(
        r###"package main
type B [1]u32
operator(b:*B) [](i:u8)->u32{return 7}
operator(b:*B) []=(i:u16,v:bool){}
func observe()->u32{var b:B;b[0]+=1;return ((&b) as u32 as *u32)[0]}
func main(){}
"###,
        r###"writeWord(observe())"###,
        &[8, 0, 0, 0],
    );
}

#[test]
fn indexed_float_updates_preserve_value_and_effect_counts() {
    run_observation(
        r###"package main
var Bases:u32
var Indices:u32
func base(p:*f32)->*f32{Bases+=1;return p}
func index()->u32{Indices+=1;return 0}
func observe()->(u32,u32,u32){var a:[1]f32;a[0]=2.0;a[0]++;a[0]--;base(&a[0])[index()]++;base(&a[0])[index()]--;return a[0] as u32,Bases,Indices}
func main(){}
"###,
        r###"var v,b,i=observe();writeWord(v);writeWord(b);writeWord(i)"###,
        &[2, 0, 0, 0, 2, 0, 0, 0, 2, 0, 0, 0],
    );
    run_observation(
        r###"package main
var Trace:u32
type F f32
operator(n:F)+(v:u8)->F{return ((n as f32)+(v as f32)) as F}
operator(n:F)-(v:u8)->F{return ((n as f32)-(v as f32)) as F}
type B struct{Value:F}
operator(b:*B) [](i:u8)->F{Trace=Trace*10+2;return b.Value}
operator(b:*B) []=(i:u8,v:F){Trace=Trace*10+3;b.Value=v}
func index()->u8{Trace=Trace*10+1;return 0}
func observe()->(u32,u32){var b:B;b.Value=2.0 as F;b[index()]++;b[index()]--;return b.Value as u32,Trace}
func main(){}
"###,
        r###"var v,t=observe();writeWord(v);writeWord(t)"###,
        &[2, 0, 0, 0, 243, 224, 1, 0],
    );
}

#[test]
fn compound_index_reference_receivers_keep_source_storage_identity() {
    run_observation(
        r###"package main
type N struct{Value:u32}
var Address:u32
var Target:*N
var Trace:u32
operator(n:*N)+(v:u32)->N{Trace=Trace*10+3;Address=n as u32;return *n}
func index()->u32{Trace=Trace*10+1;return 0}
func rhs()->u32{Trace=Trace*10+2;Target.Value=9;return 1}
func observe()->(bool,u32,u32){var a:[1]N;Target=&a[0];a[index()]+=rhs();return Address==((&a[0]) as u32),Trace,a[0].Value}
func main(){}
"###,
        r###"var equal,t,v=observe();if equal{writeWord(1)}else{writeWord(0)};writeWord(t);writeWord(v)"###,
        &[1, 0, 0, 0, 123, 0, 0, 0, 9, 0, 0, 0],
    );
    run_observation(
        r###"package main
type N struct{Value:u32}
operator(n:N)+(v:u32)->N{return N{Value:n.Value+v}}
type B struct{Value:N}
operator(b:*B) [](i:u32)->*N{return &b.Value}
operator(b:*B) []=(i:u32,v:N){b.Value=v}
func observe()->u32{var b:B;b.Value.Value=2;b[0]+=1;return b.Value.Value}
func main(){}
"###,
        r###"writeWord(observe())"###,
        &[3, 0, 0, 0],
    );
}

#[test]
fn compound_index_array_value_base_is_captured_once() {
    run_observation(
        r###"package main
var Bases:u32
var Written:u32
type B [1]u32
func base()->B{Bases+=1;return B{2}}
operator(b:B) []=(i:u32,v:u32){Written=v}
func observe()->(u32,u32){base()[0]+=1;return Bases,Written}
func main(){}
"###,
        r###"var b,w=observe();writeWord(b);writeWord(w)"###,
        &[1, 0, 0, 0, 3, 0, 0, 0],
    );
}
