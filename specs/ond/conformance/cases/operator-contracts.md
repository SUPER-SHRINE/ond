# Operator解決と代入の契約case
<!-- ond-spec-id: conformance/cases/operator-contracts -->

[共通case契約](./README.md#入力と期待結果の読み方)のOnd v1、manifest、fresh初期状態を使う。\
入力欄は`main.ond`全文であり、Core拒否行は共通case契約に従いsourceに対応する原因位置とcompile失敗を比較する。\
MemoryVm行は受理後、各行のargument列（省略時[]）でmain.observeを1回呼び返値列を比較する。\
規範定義元は[operator](../../language/v1/expressions.md#user-defined-operator)、[代入](../../language/v1/statements.md#静的成立条件)、[const適格性](../../language/v1/declarations.md#宣言)とする。

| Case ID | 要件 | main.ond全文 | 操作・期待 |
| --- | --- | --- | --- |
| `OP-01.custom-before-inherited-builtin` | `OP-01` | <code>package main<br>type Number i32<br>operator (self: Number) +(rhs: Number) -&gt; bool { return true }<br>func observe() -&gt; bool { return (1 as Number) + (2 as Number) }<br>func main() {}<br></code> | MemoryVm: main.observe([]) -> [1] |
| `OP-01.custom-result-context-reject` | `OP-01` | <code>package main<br>type Number i32<br>operator (self: Number) +(rhs: Number) -&gt; bool { return true }<br>func observe() -&gt; Number { return (1 as Number) + (2 as Number) }<br>func main() {}<br></code> | Core: compile error。選択済みcustom結果boolと要求Numberが不一致 |
| `OP-01.custom-one-result-declaration` | `OP-01` | <code>package main<br>type Number i32<br>operator (self: Number) +(rhs: Number) {}<br>func main() {}<br></code> | Core: compile error。未使用でもresult数0は不成立 |
| `BI-02.custom-array-len-priority` | `BI-02` | <code>package main<br>type Buffer [2]u8<br>operator (self: Buffer) len() -&gt; u32 { return 99 }<br>func observe() -&gt; u32 { var b: Buffer; return len(b) }<br>func main() {}<br></code> | MemoryVm: main.observe([]) -> [99] |
| `BI-02.custom-len-constant-reject` | `BI-02, DECL-02` | <code>package main<br>type Buffer [2]u8<br>operator (self: Buffer) len() -&gt; u32 { return 99 }<br>const X = len(Buffer{})<br>func main() {}<br></code> | Core: compile error。custom lenは通常callのconst適格性を適用 |
| `EVAL-02.setter-only-simple-assignment` | `EVAL-02, OP-01` | <code>package main<br>type Table struct { value: u32 }<br>var T: Table<br>var Trace: u32<br>func base() -&gt; *Table { Trace = Trace * 10 + 1; return &amp;T }<br>func index() -&gt; u32 { Trace = Trace * 10 + 2; return 1 }<br>func rhs() -&gt; u32 { Trace = Trace * 10 + 3; return 9 }<br>operator (self: *Table) []=(i: u32, value: u32) { Trace = Trace * 10 + 4; self.value = value + i }<br>func observe() -&gt; (u32, u32) { base()[index()] = rhs(); return Trace, T.value }<br>func main() {}<br></code> | MemoryVm: main.observe([]) -> [1234,10] |
| `EVAL-02.custom-compound-stage-order` | `EVAL-02, OP-01` | <code>package main<br>type Table struct { value: u32 }<br>type Getter u32<br>type Written struct { value: u32 }<br>var T: Table<br>var Trace: u32<br>func base() -&gt; *Table { Trace = Trace * 10 + 1; return &amp;T }<br>func index() -&gt; u32 { Trace = Trace * 10 + 2; return 0 }<br>operator (self: *Table) [](i: u32) -&gt; Getter { Trace = Trace * 10 + 3; return self.value as Getter }<br>func rhs() -&gt; u32 { Trace = Trace * 10 + 4; return 4 }<br>operator (self: Getter) +(rhs: u32) -&gt; Written { Trace = Trace * 10 + 5; return Written{value: (self as u32) + rhs} }<br>operator (self: *Table) []=(i: u32, value: Written) { Trace = Trace * 10 + 6; self.value = value.value }<br>func observe() -&gt; (u32, u32) { base()[index()] += rhs(); return Trace, T.value }<br>func main() {}<br></code> | MemoryVm: main.observe([]) -> [123456,4] |

構文flowの条件は[static-flow](./static-flow.md)の既存caseを同じID・入力・可否で比較する。\
特に条件省略loop、定数conditionを持つif/for、return/continue/内側終端loop後のbreakを、[静的成立条件](../../language/v1/statements.md#静的成立条件)から判定する。

## Custom parameter文脈とliteral
<!-- ond-spec-id: conformance/operator-contracts/contextual-literals -->

各行は次のtemplateのPARAM、RESULT、EXPRを置換した独立projectとし、共通manifestを使う。Core拒否は原因の型/範囲不成立、MemoryVmはfresh状態のmain.observe([])を比較する。要件はOP-01であり、定義元は[operatorの文脈型付け](../../language/v1/expressions.md#user-defined-operator)である。

```ond
package main
type N i32
operator (self:N) +(rhs:PARAM)->bool{return true}
func observe()->RESULT{n:=2 as N;return EXPR}
func main(){}
```

| Case ID | PARAM | RESULT | EXPR | 操作・期待 |
| --- | --- | --- | --- | --- |
| OP-01.literal-parameter | u8 | bool | n+1 | MemoryVm: [1] |
| OP-01.literal-parenthesized | u8 | bool | n+(1) | MemoryVm: [1] |
| OP-01.literal-overflow | u8 | bool | n+256 | Core: literalのu8範囲error |
| OP-01.literal-negative | u8 | bool | n+(-1) | Core: literalのu8範囲error |
| OP-01.literal-result-context | u8 | N | n+1 | Core: custom結果boolと要求Nの不一致 |
| OP-01.explicit-operand-builtin | u8 | N | n+(1 as N) | MemoryVm: [3]。型確定operandに適合するbuiltinを選択 |
| OP-01.literal-kind-reject | bool | bool | n+1 | Core: bool文脈への整数literal不成立 |
| OP-01.literal-kind-accept | bool | bool | n+true | MemoryVm: [1] |

## Indexの独立したread/write解決
<!-- ond-spec-id: conformance/operator-contracts/mixed-index -->

各入力はmain.ond全文とし、要件EVAL-02/OP-01、定義元は[compound代入](../../language/v1/statements.md#静的成立条件)である。各呼出しはMemoryVmのfresh状態を使い、raw pointerによるstorage読取りを通常のbuiltinアクセスとして行う。

| Case ID | main.ond全文 | 操作・期待 |
| --- | --- | --- |
| EVAL-02.custom-read-builtin-write | <code>package main<br>type Buffer [1]u32<br>var Trace:u32<br>operator (self:*Buffer) [](i:u32)-&gt;u32{Trace=Trace*10+1;return 100}<br>func rhs()-&gt;u32{Trace=Trace*10+2;return 1}<br>func observe(i:u32)-&gt;(u32,u32){var b:Buffer;b[i]+=rhs();return Trace,((&amp;b) as u32 as *u32)[0]}<br>func main(){}<br></code> | main.observe([0]) -> [12,101] |
| EVAL-02.custom-read-write-bounds | <code>package main<br>type Buffer [1]u32<br>var Trace:u32<br>operator (self:*Buffer) [](i:u32)-&gt;u32{Trace=Trace*10+1;return 100}<br>func rhs()-&gt;u32{Trace=Trace*10+2;return 1}<br>func observe(i:u32)-&gt;(u32,u32){var b:Buffer;b[i]+=rhs();return Trace,((&amp;b) as u32 as *u32)[0]}<br>func main(){}<br></code> | main.observe([1]) -> bounds fault、完了済みTrace=12 |
| EVAL-02.builtin-read-custom-write | <code>package main<br>type Buffer [1]u32<br>var Trace:u32<br>operator (self:*Buffer) []=(i:u32,value:u32){Trace=Trace*10+2;(self as u32 as *u32)[i]=value+10}<br>func rhs()-&gt;u32{Trace=Trace*10+1;return 1}<br>func observe()-&gt;(u32,u32){var b:Buffer;((&amp;b) as u32 as *u32)[0]=7;b[0]+=rhs();return Trace,((&amp;b) as u32 as *u32)[0]}<br>func main(){}<br></code> | main.observe([]) -> [12,18] |

次のtemplateは同じ未注釈indexへ複数customの文脈が課される境界を表す。INDEXを行ごとに置換し、各projectを独立に検査する。

```ond
package main
type Buffer [1]u32
operator (self:*Buffer) [](i:u8)->u32{return 100}
operator (self:*Buffer) []=(i:u16,value:u32){(self as u32 as *u32)[i as u32]=value+10}
func observe()->u32{var b:Buffer;b[INDEX]+=1;return ((&b) as u32 as *u32)[0]}
func main(){}
```

| Case ID | INDEX | 操作・期待 |
| --- | --- | --- |
| OP-01.shared-index-context-reject | 0 | Core: u8/u16の文脈競合でcompile error |
| OP-01.typed-index-independent | 0 as u8 | MemoryVm: main.observe([]) -> [101]。custom getterとbuiltin setterを選択 |
