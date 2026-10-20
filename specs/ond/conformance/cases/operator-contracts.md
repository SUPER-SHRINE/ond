# Operator解決と代入の契約case
<!-- ond-spec-id: conformance/cases/operator-contracts -->

[共通case契約](./README.md#入力と期待結果の読み方)のOnd v1、manifest、fresh初期状態を使う。\
入力欄は`main.ond`全文であり、Core拒否行はsourceに対応するError診断とcompile失敗を比較し、診断文・件数の追加条件は持たない。\
MemoryVm行は受理後、引数なしでmain.observeを1回呼び返値列を比較する。\
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
