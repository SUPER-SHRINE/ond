# static-diagnosticsの入力と期待値
<!-- ond-spec-id: conformance/cases/static-diagnostics -->

不正なsourceにcompile errorを返し、UTF-8 byte spanを現在のsourceへ対応づける。\
独立エラーを収集し、成功条件を満たした全体だけを成功結果とする規則から期待診断を導く。

規範定義元: [DIAG-01の定義元](../../language/v1/declarations.md#診断と成功結果)、[DIAG-02の定義元](../../language/v1/declarations.md#診断と成功結果)。\
要件索引は[requirements](../requirements.md)、既存fixture宣言は[registry](../../../../crates/compiler-core/src/conformance/diagnostics.rs)を参照する。

共通のproject入力は`ond.toml`の`[project]`に`name = "test"`を置いた実行projectである。\
generated numeric/型境界のsourceはすべて`main.ond`の内容として読み、static表は各file名を入力欄に記す。

`accept`単独は受理を期待し、値の記載がある行はその型・値まで照合する。\
構文解釈は表のParse phaseで照合し、内部AST/MIRの構造検査は外部適合判定へ含めない。\
診断と原因位置の比較は[共通case契約](./README.md#入力と期待結果の読み方)に従う。

| Case ID | 要件ID | 入力 | 仕様由来の期待結果 | Phase・原因位置 |
| --- | --- | --- | --- | --- |
| <code>DIAG-02.multifile-unicode-hir</code> | `DIAG-01`, `DIAG-02` | <code>main.ond:<br>package main<br>func main() {}<br><br>00.ond:<br>package main&#13;<br>// 日本語😀 true&#13;<br>func check(値: bool) { var 数: i32 = 値 }&#13;<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: 00.ond; 原因: bool値をi32で初期化</code> |
| <code>DIAG-02.imported-package-body</code> | `DIAG-01`, `DIAG-02` | <code>main.ond:<br>package main<br>import &quot;test/shared&quot;<br>func main() {}<br><br>shared/値.ond:<br>package shared<br>// 𐐀😀<br>func Value() -&gt; i32 { return 未定義 }<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: shared/値.ond; 原因: 未定義名の参照</code> |
| <code>DIAG-01.multiple-independent-errors</code> | `DIAG-01`, `DIAG-02` | <code>main.ond:<br>package main<br>func main() {}<br>func valid() -&gt; i32 { return 42 }<br><br>a.ond:<br>package main<br>// 😀<br>func first() { var x: i32 = true }<br><br>z.ond:<br>package main<br>// 日本語<br>func last() { var x = missing }<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: a.ond、z.ond; 原因: a.ondのbool値によるi32初期化とz.ondの未定義名を独立に報告</code> |
| <code>DIAG-01.invalid-global-no-partial-success</code> | `DIAG-01`, `DIAG-02` | <code>main.ond:<br>package main<br>func main() {}<br>func valid() -&gt; i32 { return 1 }<br><br>global.ond:<br>package main<br>// 😀<br>var 値: bool = 1<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: global.ond; 原因: bool変数を整数値で初期化</code> |
| <code>DIAG-02.unicode-lexical-byte-range</code> | `DIAG-01`, `DIAG-02` | <code>main.ond:<br>package main&#13;<br>// 日本語&#13;<br>func main() { 😀 }&#13;<br></code> | <code>parse error</code> | <code>Parse; reject; 原因位置: main.ond; 原因: source文字の字句不成立</code> |
| <code>DIAG-02.unicode-parser-byte-range</code> | `DIAG-01`, `DIAG-02` | <code>main.ond:<br>package main&#13;<br>// 日本語😀&#13;<br>func main() { var 値 = 1 var 次 = 2 }&#13;<br></code> | <code>parse error</code> | <code>Parse; reject; 原因位置: main.ond; 原因: 文separatorの欠落</code> |
