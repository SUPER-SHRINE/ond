# static-namesの入力と期待値
<!-- ond-spec-id: conformance/cases/static-names -->

bindingのscope開始点と一意性、shadowingとif/for scopeの可視範囲から結果を導く。

規範定義元: [NAME-01の定義元](../../language/v1/names-and-scopes.md#scopeとshadowing)、[NAME-02の定義元](../../language/v1/names-and-scopes.md#scopeとshadowing)。\
要件索引は[requirements](../requirements.md)、既存fixture宣言は[registry](../../../../crates/compiler-core/src/conformance/names.rs)を参照する。

共通のproject入力は`ond.toml`の`[project]`に`name = "test"`を置いた実行projectである。\
generated numeric/型境界のsourceはすべて`main.ond`の内容として読み、static表は各file名を入力欄に記す。

`accept`単独は受理を期待し、値の記載がある行はその型・値まで照合する。\
構文解釈は表のParse phaseで照合し、内部AST/MIRの構造検査は外部適合判定へ含めない。\
診断と原因位置の比較は[共通case契約](./README.md#入力と期待結果の読み方)に従う。

| Case ID | 要件ID | 入力 | 仕様由来の期待結果 | Phase・原因位置 |
| --- | --- | --- | --- | --- |
| <code>NAME-01.local_var_var</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>func test() { var Clash = 1; var Clash = 2; }<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>NAME-01.local_var_const</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>func test() { var Clash = 1; const Clash = 2; }<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>NAME-01.local_const_var</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>func test() { const Clash = 1; var Clash = 2; }<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>NAME-01.local_const_const</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>func test() { const Clash = 1; const Clash = 2; }<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>NAME-01.parameter_var</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>func test(Clash: i32) { var Clash = 1; }<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>NAME-01.parameter_const</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>func test(Clash: i32) { const Clash = 1; }<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>NAME-01.duplicate_parameter</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>func test(Clash: i32, Clash: i32) {}<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>NAME-01.duplicate_local_names</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>func test() { var Clash, Clash = 1, 2; }<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>NAME-01.duplicate_const_names</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>func test() { const Clash, Clash = 1, 2; }<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>NAME-02.var_self</code> | `NAME-02` | <code>main.ond:<br>package main<br>func main() {}<br>func test() { var Fresh = Fresh; }<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>NAME-02.var_simultaneous</code> | `NAME-02` | <code>main.ond:<br>package main<br>func main() {}<br>func test() { var Fresh, Other = 1, Fresh; }<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>NAME-02.const_simultaneous</code> | `NAME-02` | <code>main.ond:<br>package main<br>func main() {}<br>func test() { const Fresh, Other = 1, Fresh; }<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>NAME-02.short_simultaneous</code> | `NAME-02` | <code>main.ond:<br>package main<br>func main() {}<br>func test() { Fresh, Other := 1, Fresh; }<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>NAME-02.block_outside</code> | `NAME-02` | <code>main.ond:<br>package main<br>func main() {}<br>func test() { { var Fresh = 1; }; _ = Fresh; }<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>NAME-02.if_outside</code> | `NAME-02` | <code>main.ond:<br>package main<br>func main() {}<br>func test() { if Fresh := true; Fresh {}; _ = Fresh; }<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>NAME-02.for_outside</code> | `NAME-02` | <code>main.ond:<br>package main<br>func main() {}<br>func test() { for Fresh := 0; Fresh &lt; 1; Fresh = Fresh + 1 {}; _ = Fresh; }<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>NAME-02.then_not_else</code> | `NAME-02` | <code>main.ond:<br>package main<br>func main() {}<br>func test() { if true { var Fresh = 1; } else { _ = Fresh; } }<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>NAME-02.for_body_not_post</code> | `NAME-02` | <code>main.ond:<br>package main<br>func main() {}<br>func test() { for i := 0; i &lt; 1; i = Fresh { var Fresh = 1; } }<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>NAME-02.shadow_import_scalar</code> | `NAME-02` | <code>main.ond:<br>package main<br>import &quot;test/shared&quot;<br>func main() {}<br>func test() { var shared = 1; _ = shared.Exported; }<br><br>shared/a.ond:<br>package shared<br>const Exported = 7<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
