# Cacheとlinkの回帰入力
<!-- ond-spec-id: conformance/cases/build-observations -->

適用版はOnd Language v1、Kagura v1、artifact Contract revision 6とし、[版互換条件](../../artifacts.md#版識別と互換条件)を適用する。\
要件は`INIT-01`であり、操作の規範定義元は[cache](../../build/cache.md)、[compile/link](../../build/commands.md)、[linking](../../linking.md)、[初期化](../../language/v1/execution.md#初期化とエントリ)とする。

## Project入力と初期状態
<!-- ond-spec-id: conformance/build-observations/project-input -->

`ond.toml`は次の全文（LF区切り・末尾LFあり）とする。

```toml
[project]
name = "test"
[link]
startup = "external"
stack-size = 0x10000
[link.ram]
base = 0x1000
size = 0x40000
[link.exit]
mode = "jump"
address = 0
```

通常build、manifest記載のprojectだけ、既存cacheなしの状態から始め、同じcompiler executableを全操作で用いる。\
各sourceは次の内容であり、入力欄の改行は[共通復元規則](./README.md#入力と期待結果の読み方)に従う。

| File | 全source |
| --- | --- |
| `main.ond` | <code>package main<br>import &quot;test/right&quot;<br>import &quot;test/left&quot;<br>func main(){var _=left.Value()+right.Value()}</code> |
| `leaf/leaf.ond` | <code>package leaf<br>var Seed:i32=8<br>func Value()-&gt;i32{var n=Seed;return n/3}</code> |
| `left/left.ond` | <code>package left<br>import &quot;test/leaf&quot;<br>func Value()-&gt;i32{return leaf.Value()}</code> |
| `right/right.ond` | <code>package right<br>import &quot;test/leaf&quot;<br>func Value()-&gt;i32{return leaf.Value()}</code> |
| `other/other.ond` | <code>package other<br>func Value()-&gt;i32{return 5}</code> |

## 操作と比較
<!-- ond-spec-id: conformance/build-observations/operations -->

project rootをcwdとして`ond compile .`、`ond link`を行い、正常linkは実行可能バイナリの全bytesとsidecarのJSON値を比較する。

| 操作 | 期待 |
| --- | --- |
| cold compile後、manifestと参照artifactを読みlink | compile/link成功 |
| source・設定・compiler不変でwarm compileしlink | coldと同じバイナリ全bytesとsidecarのJSON値 |
| cold manifestのpackagesからtest/leafのArtifactRefを除去してlink | 依存入力不足のlink error、CLI終了code 1 |
| cold manifestのcontract.revisionを7へ変更してlink | 版不適合のlink error、CLI終了code 1 |
| cold manifestのcompiler_idの先頭hex digitだけを別値へ変更（0なら1、他は0）してlink | 生成元不一致のlink error、CLI終了code 1 |
| leafのSeedだけ9へ変更して再compileし、cold manifestのleaf ArtifactRefだけを新しい参照へ置換してlink | 依存build_id不整合のlink error、CLI終了code 1 |

この入力での観測はcache再利用時のimageと依存整合性である。\
各packageのinitializerの存在と内部表現は[初期化の許容規則](../../language/v1/execution.md#初期化とエントリ)に従い、初期化順全体の区別に必要な観測はこのcaseの比較項目に含まれない。

## Manifest依存のnamespaceと生成source
<!-- ond-spec-id: conformance/build-observations/dependency-projects -->

要件は`PKG-01`、`PKG-02`とし、[manifest依存](../../build/packages.md#manifestとsource探索)、[packageとimport](../../language/v1/names-and-scopes.md#packageとimport)、[生成source入力](../../build/commands.md#入出力と失敗時)を適用する。\
各caseは独立した新しいdirectoryで行い、表の全fileをUTF-8、LF、末尾LFありで置く。表の`<br>`は共通復元規則でLFへ戻す。\
各caseのp/ond.tomlへ、[共有project入力](#project入力と初期状態)のlink・link.ram・link.exit tableを末尾に追記する。\
依存projectのmanifestにはlink設定を追加しない。\
pをcwdとして通常compileと`--test` compileをそれぞれ行い、成功時は各manifestを位置引数に明示してlinkする。\
成功はcompile/linkの終了code 0、compile errorは終了code 1とstderr診断で比較する。

共通fileは`p/main_test.ond`の全文`package main`、`func TestRoot(){}`の2行とする。

| Case | 追加fileと全内容 | 期待 |
| --- | --- | --- |
| dependency-library-main-function | `p/ond.toml`: <code>[project]<br>name="p"<br>[dependencies]<br>lib={path="../lib"}</code><br>`p/main.ond`: <code>package main<br>import "hogeLib"<br>func main(){_=hogeLib.Value()}</code><br>`lib/ond.toml`: <code>[project]<br>name="hogeLib"</code><br>`lib/main.ond`: <code>package hogeLib<br>func main(){}<br>func Value()->u32{return 1}</code> | 通常/test compileと各linkが成功。test一覧は`package="p"`、`name="TestRoot"`、`symbol="main.TestRoot"`の1件 |
| dependency-library-main-subpackage | `dependency-library-main-function`の`lib/main.ond`に代えて`lib/main/api.ond`: <code>package main<br>func main(){}<br>func Value()->u32{return 1}</code>を置き、`p/main.ond`を<code>package main<br>import other "hogeLib/main"<br>func main(){_=other.Value()}</code>へ置換 | 通常/test compileと各linkが成功。依存packageはhogeLib/mainで、実行rootはp |
| dependency-application-root | `dependency-library-main-function`の`lib/ond.toml`のnameを`main`、`lib/main.ond`のpackageを`main`、`p/main.ond`のimportを`import other "main"`、呼出しを`other.Value()`へ置換 | 通常/test compileとも実行root importのcompile error |
| dependency-application-subpackage | `dependency-application-root`へ`lib/api/api.ond`: <code>package api<br>func Value()->u32{return 2}</code>を追加し、`p/main.ond`を<code>package main<br>import "main/api"<br>func main(){_=api.Value()}</code>へ置換 | 通常/test compileと各linkが成功 |
| generated-path-project | `p/ond.toml`: <code>[project]<br>name="p"<br>[dependencies]<br>generated={path="../generated"}</code><br>`p/main.ond`: <code>package main<br>import "generated"<br>func main(){_=generated.Value()}</code><br>生成操作で`generated/ond.toml`: <code>[project]<br>name="generated"</code>と`generated/api.ond`: <code>package generated<br>func Value()->u32{return 3}</code>を書き終えてからcompile | 通常/test compileと各linkが成功。生成directoryはcompile完了後に削除できる |
