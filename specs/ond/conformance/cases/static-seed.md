# static-seedの入力と期待値
<!-- ond-spec-id: conformance/cases/static-seed -->

各行の要件と定義元に従い、function型文法、空result error、import定数の値、組み込み名とroot main条件を適用する。\
構文解釈と宣言値をそれぞれのphaseで比較する。

規範定義元: [DIAG-02の定義元](../../language/v1/declarations.md#診断と成功結果)、[FP-01の定義元](../../language/v1/expressions.md#floating-point-arithmetic)、[NAME-03の定義元](../../language/v1/names-and-scopes.md#scopeとshadowing)、[NIL-01の定義元](../../language/v1/types.md#nilとpointer)、[NUM-02の定義元](../../language/v1/expressions.md#integer-constant-arithmetic)、[PKG-01の定義元](../../language/v1/names-and-scopes.md#packageとimport)、[PKG-03の定義元](../../language/v1/names-and-scopes.md#scopeとshadowing)、[STR-01の定義元](../../language/v1/expressions.md#文字列リテラル)、[SYN-11の定義元](../../language/v1/declarations.md#関数)、[SYN-12の定義元](../../language/v1/lexical.md#文法の定義元)。\
要件索引は[requirements](../requirements.md)、既存fixture宣言は[registry](../../../../crates/compiler-core/src/conformance/cases.rs)を参照する。

共通のproject入力は`ond.toml`の`[project]`に`name = "test"`を置いた実行projectである。\
generated numeric/型境界のsourceはすべて`main.ond`の内容として読み、static表は各file名を入力欄に記す。

`accept`単独は受理を期待し、値の記載がある行はその型・値まで照合する。\
構文解釈は表のParse phaseで照合し、内部AST/MIRの構造検査は外部適合判定へ含めない。\
診断と原因位置の比較は[共通case契約](./README.md#入力と期待結果の読み方)に従う。

| Case ID | 要件ID | 入力 | 仕様由来の期待結果 | Phase・原因位置 |
| --- | --- | --- | --- | --- |
| <code>SYN-11.unnamed-parameter.ast</code> | `SYN-11` | <code>main.ond:<br>package main<br>type F func(i32) -&gt; i32<br></code> | <code>accept; 構文解釈: one unnamed parameter, not a blank binding</code> | <code>Parse; accept</code> |
| <code>SYN-12.empty-results.reject</code> | `SYN-12` | <code>main.ond:<br>package main<br>func main() -&gt; () {}<br></code> | <code>parse error</code> | <code>Parse; reject; 原因位置: main.ond</code> |
| <code>NUM-02.imported-constant.core</code> | `NUM-02`, `PKG-03`, `FP-01`, `NIL-01`, `STR-01` | <code>main.ond:<br>package main<br>import &quot;test/shared&quot;<br>const Answer = shared.Base * 2<br>const Flag = true<br>const Text = &quot;音&quot;<br>const Pointer: *u8 = nil<br>const F = -0.0<br>func main() {}<br>func result() -&gt; i32 { return Answer; }<br><br>shared/shared.ond:<br>package shared<br>const Base = 21<br></code> | <code>accept; Answer: i32 = integer bits 0x000000000000002a; Flag: bool = true; Text: [3]u8 = bytes [233, 159, 179]; Pointer: *u8 = typed nil; F: f32 = f32 bits 0x80000000</code> | <code>Core; accept</code> |
| <code>NAME-03.unicode-multifile.reject</code> | `NAME-03`, `DIAG-02` | <code>main.ond:<br>package main<br>// i32<br>func main() {}<br><br>other.ond:<br>package main<br>// 音のコメント<br>func f() { var i32 = 1; }<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: other.ond</code> |
| <code>PKG-01.missing-main-file.reject</code> | `PKG-01` | <code>other.ond:<br>package main<br>func main() {}<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: project</code> |
