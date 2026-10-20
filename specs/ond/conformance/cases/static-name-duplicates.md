# static-name-duplicatesの入力と期待値
<!-- ond-spec-id: conformance/cases/static-name-duplicates -->

package scopeでtype/var/const/function名が一意である条件を、二つの宣言の組合せへ適用する。

規範定義元: [NAME-01の定義元](../../language/v1/names-and-scopes.md#scopeとshadowing)。\
要件索引は[requirements](../requirements.md)、既存fixture宣言は[registry](../../../../crates/compiler-core/src/conformance/name_duplicates.rs)を参照する。

共通のproject入力は`ond.toml`の`[project]`に`name = "test"`を置いた実行projectである。\
generated numeric/型境界のsourceはすべて`main.ond`の内容として読み、static表は各file名を入力欄に記す。

以下は移行時の既存case宣言に含まれる入力と仕様由来Expectedデータを記録したものであり、compilerを実行して得た結果の転記ではない。\
`accept`単独は受理を期待し、値の記載がある行はその型・値まで照合する。\
`implementation AST/MIR`は外部の受理・値期待と区分した実装回帰検査である。\
診断の原文は照合対象を示し、規範上の失敗結果はcompile errorとして読む。

| Case ID | 要件ID | 入力 | 仕様由来の期待結果と内部検査 |
| --- | --- | --- | --- |
| <code>NAME-01.package_var_var</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>var Clash: i32<br><br>z.ond:<br>package main<br>var Clash: i32<br></code> | <code>compile error; duplicate top-level declaration</code> |
| <code>NAME-01.package_var_const</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>var Clash: i32<br><br>z.ond:<br>package main<br>const Clash = 1<br></code> | <code>compile error; duplicate top-level declaration</code> |
| <code>NAME-01.package_var_func</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>var Clash: i32<br><br>z.ond:<br>package main<br>func Clash() {}<br></code> | <code>compile error; duplicate top-level declaration</code> |
| <code>NAME-01.package_var_type</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>var Clash: i32<br><br>z.ond:<br>package main<br>type Clash i32<br></code> | <code>compile error; duplicate top-level declaration</code> |
| <code>NAME-01.package_const_var</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>const Clash = 1<br><br>z.ond:<br>package main<br>var Clash: i32<br></code> | <code>compile error; duplicate top-level declaration</code> |
| <code>NAME-01.package_const_const</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>const Clash = 1<br><br>z.ond:<br>package main<br>const Clash = 1<br></code> | <code>compile error; duplicate top-level declaration</code> |
| <code>NAME-01.package_const_func</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>const Clash = 1<br><br>z.ond:<br>package main<br>func Clash() {}<br></code> | <code>compile error; duplicate top-level declaration</code> |
| <code>NAME-01.package_const_type</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>const Clash = 1<br><br>z.ond:<br>package main<br>type Clash i32<br></code> | <code>compile error; duplicate top-level declaration</code> |
| <code>NAME-01.package_func_var</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>func Clash() {}<br><br>z.ond:<br>package main<br>var Clash: i32<br></code> | <code>compile error; duplicate top-level declaration</code> |
| <code>NAME-01.package_func_const</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>func Clash() {}<br><br>z.ond:<br>package main<br>const Clash = 1<br></code> | <code>compile error; duplicate top-level declaration</code> |
| <code>NAME-01.package_func_func</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>func Clash() {}<br><br>z.ond:<br>package main<br>func Clash() {}<br></code> | <code>compile error; duplicate top-level declaration</code> |
| <code>NAME-01.package_func_type</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>func Clash() {}<br><br>z.ond:<br>package main<br>type Clash i32<br></code> | <code>compile error; duplicate top-level declaration</code> |
| <code>NAME-01.package_type_var</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>type Clash i32<br><br>z.ond:<br>package main<br>var Clash: i32<br></code> | <code>compile error; duplicate top-level declaration</code> |
| <code>NAME-01.package_type_const</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>type Clash i32<br><br>z.ond:<br>package main<br>const Clash = 1<br></code> | <code>compile error; duplicate top-level declaration</code> |
| <code>NAME-01.package_type_func</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>type Clash i32<br><br>z.ond:<br>package main<br>func Clash() {}<br></code> | <code>compile error; duplicate top-level declaration</code> |
| <code>NAME-01.package_type_type</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>type Clash i32<br><br>z.ond:<br>package main<br>type Clash i32<br></code> | <code>compile error; duplicate top-level declaration</code> |
