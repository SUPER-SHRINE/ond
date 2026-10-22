# static-flowの入力と期待値
<!-- ond-spec-id: conformance/cases/static-flow -->

bool条件、必須block、最内loopへのbreak/continue、全経路returnおよび終端loopの構造条件から結果を導く。

規範定義元: [FLOW-02の定義元](../../language/v1/statements.md#静的成立条件)、[FLOW-03の定義元](../../language/v1/statements.md#静的成立条件)。\
要件索引は[requirements](../requirements.md)、既存fixture宣言は[registry](../../../../crates/compiler-core/src/conformance/flow.rs)を参照する。

入力・phase・比較の共通条件は[case契約](./README.md#入力と期待結果の読み方)に従う。

| Case ID | 要件ID | 入力 | 仕様由来の期待結果 | Phase・原因位置 |
| --- | --- | --- | --- | --- |
| <code>FLOW-02.if_integer</code> | `FLOW-02` | <code>main.ond:<br>package main<br>func main() {}<br>func test() { if 1 {} }<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>FLOW-02.for_integer</code> | `FLOW-02` | <code>main.ond:<br>package main<br>func main() {}<br>func test() { for 1 {} }<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>FLOW-02.for_clause_integer</code> | `FLOW-02` | <code>main.ond:<br>package main<br>func main() {}<br>func test() { for i := 0; 1; i = i + 1 {} }<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>FLOW-02.break_outside</code> | `FLOW-02` | <code>main.ond:<br>package main<br>func main() {}<br>func test() { break; }<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>FLOW-02.continue_outside</code> | `FLOW-02` | <code>main.ond:<br>package main<br>func main() {}<br>func test() { continue; }<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>FLOW-02.if_missing_block</code> | `FLOW-02` | <code>main.ond:<br>package main<br>func main() {}<br>func test() { if true return; }<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>FLOW-02.for_missing_block</code> | `FLOW-02` | <code>main.ond:<br>package main<br>func main() {}<br>func test() { for true return; }<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>FLOW-02.else_missing_block</code> | `FLOW-02` | <code>main.ond:<br>package main<br>func main() {}<br>func test() { if true {} else return; }<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>FLOW-03.both_branches</code> | `FLOW-03` | <code>main.ond:<br>package main<br>func main() {}<br>func test(b: bool) -&gt; i32 { if b { return 1; } else { return 2; } }<br></code> | <code>accept; </code> | <code>Core; accept</code> |
| <code>FLOW-03.one_branch</code> | `FLOW-03` | <code>main.ond:<br>package main<br>func main() {}<br>func test(b: bool) -&gt; i32 { if b { return 1; } }<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>FLOW-03.nested_missing_branch</code> | `FLOW-03` | <code>main.ond:<br>package main<br>func main() {}<br>func test(b: bool) -&gt; i32 { if b { if b { return 1; } } else { return 2; } }<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>FLOW-03.nested_all_returns</code> | `FLOW-03` | <code>main.ond:<br>package main<br>func main() {}<br>func test(b: bool) -&gt; i32 { if b { if b { return 1; } else { return 2; } } else { return 3; } }<br></code> | <code>accept; </code> | <code>Core; accept</code> |
| <code>FLOW-03.infinite_loop</code> | `FLOW-03` | <code>main.ond:<br>package main<br>func main() {}<br>func test(b: bool) -&gt; i32 { for {} }<br></code> | <code>accept; </code> | <code>Core; accept</code> |
| <code>FLOW-03.infinite_clause</code> | `FLOW-03` | <code>main.ond:<br>package main<br>func main() {}<br>func test(b: bool) -&gt; i32 { for ; ; {} }<br></code> | <code>accept; </code> | <code>Core; accept</code> |
| <code>FLOW-03.loop_break</code> | `FLOW-03` | <code>main.ond:<br>package main<br>func main() {}<br>func test(b: bool) -&gt; i32 { for { break; } }<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>FLOW-03.nested_break_inner</code> | `FLOW-03` | <code>main.ond:<br>package main<br>func main() {}<br>func test(b: bool) -&gt; i32 { for { for { break; } } }<br></code> | <code>accept; </code> | <code>Core; accept</code> |
| <code>FLOW-03.nested_break_outer</code> | `FLOW-03` | <code>main.ond:<br>package main<br>func main() {}<br>func test(b: bool) -&gt; i32 { for { for { break; }; break; } }<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>FLOW-03.unreachable_break_return</code> | `FLOW-03` | <code>main.ond:<br>package main<br>func main() {}<br>func test(b: bool) -&gt; i32 { for { return 1; break; } }<br></code> | <code>accept; </code> | <code>Core; accept</code> |
| <code>FLOW-03.unreachable_break_continue</code> | `FLOW-03` | <code>main.ond:<br>package main<br>func main() {}<br>func test(b: bool) -&gt; i32 { for { continue; break; } }<br></code> | <code>accept; </code> | <code>Core; accept</code> |
| <code>FLOW-03.unreachable_break_after_nested_infinite</code> | `FLOW-03` | <code>main.ond:<br>package main<br>func main() {}<br>func test(b: bool) -&gt; i32 { for { for {}; break; } }<br></code> | <code>accept; </code> | <code>Core; accept</code> |
| <code>FLOW-03.if_false_break</code> | `FLOW-03` | <code>main.ond:<br>package main<br>func main() {}<br>func test(b: bool) -&gt; i32 { for { if false { break; } } }<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>FLOW-03.conditional_loop</code> | `FLOW-03` | <code>main.ond:<br>package main<br>func main() {}<br>func test(b: bool) -&gt; i32 { for b { return 1; } }<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>FLOW-03.literal_true_loop</code> | `FLOW-03` | <code>main.ond:<br>package main<br>func main() {}<br>func test(b: bool) -&gt; i32 { for true {} }<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>FLOW-03.conditional_loop_then_return</code> | `FLOW-03` | <code>main.ond:<br>package main<br>func main() {}<br>func test(b: bool) -&gt; i32 { for b { return 1; }; return 2; }<br></code> | <code>accept; </code> | <code>Core; accept</code> |
| <code>FLOW-03.branch_terminal_loop</code> | `FLOW-03` | <code>main.ond:<br>package main<br>func main() {}<br>func test(b: bool) -&gt; i32 { if b { return 1; } else { for {} } }<br></code> | <code>accept; </code> | <code>Core; accept</code> |
