# Layoutと独自operatorの入力と期待値
<!-- ond-spec-id: conformance/cases/layout-operators -->

共通の版、manifest、phase、初期状態と比較方法は[case契約](./README.md#入力と期待結果の読み方)に従う。\
入力欄は各caseの`main.ond`全文であり、実行は新しい状態で`main.observe`を引数なしで1回呼ぶ。

規範定義元は[layout構文](../../language/v1/builtins.md#組み込み構文)、[Kagura data layout](../../targets/kagura-v1/machine.md#5-data-layout)、[operator len](../../language/v1/builtins.md#41-配列に対する-len)、[独自operator](../../language/v1/expressions.md#user-defined-operator)とする。

| Case ID | 要件ID | 入力 | 操作・期待結果 |
| --- | --- | --- | --- |
| `BI-04.layout-padded-struct` | `BI-04` | <code>package main<br>type S struct { a: u8; b: u32; c: u8 }<br>func observe() -&gt; (u32, u32) { return sizeof(S), alignof(S) }<br>func main() {}<br></code> | MemoryVm; 受理; `main.observe([]) -> [12, 4]` |
| `BI-04.layout-zero-array` | `BI-04` | <code>package main<br>func observe() -&gt; (u32, u32) { return sizeof([0]u32), alignof([0]u32) }<br>func main() {}<br></code> | MemoryVm; 受理; `main.observe([]) -> [0, 4]` |
| `BI-02.operator-len-struct` | `BI-02` | <code>package main<br>type Buffer struct { count: u32 }<br>operator (self: Buffer) len() -&gt; u32 { return self.count }<br>func observe() -&gt; u32 { value := Buffer{count: 7}; return len(value) }<br>func main() {}<br></code> | MemoryVm; 受理; `main.observe([]) -> [7]` |
| `OP-01.operator-index-struct` | `OP-01` | <code>package main<br>type Table struct { values: [2]u32 }<br>operator (self: Table) [](index: u32) -&gt; u32 { return self.values[index] }<br>func observe() -&gt; u32 { value := Table{values: [2]u32{11, 42}}; return value[1 as u32] }<br>func main() {}<br></code> | MemoryVm; 受理; `main.observe([]) -> [42]` |
