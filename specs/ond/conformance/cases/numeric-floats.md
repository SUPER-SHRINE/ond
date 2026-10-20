# numeric-floatsの入力と期待値
<!-- ond-spec-id: conformance/cases/numeric-floats -->

[Ond binary32](../../language/v1/expressions.md#floating-point-arithmetic)の丸め、canonical NaN、signed zeroと比較規則から結果bitsまたはboolを導く。

規範定義元: [FP-01の定義元](../../language/v1/expressions.md#floating-point-arithmetic)。\
要件索引は[requirements](../requirements.md)、既存fixture宣言は[registry](../../../../crates/compiler-core/src/conformance/numeric/floats.rs)を参照する。

共通のproject入力は`ond.toml`の`[project]`に`name = "test"`を置いた実行projectである。\
generated numeric/型境界のsourceはすべて`main.ond`の内容として読み、static表は各file名を入力欄に記す。

`accept`単独は受理を期待し、値の記載がある行はその型・値まで照合する。\
構文解釈は表のParse phaseで照合し、内部AST/MIRの構造検査は外部適合判定へ含めない。\
診断の原文は照合対象を示し、規範上の失敗結果はcompile errorとして読む。

| Case ID | 要件ID | 入力 | 仕様由来の期待結果 | Phase・診断比較 |
| --- | --- | --- | --- | --- |
| <code>FP-01.add</code> | `FP-01` | <code>package main<br><br>const X = 1.5 + 2.25<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x40700000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.subtract</code> | `FP-01` | <code>package main<br><br>const X = 1.5 - 2.25<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0xbf400000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.multiply</code> | `FP-01` | <code>package main<br><br>const X = 1.5 * 2.0<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x40400000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.divide</code> | `FP-01` | <code>package main<br><br>const X = 1.0 / 3.0<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x3eaaaaab</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.unary_plus</code> | `FP-01` | <code>package main<br><br>const X = +1.5<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x3fc00000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.unary_minus</code> | `FP-01` | <code>package main<br><br>const X = -1.5<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0xbfc00000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.add_tie_even</code> | `FP-01` | <code>package main<br><br>const X = 1.0 + 0x1p-24<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x3f800000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.add_tie_odd</code> | `FP-01` | <code>package main<br><br>const X = 0x1.000002p0 + 0x1p-24<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x3f800002</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.sub_tie_even</code> | `FP-01` | <code>package main<br><br>const X = 0x1.000002p0 - 0x1p-24<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x3f800000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.sub_tie_odd</code> | `FP-01` | <code>package main<br><br>const X = 0x1.000004p0 - 0x1p-24<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x3f800002</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.mul_underflow_even</code> | `FP-01` | <code>package main<br><br>const X = 0x1p-149 * 0.5<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x00000000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.mul_underflow_odd</code> | `FP-01` | <code>package main<br><br>const X = 0x1.8p-148 * 0.5<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x00000002</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.div_underflow_even</code> | `FP-01` | <code>package main<br><br>const X = 0x1p-149 / 2.0<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x00000000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.div_underflow_odd</code> | `FP-01` | <code>package main<br><br>const X = 0x1.8p-148 / 2.0<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x00000002</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.add_subnormal</code> | `FP-01` | <code>package main<br><br>const X = 0x1p-149 + 0x1p-149<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x00000002</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.sub_subnormal</code> | `FP-01` | <code>package main<br><br>const X = 0x1p-126 - 0x1.fffffcp-127<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x00000001</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.negative_underflow</code> | `FP-01` | <code>package main<br><br>const X = -0x1p-149 / 2.0<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x80000000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.positive_zero</code> | `FP-01` | <code>package main<br><br>const X = -0.0 + 0.0<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x00000000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.negative_zero_add</code> | `FP-01` | <code>package main<br><br>const X = -0.0 + -0.0<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x80000000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.negative_zero_sub</code> | `FP-01` | <code>package main<br><br>const X = -0.0 - 0.0<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x80000000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.negative_zero_mul</code> | `FP-01` | <code>package main<br><br>const X = 0.0 * -1.0<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x80000000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.negative_zero_div</code> | `FP-01` | <code>package main<br><br>const X = 0.0 / -1.0<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x80000000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.overflow_add</code> | `FP-01` | <code>package main<br><br>const X = 0x1.fffffep127 + 0x1.fffffep127<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x7f800000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.overflow_sub</code> | `FP-01` | <code>package main<br><br>const X = -0x1.fffffep127 - 0x1.fffffep127<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0xff800000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.overflow_mul</code> | `FP-01` | <code>package main<br><br>const X = 0x1.fffffep127 * 2.0<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x7f800000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.overflow_div</code> | `FP-01` | <code>package main<br><br>const X = 0x1.fffffep127 / 0.5<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x7f800000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.positive_inf</code> | `FP-01` | <code>package main<br><br>const X = 1.0 / 0.0<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x7f800000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.negative_inf</code> | `FP-01` | <code>package main<br><br>const X = -1.0 / 0.0<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0xff800000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.nan_div</code> | `FP-01` | <code>package main<br><br>const X = 0.0 / 0.0<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x7fc00000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.nan_mul</code> | `FP-01` | <code>package main<br><br>const X = (1.0 / 0.0) * 0.0<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x7fc00000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.nan_sub</code> | `FP-01` | <code>package main<br><br>const X = (1.0 / 0.0) - (1.0 / 0.0)<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x7fc00000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.nan_add</code> | `FP-01` | <code>package main<br><br>const X = (1.0 / 0.0) + (-1.0 / 0.0)<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x7fc00000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.nan_neg</code> | `FP-01` | <code>package main<br><br>const X = -(0.0 / 0.0)<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x7fc00000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.nan_left.eq</code> | `FP-01` | <code>package main<br>const A: f32 = 0.0 / 0.0<br>const B: f32 = 1.0<br>const X = A == B<br>func main() {}<br></code> | <code>accept; X: bool = false</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.nan_left.ne</code> | `FP-01` | <code>package main<br>const A: f32 = 0.0 / 0.0<br>const B: f32 = 1.0<br>const X = A != B<br>func main() {}<br></code> | <code>accept; X: bool = true</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.nan_left.lt</code> | `FP-01` | <code>package main<br>const A: f32 = 0.0 / 0.0<br>const B: f32 = 1.0<br>const X = A &lt; B<br>func main() {}<br></code> | <code>accept; X: bool = false</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.nan_left.le</code> | `FP-01` | <code>package main<br>const A: f32 = 0.0 / 0.0<br>const B: f32 = 1.0<br>const X = A &lt;= B<br>func main() {}<br></code> | <code>accept; X: bool = false</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.nan_left.gt</code> | `FP-01` | <code>package main<br>const A: f32 = 0.0 / 0.0<br>const B: f32 = 1.0<br>const X = A &gt; B<br>func main() {}<br></code> | <code>accept; X: bool = false</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.nan_left.ge</code> | `FP-01` | <code>package main<br>const A: f32 = 0.0 / 0.0<br>const B: f32 = 1.0<br>const X = A &gt;= B<br>func main() {}<br></code> | <code>accept; X: bool = false</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.nan_right.eq</code> | `FP-01` | <code>package main<br>const A: f32 = 1.0<br>const B: f32 = 0.0 / 0.0<br>const X = A == B<br>func main() {}<br></code> | <code>accept; X: bool = false</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.nan_right.ne</code> | `FP-01` | <code>package main<br>const A: f32 = 1.0<br>const B: f32 = 0.0 / 0.0<br>const X = A != B<br>func main() {}<br></code> | <code>accept; X: bool = true</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.nan_right.lt</code> | `FP-01` | <code>package main<br>const A: f32 = 1.0<br>const B: f32 = 0.0 / 0.0<br>const X = A &lt; B<br>func main() {}<br></code> | <code>accept; X: bool = false</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.nan_right.le</code> | `FP-01` | <code>package main<br>const A: f32 = 1.0<br>const B: f32 = 0.0 / 0.0<br>const X = A &lt;= B<br>func main() {}<br></code> | <code>accept; X: bool = false</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.nan_right.gt</code> | `FP-01` | <code>package main<br>const A: f32 = 1.0<br>const B: f32 = 0.0 / 0.0<br>const X = A &gt; B<br>func main() {}<br></code> | <code>accept; X: bool = false</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.nan_right.ge</code> | `FP-01` | <code>package main<br>const A: f32 = 1.0<br>const B: f32 = 0.0 / 0.0<br>const X = A &gt;= B<br>func main() {}<br></code> | <code>accept; X: bool = false</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.nan_both.eq</code> | `FP-01` | <code>package main<br>const A: f32 = 0.0 / 0.0<br>const B: f32 = 0.0 / 0.0<br>const X = A == B<br>func main() {}<br></code> | <code>accept; X: bool = false</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.nan_both.ne</code> | `FP-01` | <code>package main<br>const A: f32 = 0.0 / 0.0<br>const B: f32 = 0.0 / 0.0<br>const X = A != B<br>func main() {}<br></code> | <code>accept; X: bool = true</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.nan_both.lt</code> | `FP-01` | <code>package main<br>const A: f32 = 0.0 / 0.0<br>const B: f32 = 0.0 / 0.0<br>const X = A &lt; B<br>func main() {}<br></code> | <code>accept; X: bool = false</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.nan_both.le</code> | `FP-01` | <code>package main<br>const A: f32 = 0.0 / 0.0<br>const B: f32 = 0.0 / 0.0<br>const X = A &lt;= B<br>func main() {}<br></code> | <code>accept; X: bool = false</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.nan_both.gt</code> | `FP-01` | <code>package main<br>const A: f32 = 0.0 / 0.0<br>const B: f32 = 0.0 / 0.0<br>const X = A &gt; B<br>func main() {}<br></code> | <code>accept; X: bool = false</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.nan_both.ge</code> | `FP-01` | <code>package main<br>const A: f32 = 0.0 / 0.0<br>const B: f32 = 0.0 / 0.0<br>const X = A &gt;= B<br>func main() {}<br></code> | <code>accept; X: bool = false</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.signed_zero.eq</code> | `FP-01` | <code>package main<br>const A: f32 = -0.0<br>const B: f32 = 0.0<br>const X = A == B<br>func main() {}<br></code> | <code>accept; X: bool = true</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.signed_zero.ne</code> | `FP-01` | <code>package main<br>const A: f32 = -0.0<br>const B: f32 = 0.0<br>const X = A != B<br>func main() {}<br></code> | <code>accept; X: bool = false</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.signed_zero.lt</code> | `FP-01` | <code>package main<br>const A: f32 = -0.0<br>const B: f32 = 0.0<br>const X = A &lt; B<br>func main() {}<br></code> | <code>accept; X: bool = false</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.signed_zero.le</code> | `FP-01` | <code>package main<br>const A: f32 = -0.0<br>const B: f32 = 0.0<br>const X = A &lt;= B<br>func main() {}<br></code> | <code>accept; X: bool = true</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.signed_zero.gt</code> | `FP-01` | <code>package main<br>const A: f32 = -0.0<br>const B: f32 = 0.0<br>const X = A &gt; B<br>func main() {}<br></code> | <code>accept; X: bool = false</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.signed_zero.ge</code> | `FP-01` | <code>package main<br>const A: f32 = -0.0<br>const B: f32 = 0.0<br>const X = A &gt;= B<br>func main() {}<br></code> | <code>accept; X: bool = true</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.ordered.eq</code> | `FP-01` | <code>package main<br>const A: f32 = -1.5<br>const B: f32 = 1.5<br>const X = A == B<br>func main() {}<br></code> | <code>accept; X: bool = false</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.ordered.ne</code> | `FP-01` | <code>package main<br>const A: f32 = -1.5<br>const B: f32 = 1.5<br>const X = A != B<br>func main() {}<br></code> | <code>accept; X: bool = true</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.ordered.lt</code> | `FP-01` | <code>package main<br>const A: f32 = -1.5<br>const B: f32 = 1.5<br>const X = A &lt; B<br>func main() {}<br></code> | <code>accept; X: bool = true</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.ordered.le</code> | `FP-01` | <code>package main<br>const A: f32 = -1.5<br>const B: f32 = 1.5<br>const X = A &lt;= B<br>func main() {}<br></code> | <code>accept; X: bool = true</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.ordered.gt</code> | `FP-01` | <code>package main<br>const A: f32 = -1.5<br>const B: f32 = 1.5<br>const X = A &gt; B<br>func main() {}<br></code> | <code>accept; X: bool = false</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.ordered.ge</code> | `FP-01` | <code>package main<br>const A: f32 = -1.5<br>const B: f32 = 1.5<br>const X = A &gt;= B<br>func main() {}<br></code> | <code>accept; X: bool = false</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.infinities.eq</code> | `FP-01` | <code>package main<br>const A: f32 = -1.0 / 0.0<br>const B: f32 = 1.0 / 0.0<br>const X = A == B<br>func main() {}<br></code> | <code>accept; X: bool = false</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.infinities.ne</code> | `FP-01` | <code>package main<br>const A: f32 = -1.0 / 0.0<br>const B: f32 = 1.0 / 0.0<br>const X = A != B<br>func main() {}<br></code> | <code>accept; X: bool = true</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.infinities.lt</code> | `FP-01` | <code>package main<br>const A: f32 = -1.0 / 0.0<br>const B: f32 = 1.0 / 0.0<br>const X = A &lt; B<br>func main() {}<br></code> | <code>accept; X: bool = true</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.infinities.le</code> | `FP-01` | <code>package main<br>const A: f32 = -1.0 / 0.0<br>const B: f32 = 1.0 / 0.0<br>const X = A &lt;= B<br>func main() {}<br></code> | <code>accept; X: bool = true</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.infinities.gt</code> | `FP-01` | <code>package main<br>const A: f32 = -1.0 / 0.0<br>const B: f32 = 1.0 / 0.0<br>const X = A &gt; B<br>func main() {}<br></code> | <code>accept; X: bool = false</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-01.infinities.ge</code> | `FP-01` | <code>package main<br>const A: f32 = -1.0 / 0.0<br>const B: f32 = 1.0 / 0.0<br>const X = A &gt;= B<br>func main() {}<br></code> | <code>accept; X: bool = false</code> | <code>Core; accept; diagnostics=0</code> |
