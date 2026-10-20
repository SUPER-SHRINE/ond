# static-scopeの入力と期待値
<!-- ond-spec-id: conformance/cases/static-scope -->

manifest/package/import成立条件、file scope、循環検査、通常mainのsignatureとinitの通常関数規則を各fixtureへ適用する。

規範定義元: [INIT-03の定義元](../../language/v1/execution.md#初期化とエントリ)、[PKG-01の定義元](../../language/v1/names-and-scopes.md#packageとimport)、[PKG-02の定義元](../../language/v1/names-and-scopes.md#packageとimport)、[PKG-03の定義元](../../language/v1/names-and-scopes.md#scopeとshadowing)。\
要件索引は[requirements](../requirements.md)、既存fixture宣言は[registry](../../../../crates/compiler-core/src/conformance/scope.rs)を参照する。

共通のproject入力は`ond.toml`の`[project]`に`name = "test"`を置いた実行projectである。\
generated numeric/型境界のsourceはすべて`main.ond`の内容として読み、static表は各file名を入力欄に記す。\
例外`PKG-01.missing_manifest`はond.tomlを持たず、それ以外の行は共通manifestを使う。

`accept`単独は受理を期待し、値の記載がある行はその型・値まで照合する。\
構文解釈は表のParse phaseで照合し、内部AST/MIRの構造検査は外部適合判定へ含めない。\
診断と原因位置の比較は[共通case契約](./README.md#入力と期待結果の読み方)に従う。

| Case ID | 要件ID | 入力 | 仕様由来の期待結果 | Phase・原因位置 |
| --- | --- | --- | --- | --- |
| <code>PKG-01.missing_manifest</code> | `PKG-01` | <code>main.ond:<br>package main<br>func main() {}<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: project</code> |
| <code>PKG-01.empty_project</code> | `PKG-01` | <code></code> | <code>compile error</code> | <code>Core; reject; 原因位置: project</code> |
| <code>PKG-01.root_package_mismatch</code> | `PKG-01` | <code>main.ond:<br>package other<br>func main() {}<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond、project</code> |
| <code>PKG-01.directory_package_mismatch</code> | `PKG-01` | <code>main.ond:<br>package main<br>func main() {}<br><br>lib/a.ond:<br>package wrong<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: lib/a.ond</code> |
| <code>PKG-01.mixed_package_clauses</code> | `PKG-01` | <code>main.ond:<br>package main<br>func main() {}<br><br>lib/a.ond:<br>package lib<br><br>lib/b.ond:<br>package wrong<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: lib/b.ond、lib/a.ond</code> |
| <code>PKG-01.empty_source</code> | `PKG-01` | <code>main.ond:<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>PKG-01.declaration_free_package</code> | `PKG-01` | <code>main.ond:<br>package main<br>import &quot;test/lib&quot;<br>func main() {}<br><br>lib/empty.ond:<br>package lib<br></code> | <code>accept; </code> | <code>Core; accept</code> |
| <code>PKG-02.missing_import</code> | `PKG-02` | <code>main.ond:<br>package main<br>import &quot;missing&quot;<br>func main() {}<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>PKG-02.root_import</code> | `PKG-02` | <code>main.ond:<br>package main<br>func main() {}<br><br>lib/a.ond:<br>package lib<br>import &quot;.&quot;<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: lib/a.ond</code> |
| <code>PKG-02.self_cycle</code> | `PKG-02` | <code>main.ond:<br>package main<br>func main() {}<br><br>lib/a.ond:<br>package lib<br>import &quot;test/lib&quot;<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: project</code> |
| <code>PKG-02.multi_cycle</code> | `PKG-02` | <code>main.ond:<br>package main<br>func main() {}<br><br>a/a.ond:<br>package a<br>import &quot;test/b&quot;<br><br>b/b.ond:<br>package b<br>import &quot;test/c&quot;<br><br>c/c.ond:<br>package c<br>import &quot;test/a&quot;<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: project</code> |
| <code>PKG-02.invalid_path_parent</code> | `PKG-02` | <code>main.ond:<br>package main<br>import &quot;../lib&quot;<br>func main() {}<br><br>lib/a.ond:<br>package lib<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>PKG-02.invalid_path_absolute</code> | `PKG-02` | <code>main.ond:<br>package main<br>import &quot;/lib&quot;<br>func main() {}<br><br>lib/a.ond:<br>package lib<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>PKG-02.invalid_path_dot</code> | `PKG-02` | <code>main.ond:<br>package main<br>import &quot;./lib&quot;<br>func main() {}<br><br>lib/a.ond:<br>package lib<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>PKG-02.invalid_path_empty</code> | `PKG-02` | <code>main.ond:<br>package main<br>import &quot;&quot;<br>func main() {}<br><br>lib/a.ond:<br>package lib<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>PKG-02.invalid_path_trailing</code> | `PKG-02` | <code>main.ond:<br>package main<br>import &quot;lib/&quot;<br>func main() {}<br><br>lib/a.ond:<br>package lib<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>PKG-02.invalid_path_backslash</code> | `PKG-02` | <code>main.ond:<br>package main<br>import &quot;lib\\sub&quot;<br>func main() {}<br><br>lib/a.ond:<br>package lib<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>PKG-03.file_scoped_imports</code> | `PKG-03` | <code>main.ond:<br>package main<br>import &quot;test/nested/lib&quot;<br>var G: lib.T = lib.C as lib.T<br>const C = lib.C<br>func main() { G = lib.G; }<br><br>z.ond:<br>package main<br>import &quot;test/nested/lib&quot;<br><br>zz.ond:<br>package main<br>import &quot;test/nested/lib&quot;<br><br>zzz.ond:<br>package main<br>import &quot;test/nested/lib&quot;<br>const Later = lib.C<br>func later() -&gt; lib.T { return lib.G; }<br><br>nested/lib/a.ond:<br>package lib<br>type T i32<br>const C = 7<br>var G: T = 3 as T<br></code> | <code>accept; C: i32 = integer bits 0x0000000000000007; Later: i32 = integer bits 0x0000000000000007</code> | <code>Core; accept</code> |
| <code>PKG-03.same_alias_in_distinct_files</code> | `PKG-03` | <code>main.ond:<br>package main<br>import &quot;test/a/lib&quot;<br>func main() {}<br><br>z.ond:<br>package main<br>import &quot;test/b/lib&quot;<br><br>a/lib/a.ond:<br>package lib<br><br>b/lib/b.ond:<br>package lib<br></code> | <code>accept; </code> | <code>Core; accept</code> |
| <code>INIT-03.missing_main</code> | `INIT-03` | <code>main.ond:<br>package main<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: project</code> |
| <code>INIT-03.other_package_main_is_not_entry</code> | `INIT-03` | <code>main.ond:<br>package main<br><br>lib/a.ond:<br>package lib<br>func main() {}<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: project</code> |
| <code>INIT-03.other_package_main_allowed</code> | `INIT-03` | <code>main.ond:<br>package main<br>func main() {}<br><br>lib/a.ond:<br>package lib<br>func main(x: i32) -&gt; i32 { return x; }<br></code> | <code>accept; </code> | <code>Core; accept</code> |
| <code>INIT-03.duplicate_main</code> | `INIT-03` | <code>main.ond:<br>package main<br>func main() {}<br><br>z.ond:<br>package main<br>func main() {}<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: z.ond</code> |
| <code>INIT-03.main_type_conflict</code> | `INIT-03` | <code>main.ond:<br>package main<br>type main i32<br>func main() {}<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>INIT-03.main_parameter</code> | `INIT-03` | <code>main.ond:<br>package main<br>func main(x: i32) {  }<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>INIT-03.main_return</code> | `INIT-03` | <code>main.ond:<br>package main<br>func main() -&gt; i32 { return 0; }<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>INIT-03.main_both</code> | `INIT-03` | <code>main.ond:<br>package main<br>func main(x: i32) -&gt; i32 { return x; }<br></code> | <code>compile error</code> | <code>Core; reject; 原因位置: main.ond</code> |
| <code>INIT-03.init_empty</code> | `INIT-03` | <code>main.ond:<br>package main<br>func main() {}<br>func init() {  }<br></code> | <code>accept; </code> | <code>Core; accept</code> |
| <code>INIT-03.init_parameter</code> | `INIT-03` | <code>main.ond:<br>package main<br>func main() {}<br>func init(x: i32) {  }<br></code> | <code>accept; </code> | <code>Core; accept</code> |
| <code>INIT-03.init_return</code> | `INIT-03` | <code>main.ond:<br>package main<br>func main() {}<br>func init() -&gt; i32 { return 0; }<br></code> | <code>accept; </code> | <code>Core; accept</code> |
| <code>INIT-03.init_both</code> | `INIT-03` | <code>main.ond:<br>package main<br>func main() {}<br>func init(x: i32) -&gt; i32 { return x; }<br></code> | <code>accept; </code> | <code>Core; accept</code> |
| <code>INIT-03.imported_init</code> | `INIT-03` | <code>main.ond:<br>package main<br>func main() {}<br><br>lib/a.ond:<br>package lib<br>func init() {}<br></code> | <code>accept; </code> | <code>Core; accept</code> |
