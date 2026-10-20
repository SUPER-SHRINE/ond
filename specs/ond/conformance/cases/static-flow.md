# static-flowの入力と期待値
<!-- ond-spec-id: conformance/cases/static-flow -->

bool条件、必須block、最内loopへのbreak/continue、全経路returnおよび終端loopの構造条件から結果を導く。

規範定義元: [FLOW-02の定義元](../../language/v1/statements.md#静的成立条件)、[FLOW-03の定義元](../../language/v1/statements.md#静的成立条件)。\
要件索引は[requirements](../requirements.md)、既存fixture宣言は[registry](../../../../crates/compiler-core/src/conformance/flow.rs)を参照する。

共通のproject入力は`ond.toml`の`[project]`に`name = "test"`を置いた実行projectである。\
generated numeric/型境界のsourceはすべて`main.ond`の内容として読み、static表は各file名を入力欄に記す。

`accept`単独は受理を期待し、値の記載がある行はその型・値まで照合する。\
構文解釈は表のParse phaseで照合し、内部AST/MIRの構造検査は外部適合判定へ含めない。\
診断の原文は照合対象を示し、規範上の失敗結果はcompile errorとして読む。

| Case ID | 要件ID | 入力 | 仕様由来の期待結果 | Phase・診断比較 |
| --- | --- | --- | --- | --- |
| <code>FLOW-02.if_integer</code> | `FLOW-02` | <code>main.ond:<br>package main<br>func main() {}<br>func test() { if 1 {} }<br></code> | <code>compile error</code> | <code>Core; reject; diagnostics=1; Error@main.ond:[45,46) contains=condition must have type bool</code> |
| <code>FLOW-02.for_integer</code> | `FLOW-02` | <code>main.ond:<br>package main<br>func main() {}<br>func test() { for 1 {} }<br></code> | <code>compile error</code> | <code>Core; reject; diagnostics=1; Error@main.ond:[46,47) contains=condition must have type bool</code> |
| <code>FLOW-02.for_clause_integer</code> | `FLOW-02` | <code>main.ond:<br>package main<br>func main() {}<br>func test() { for i := 0; 1; i = i + 1 {} }<br></code> | <code>compile error</code> | <code>Core; reject; diagnostics=1; Error@main.ond:[54,55) contains=condition must have type bool</code> |
| <code>FLOW-02.break_outside</code> | `FLOW-02` | <code>main.ond:<br>package main<br>func main() {}<br>func test() { break; }<br></code> | <code>compile error</code> | <code>Core; reject; diagnostics=1; Error@main.ond:[42,47) contains=break/continue requires an enclosing loop</code> |
| <code>FLOW-02.continue_outside</code> | `FLOW-02` | <code>main.ond:<br>package main<br>func main() {}<br>func test() { continue; }<br></code> | <code>compile error</code> | <code>Core; reject; diagnostics=1; Error@main.ond:[42,50) contains=break/continue requires an enclosing loop</code> |
| <code>FLOW-02.if_missing_block</code> | `FLOW-02` | <code>main.ond:<br>package main<br>func main() {}<br>func test() { if true return; }<br></code> | <code>compile error</code> | <code>Core; reject; diagnostics=1; Error@main.ond:[50,56) contains=expected `{`</code> |
| <code>FLOW-02.for_missing_block</code> | `FLOW-02` | <code>main.ond:<br>package main<br>func main() {}<br>func test() { for true return; }<br></code> | <code>compile error</code> | <code>Core; reject; diagnostics=1; Error@main.ond:[51,57) contains=expected `{`</code> |
| <code>FLOW-02.else_missing_block</code> | `FLOW-02` | <code>main.ond:<br>package main<br>func main() {}<br>func test() { if true {} else return; }<br></code> | <code>compile error</code> | <code>Core; reject; diagnostics=1; Error@main.ond:[58,64) contains=expected `if` or block after `else`</code> |
| <code>FLOW-03.both_branches</code> | `FLOW-03` | <code>main.ond:<br>package main<br>func main() {}<br>func test(b: bool) -&gt; i32 { if b { return 1; } else { return 2; } }<br></code> | <code>accept; </code> | <code>Core; accept; diagnostics=0</code> |
| <code>FLOW-03.one_branch</code> | `FLOW-03` | <code>main.ond:<br>package main<br>func main() {}<br>func test(b: bool) -&gt; i32 { if b { return 1; } }<br></code> | <code>compile error</code> | <code>Core; reject; diagnostics=1; Error@main.ond:[54,76) contains=missing return in function `test`</code> |
| <code>FLOW-03.nested_missing_branch</code> | `FLOW-03` | <code>main.ond:<br>package main<br>func main() {}<br>func test(b: bool) -&gt; i32 { if b { if b { return 1; } } else { return 2; } }<br></code> | <code>compile error</code> | <code>Core; reject; diagnostics=1; Error@main.ond:[54,104) contains=missing return in function `test`</code> |
| <code>FLOW-03.nested_all_returns</code> | `FLOW-03` | <code>main.ond:<br>package main<br>func main() {}<br>func test(b: bool) -&gt; i32 { if b { if b { return 1; } else { return 2; } } else { return 3; } }<br></code> | <code>accept; </code> | <code>Core; accept; diagnostics=0</code> |
| <code>FLOW-03.infinite_loop</code> | `FLOW-03` | <code>main.ond:<br>package main<br>func main() {}<br>func test(b: bool) -&gt; i32 { for {} }<br></code> | <code>accept; </code> | <code>Core; accept; diagnostics=0</code> |
| <code>FLOW-03.infinite_clause</code> | `FLOW-03` | <code>main.ond:<br>package main<br>func main() {}<br>func test(b: bool) -&gt; i32 { for ; ; {} }<br></code> | <code>accept; </code> | <code>Core; accept; diagnostics=0</code> |
| <code>FLOW-03.loop_break</code> | `FLOW-03` | <code>main.ond:<br>package main<br>func main() {}<br>func test(b: bool) -&gt; i32 { for { break; } }<br></code> | <code>compile error</code> | <code>Core; reject; diagnostics=1; Error@main.ond:[54,72) contains=missing return in function `test`</code> |
| <code>FLOW-03.nested_break_inner</code> | `FLOW-03` | <code>main.ond:<br>package main<br>func main() {}<br>func test(b: bool) -&gt; i32 { for { for { break; } } }<br></code> | <code>accept; </code> | <code>Core; accept; diagnostics=0</code> |
| <code>FLOW-03.nested_break_outer</code> | `FLOW-03` | <code>main.ond:<br>package main<br>func main() {}<br>func test(b: bool) -&gt; i32 { for { for { break; }; break; } }<br></code> | <code>compile error</code> | <code>Core; reject; diagnostics=1; Error@main.ond:[54,88) contains=missing return in function `test`</code> |
| <code>FLOW-03.unreachable_break_return</code> | `FLOW-03` | <code>main.ond:<br>package main<br>func main() {}<br>func test(b: bool) -&gt; i32 { for { return 1; break; } }<br></code> | <code>accept; </code> | <code>Core; accept; diagnostics=0</code> |
| <code>FLOW-03.unreachable_break_continue</code> | `FLOW-03` | <code>main.ond:<br>package main<br>func main() {}<br>func test(b: bool) -&gt; i32 { for { continue; break; } }<br></code> | <code>accept; </code> | <code>Core; accept; diagnostics=0</code> |
| <code>FLOW-03.unreachable_break_after_nested_infinite</code> | `FLOW-03` | <code>main.ond:<br>package main<br>func main() {}<br>func test(b: bool) -&gt; i32 { for { for {}; break; } }<br></code> | <code>accept; </code> | <code>Core; accept; diagnostics=0</code> |
| <code>FLOW-03.if_false_break</code> | `FLOW-03` | <code>main.ond:<br>package main<br>func main() {}<br>func test(b: bool) -&gt; i32 { for { if false { break; } } }<br></code> | <code>compile error</code> | <code>Core; reject; diagnostics=1; Error@main.ond:[54,85) contains=missing return in function `test`</code> |
| <code>FLOW-03.conditional_loop</code> | `FLOW-03` | <code>main.ond:<br>package main<br>func main() {}<br>func test(b: bool) -&gt; i32 { for b { return 1; } }<br></code> | <code>compile error</code> | <code>Core; reject; diagnostics=1; Error@main.ond:[54,77) contains=missing return in function `test`</code> |
| <code>FLOW-03.literal_true_loop</code> | `FLOW-03` | <code>main.ond:<br>package main<br>func main() {}<br>func test(b: bool) -&gt; i32 { for true {} }<br></code> | <code>compile error</code> | <code>Core; reject; diagnostics=1; Error@main.ond:[54,69) contains=missing return in function `test`</code> |
| <code>FLOW-03.conditional_loop_then_return</code> | `FLOW-03` | <code>main.ond:<br>package main<br>func main() {}<br>func test(b: bool) -&gt; i32 { for b { return 1; }; return 2; }<br></code> | <code>accept; </code> | <code>Core; accept; diagnostics=0</code> |
| <code>FLOW-03.branch_terminal_loop</code> | `FLOW-03` | <code>main.ond:<br>package main<br>func main() {}<br>func test(b: bool) -&gt; i32 { if b { return 1; } else { for {} } }<br></code> | <code>accept; </code> | <code>Core; accept; diagnostics=0</code> |
