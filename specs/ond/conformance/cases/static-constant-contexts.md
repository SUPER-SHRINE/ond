# static-constant-contextsの入力と期待値
<!-- ond-spec-id: conformance/cases/static-constant-contexts -->

短絡表の値評価経路、両辺の名前・型検査、全式const適格性、値要求境界と明示castの範囲条件から受理・値・compile errorを導く。

規範定義元: [EVAL-01の定義元](../../language/v1/expressions.md#式と評価順)、[SYN-12の定義元](../../language/v1/lexical.md#文法の定義元)。\
要件索引は[requirements](../requirements.md)、既存fixture宣言は[registry](../../../../crates/compiler-core/src/conformance/constant_contexts.rs)を参照する。

共通のproject入力は`ond.toml`の`[project]`に`name = "test"`を置いた実行projectである。\
generated numeric/型境界のsourceはすべて`main.ond`の内容として読み、static表は各file名を入力欄に記す。

以下は移行時の既存case宣言に含まれる入力と仕様由来Expectedデータを記録したものであり、compilerを実行して得た結果の転記ではない。\
`accept`単独は受理を期待し、値の記載がある行はその型・値まで照合する。\
`implementation AST/MIR`は外部の受理・値期待と区分した実装回帰検査である。\
診断の原文は照合対象を示し、規範上の失敗結果はcompile errorとして読む。

| Case ID | 要件ID | 入力 | 仕様由来の期待結果と内部検査 |
| --- | --- | --- | --- |
| <code>SYN-12.q03-short_and</code> | `SYN-12`, `EVAL-01` | <code>main.ond:<br>package main<br>const X = false &amp;&amp; (1 / 0 == 0)<br>func f() -&gt; bool { return true; }<br>func array(x: u8) -&gt; [2]u8 { return [2]u8{}; }<br>func main() {}<br></code> | <code>accept; X: bool = false</code> |
| <code>SYN-12.q03-short_or</code> | `SYN-12`, `EVAL-01` | <code>main.ond:<br>package main<br>const X = true &#124;&#124; (1 / 0 == 0)<br>func f() -&gt; bool { return true; }<br>func array(x: u8) -&gt; [2]u8 { return [2]u8{}; }<br>func main() {}<br></code> | <code>accept; X: bool = true</code> |
| <code>SYN-12.q03-array_call_evaluates_argument</code> | `SYN-12`, `EVAL-01` | <code>main.ond:<br>package main<br>const X = len(array(1 / 0))<br>func f() -&gt; bool { return true; }<br>func array(x: u8) -&gt; [2]u8 { return [2]u8{}; }<br>func main() {}<br></code> | <code>compile error; division by zero</code> |
| <code>SYN-12.q03-array_literal_evaluates_element</code> | `SYN-12`, `EVAL-01` | <code>main.ond:<br>package main<br>const X = len([2]i32{1 / 0, 0})<br>func f() -&gt; bool { return true; }<br>func array(x: u8) -&gt; [2]u8 { return [2]u8{}; }<br>func main() {}<br></code> | <code>compile error; division by zero</code> |
| <code>SYN-12.q03-array_len_evaluates_cast</code> | `SYN-12`, `EVAL-01` | <code>main.ond:<br>package main<br>const X = len(array((250 + 6) as u8))<br>func f() -&gt; bool { return true; }<br>func array(x: u8) -&gt; [2]u8 { return [2]u8{}; }<br>func main() {}<br></code> | <code>compile error; constant cast is out of range</code> |
| <code>SYN-12.q03-utf8_length</code> | `SYN-12`, `EVAL-01` | <code>main.ond:<br>package main<br>const X = len(&quot;音&quot;)<br>func f() -&gt; bool { return true; }<br>func array(x: u8) -&gt; [2]u8 { return [2]u8{}; }<br>func main() {}<br></code> | <code>accept; X: u32 = integer bits 0x0000000000000003</code> |
| <code>SYN-12.q03-short_valid_cast</code> | `SYN-12`, `EVAL-01` | <code>main.ond:<br>package main<br>const X = false &amp;&amp; ((250 + 5) as u8 == 0)<br>func f() -&gt; bool { return true; }<br>func array(x: u8) -&gt; [2]u8 { return [2]u8{}; }<br>func main() {}<br></code> | <code>accept; X: bool = false</code> |
| <code>SYN-12.q03-ordinary_call</code> | `SYN-12`, `EVAL-01` | <code>main.ond:<br>package main<br>const X = f()<br>func f() -&gt; bool { return true; }<br>func array(x: u8) -&gt; [2]u8 { return [2]u8{}; }<br>func main() {}<br></code> | <code>compile error; constant initializer requires a compile-time value</code> |
| <code>SYN-12.q03-skipped_call_and</code> | `SYN-12`, `EVAL-01` | <code>main.ond:<br>package main<br>const X = false &amp;&amp; f()<br>func f() -&gt; bool { return true; }<br>func array(x: u8) -&gt; [2]u8 { return [2]u8{}; }<br>func main() {}<br></code> | <code>compile error; constant initializer requires a compile-time value</code> |
| <code>SYN-12.q03-skipped_call_or</code> | `SYN-12`, `EVAL-01` | <code>main.ond:<br>package main<br>const X = true &#124;&#124; f()<br>func f() -&gt; bool { return true; }<br>func array(x: u8) -&gt; [2]u8 { return [2]u8{}; }<br>func main() {}<br></code> | <code>compile error; constant initializer requires a compile-time value</code> |
| <code>SYN-12.q03-skipped_unknown</code> | `SYN-12`, `EVAL-01` | <code>main.ond:<br>package main<br>const X = false &amp;&amp; missing<br>func f() -&gt; bool { return true; }<br>func array(x: u8) -&gt; [2]u8 { return [2]u8{}; }<br>func main() {}<br></code> | <code>compile error; unknown</code> |
| <code>SYN-12.q03-skipped_type_error</code> | `SYN-12`, `EVAL-01` | <code>main.ond:<br>package main<br>const X = false &amp;&amp; 1<br>func f() -&gt; bool { return true; }<br>func array(x: u8) -&gt; [2]u8 { return [2]u8{}; }<br>func main() {}<br></code> | <code>compile error; integer literal requires an integer type</code> |
| <code>SYN-12.q03-len_argument_type_error</code> | `SYN-12`, `EVAL-01` | <code>main.ond:<br>package main<br>const X = len(array(true))<br>func f() -&gt; bool { return true; }<br>func array(x: u8) -&gt; [2]u8 { return [2]u8{}; }<br>func main() {}<br></code> | <code>compile error; boolean literal</code> |
| <code>SYN-12.q03-len_unknown</code> | `SYN-12`, `EVAL-01` | <code>main.ond:<br>package main<br>const X = len(array(missing))<br>func f() -&gt; bool { return true; }<br>func array(x: u8) -&gt; [2]u8 { return [2]u8{}; }<br>func main() {}<br></code> | <code>compile error; unknown</code> |
| <code>SYN-12.q03-evaluated_cast_range</code> | `SYN-12`, `EVAL-01` | <code>main.ond:<br>package main<br>const X = (250 + 6) as u8<br>func f() -&gt; bool { return true; }<br>func array(x: u8) -&gt; [2]u8 { return [2]u8{}; }<br>func main() {}<br></code> | <code>compile error; out of range</code> |
| <code>SYN-12.q03-len_cast_type_error</code> | `SYN-12`, `EVAL-01` | <code>main.ond:<br>package main<br>const X = len(array(true as u8))<br>func f() -&gt; bool { return true; }<br>func array(x: u8) -&gt; [2]u8 { return [2]u8{}; }<br>func main() {}<br></code> | <code>compile error; conversion</code> |
| <code>SYN-12.q03-short_cast_range_and</code> | `SYN-12`, `EVAL-01` | <code>main.ond:<br>package main<br>const X = false &amp;&amp; ((250 + 6) as u8 == 0)<br>func f(x: u8) -&gt; bool { return true; }<br>func main() {}<br></code> | <code>accept; X: bool = false</code> |
| <code>SYN-12.q03-short_cast_range_or</code> | `SYN-12`, `EVAL-01` | <code>main.ond:<br>package main<br>const X = true &#124;&#124; ((250 + 6) as u8 == 0)<br>func f(x: u8) -&gt; bool { return true; }<br>func main() {}<br></code> | <code>accept; X: bool = true</code> |
| <code>SYN-12.q03-computed_short_cast</code> | `SYN-12`, `EVAL-01` | <code>main.ond:<br>package main<br>const X = (1 &gt; 2) &amp;&amp; ((250 + 6) as u8 == 0)<br>func f(x: u8) -&gt; bool { return true; }<br>func main() {}<br></code> | <code>accept; X: bool = false</code> |
| <code>SYN-12.q03-nested_short_cast</code> | `SYN-12`, `EVAL-01` | <code>main.ond:<br>package main<br>const X = false &amp;&amp; (true &amp;&amp; ((250 + 6) as u8 == 0))<br>func f(x: u8) -&gt; bool { return true; }<br>func main() {}<br></code> | <code>accept; X: bool = false</code> |
| <code>SYN-12.q03-evaluated_short_cast</code> | `SYN-12`, `EVAL-01` | <code>main.ond:<br>package main<br>const X = true &amp;&amp; ((250 + 6) as u8 == 0)<br>func f(x: u8) -&gt; bool { return true; }<br>func main() {}<br></code> | <code>compile error; constant cast is out of range</code> |
| <code>SYN-12.q03-evaluated_or_cast</code> | `SYN-12`, `EVAL-01` | <code>main.ond:<br>package main<br>const X = false &#124;&#124; ((250 + 6) as u8 == 0)<br>func f(x: u8) -&gt; bool { return true; }<br>func main() {}<br></code> | <code>compile error; constant cast is out of range</code> |
| <code>SYN-12.q03-skipped_cast_type_error</code> | `SYN-12`, `EVAL-01` | <code>main.ond:<br>package main<br>const X = false &amp;&amp; (true as u8 == 0)<br>func f(x: u8) -&gt; bool { return true; }<br>func main() {}<br></code> | <code>compile error; invalid explicit type conversion</code> |
| <code>SYN-12.q03-skipped_cast_unknown</code> | `SYN-12`, `EVAL-01` | <code>main.ond:<br>package main<br>const X = false &amp;&amp; (missing as u8 == 0)<br>func f(x: u8) -&gt; bool { return true; }<br>func main() {}<br></code> | <code>compile error; unknown identifier</code> |
| <code>SYN-12.q03-skipped_cast_call_forbidden</code> | `SYN-12`, `EVAL-01` | <code>main.ond:<br>package main<br>const X = false &amp;&amp; f((250 + 6) as u8)<br>func f(x: u8) -&gt; bool { return true; }<br>func main() {}<br></code> | <code>compile error; constant initializer requires a compile-time value</code> |
