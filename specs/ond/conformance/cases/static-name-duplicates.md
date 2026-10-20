# static-name-duplicatesの入力と期待値
<!-- ond-spec-id: conformance/cases/static-name-duplicates -->

package scopeでtype/var/const/function名が一意である条件を、二つの宣言の組合せへ適用する。

規範定義元: [NAME-01の定義元](../../language/v1/names-and-scopes.md#scopeとshadowing)。\
要件索引は[requirements](../requirements.md)、既存fixture宣言は[registry](../../../../crates/compiler-core/src/conformance/name_duplicates.rs)を参照する。

入力・phase・比較の共通条件は[case契約](./README.md#入力と期待結果の読み方)に従う。

| Case ID | 要件ID | 入力 | 仕様由来の期待結果 | Phase・原因位置 |
| --- | --- | --- | --- | --- |
| <code>NAME-01.package_var_var</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>var Clash: i32<br><br>z.ond:<br>package main<br>var Clash: i32<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: z.ond</code> |
| <code>NAME-01.package_var_const</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>var Clash: i32<br><br>z.ond:<br>package main<br>const Clash = 1<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: z.ond</code> |
| <code>NAME-01.package_var_func</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>var Clash: i32<br><br>z.ond:<br>package main<br>func Clash() {}<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: z.ond</code> |
| <code>NAME-01.package_var_type</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>var Clash: i32<br><br>z.ond:<br>package main<br>type Clash i32<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: z.ond</code> |
| <code>NAME-01.package_const_var</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>const Clash = 1<br><br>z.ond:<br>package main<br>var Clash: i32<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: z.ond</code> |
| <code>NAME-01.package_const_const</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>const Clash = 1<br><br>z.ond:<br>package main<br>const Clash = 1<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: z.ond</code> |
| <code>NAME-01.package_const_func</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>const Clash = 1<br><br>z.ond:<br>package main<br>func Clash() {}<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: z.ond</code> |
| <code>NAME-01.package_const_type</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>const Clash = 1<br><br>z.ond:<br>package main<br>type Clash i32<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: z.ond</code> |
| <code>NAME-01.package_func_var</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>func Clash() {}<br><br>z.ond:<br>package main<br>var Clash: i32<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: z.ond</code> |
| <code>NAME-01.package_func_const</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>func Clash() {}<br><br>z.ond:<br>package main<br>const Clash = 1<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: z.ond</code> |
| <code>NAME-01.package_func_func</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>func Clash() {}<br><br>z.ond:<br>package main<br>func Clash() {}<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: z.ond</code> |
| <code>NAME-01.package_func_type</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>func Clash() {}<br><br>z.ond:<br>package main<br>type Clash i32<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: z.ond</code> |
| <code>NAME-01.package_type_var</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>type Clash i32<br><br>z.ond:<br>package main<br>var Clash: i32<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: z.ond</code> |
| <code>NAME-01.package_type_const</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>type Clash i32<br><br>z.ond:<br>package main<br>const Clash = 1<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: z.ond</code> |
| <code>NAME-01.package_type_func</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>type Clash i32<br><br>z.ond:<br>package main<br>func Clash() {}<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: z.ond</code> |
| <code>NAME-01.package_type_type</code> | `NAME-01` | <code>main.ond:<br>package main<br>func main() {}<br>type Clash i32<br><br>z.ond:<br>package main<br>type Clash i32<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: z.ond</code> |
