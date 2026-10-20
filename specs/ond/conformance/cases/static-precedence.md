# static-precedenceの入力と期待値
<!-- ond-spec-id: conformance/cases/static-precedence -->

優先順位7段階、同順位左結合、postfix変換で式を解釈して定数値を導く。\
Parse caseは本書の結合構造、Core caseは定数値を観測する。

規範定義元: [SYN-07の定義元](../../language/v1/expressions.md#演算子の優先順位)。\
要件索引は[requirements](../requirements.md)、既存fixture宣言は[registry](../../../../crates/compiler-core/src/conformance/precedence.rs)を参照する。

共通のproject入力は`ond.toml`の`[project]`に`name = "test"`を置いた実行projectである。\
generated numeric/型境界のsourceはすべて`main.ond`の内容として読み、static表は各file名を入力欄に記す。

`accept`単独は受理を期待し、値の記載がある行はその型・値まで照合する。\
構文解釈は表のParse phaseで照合し、内部AST/MIRの構造検査は外部適合判定へ含めない。\
診断の原文は照合対象を示し、規範上の失敗結果はcompile errorとして読む。

| Case ID | 要件ID | 入力 | 仕様由来の期待結果 | Phase・診断比較 |
| --- | --- | --- | --- | --- |
| <code>SYN-07.precedence-associativity.ast</code> | `SYN-07` | <code>main.ond:<br>package main<br>const E0 = true &#124;&#124; false &amp;&amp; false<br>const E1 = true &amp;&amp; 1 == 1<br>const E2 = 1 == 2 + 3<br>const E3 = 1 + 2 * 3<br>const E4 = 2 * -3<br>const E5 = -1 as i32<br>const E6 = 1 + 2 as i32<br>const E7 = f()[0].X as i32 as u32<br>const E8 = true &#124;&#124; false &#124;&#124; false<br>const E9 = true &amp;&amp; false &amp;&amp; false<br>const E10 = 1 &lt; 2 == true<br>const E11 = 8 - 3 + 2 &#124; 1 ^ 4<br>const E12 = 32 / 2 * 3 % 5 &lt;&lt; 1 &gt;&gt; 1 &amp; 7 &amp;^ 2<br>const E13 = !-^x<br>const E14 = (1 + 2) * 3<br></code> | <code>accept; 構文解釈: seven levels, left associativity and postfix chain</code> | <code>Parse; accept; diagnostics=0</code> |
| <code>SYN-07.precedence-values.core</code> | `SYN-07` | <code>main.ond:<br>package main<br>const E0 = true &#124;&#124; false &amp;&amp; false<br>const E1 = true &amp;&amp; 1 == 1<br>const E2 = 1 == 2 + 3<br>const E3 = 1 + 2 * 3<br>const E4 = 2 * -3<br>const E5 = -1 as i32<br>const E6 = 1 + 2 as i32<br>const E7 = true &#124;&#124; false &#124;&#124; false<br>const E8 = true &amp;&amp; false &amp;&amp; false<br>const E9 = 1 &lt; 2 == true<br>const E10 = 8 - 3 + 2 &#124; 1 ^ 4<br>const E11 = 32 / 2 * 3 % 5 &lt;&lt; 1 &gt;&gt; 1 &amp; 7 &amp;^ 2<br>const E12 = (1 + 2) * 3<br>func main() {}<br></code> | <code>accept; E0: bool = true; E1: bool = true; E2: bool = false; E3: i32 = integer bits 0x0000000000000007; E4: i32 = integer bits 0xfffffffffffffffa; E5: i32 = integer bits 0xffffffffffffffff; E6: i32 = integer bits 0x0000000000000003; E7: bool = true; E8: bool = false; E9: bool = true; E10: i32 = integer bits 0x0000000000000003; E11: i32 = integer bits 0x0000000000000001; E12: i32 = integer bits 0x0000000000000009</code> | <code>Core; accept; diagnostics=0</code> |

## Parse caseの結合期待
<!-- ond-spec-id: conformance/cases/precedence-parse-grouping -->

`SYN-07.precedence-associativity.ast`の受理は構文解析の成功だけを表す。\
名前`f`と`x`の解決、const適格性、entryの存在はこのcaseの操作に含めない。\
各宣言の式は次の括弧付き構造と同じ結合を持つことを照合する。

| 宣言 | 結合構造 |
| --- | --- |
| E0 | <code>true &#124;&#124; (false &amp;&amp; false)</code> |
| E1 | `true && (1 == 1)` |
| E2 | `1 == (2 + 3)` |
| E3 | `1 + (2 * 3)` |
| E4 | `2 * (-3)` |
| E5 | `-(1 as i32)` |
| E6 | `1 + (2 as i32)` |
| E7 | `(((f())[0]).X as i32) as u32` |
| E8 | <code>(true &#124;&#124; false) &#124;&#124; false</code> |
| E9 | `(true && false) && false` |
| E10 | `(1 < 2) == true` |
| E11 | <code>((((8 - 3) + 2) &#124; 1) ^ 4)</code> |
| E12 | `((((((32 / 2) * 3) % 5) << 1) >> 1) & 7) &^ 2` |
| E13 | `!(-(^x))` |
| E14 | `(1 + 2) * 3` |
