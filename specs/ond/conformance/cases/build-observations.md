# Cacheとlinkの回帰入力
<!-- ond-spec-id: conformance/cases/build-observations -->

適用版はOnd Language v1、Kagura v1、artifact Contract revision 7とし、[版互換条件](../../artifacts.md#版識別と互換条件)を適用する。\
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

build、manifest記載のprojectだけ、既存cacheなしの状態から始め、同じcompiler executableを全操作で用いる。\
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
| cold manifestのcontract.revisionを8へ変更してlink | 版不適合のlink error、CLI終了code 1 |
| cold manifestのcompiler_idの先頭hex digitだけを別値へ変更（0なら1、他は0）してlink | 生成元不一致のlink error、CLI終了code 1 |
| leafのSeedだけ9へ変更して再compileし、cold manifestのleaf ArtifactRefだけを新しい参照へ置換してlink | 依存build_id不整合のlink error、CLI終了code 1 |

この入力での観測はcache再利用時のimageと依存整合性である。\
各packageのinitializerの存在と内部表現は[初期化の許容規則](../../language/v1/execution.md#初期化とエントリ)に従い、初期化順全体の区別に必要な観測はこのcaseの比較項目に含まれない。

## Source file集合とpackage scope
<!-- ond-spec-id: conformance/build-observations/source-files -->

要件は`PKG-01`、`PKG-04`、`SYN-10`、`ROLE-05`とし、[source探索境界](../../build/packages.md#source探索境界)、[package scope](../../language/v1/names-and-scopes.md#scopeとshadowing)、[関数宣言](../../language/v1/declarations.md#関数)と[LSPの名前解決](../../language/v1/roles.md#lspの補完と診断)を適用する。\
PackageBuildInputとsource hashは[公開入力規則](../../build/cache.md#公開識別子)から構成する。\
独立したprojectのmanifestは`[project]`、`name = "p"`の2行とし、次の3 fileとともにUTF-8、LF区切り、末尾LFありで置く。\
依存、role設定、既存cacheと他のsourceは持たず、project rootで`ond compile .`を実行する。

`main.ond`:

```ond
package main
var Result:u32
func main(){var value:Device;_=value.secret;Result=TestArgument(TestResult())+TestGeneric[u32](2)+value.TestMethod()+Collect()}
```

`test.ond`:

```ond
package main
type Device struct{secret:u32}
func TestArgument(x:u32)->u32{return x}
func TestResult()->u32{return 1}
func TestGeneric[T](x:T)->T{return x}
func(x:Device) TestMethod()->u32{return x.secret+3}
func privateHelper()->u32{return 4}
```

`feature_test.ond`:

```ond
package main
func Collect()->u32{return privateHelper()}
```

各行はこの入力から独立して開始する。\
MemoryVmは[case共通契約](./README.md#入力と期待結果の読み方)のfreshなglobal状態を用い、LSPは新規sessionで3 fileを開く。

| Case ID | 操作 | 期待・比較方法 |
| --- | --- | --- |
| PKG-01.source-files | compile後にrootのPackageBuildInput.sourcesを比較 | 成功し、path列はfeature_test.ond、main.ond、test.ondの順で、各content_hashは対応する全file bytesのSHA-256に一致 |
| PKG-04.source-file-scope | Core検査後、MemoryVmでmainを引数なしで1回呼ぶ | privateHelperとDevice.secretを同一packageの宣言として解決し、Resultはu32の10 |
| SYN-10.function-name-prefix | Core検査で各関数の宣言型を比較 | TestArgumentはu32 parameter 1個とu32 result 1個、TestResultはu32 result 1個、TestGenericは型parameter T 1個とT parameter/result各1個、TestMethodはDevice receiverとu32 result 1個として受理 |
| PKG-01.source-file-lsp | mainのResult代入の右辺先頭、TestArgumentのreturn式先頭、Collectのreturn式先頭で補完 | 各位置の関数候補にprivateHelper、Collect、TestArgument、TestResult、TestGenericを含む。mainのvalue.secret位置ではsecret fieldを解決 |

## Manifest依存のnamespaceと生成source
<!-- ond-spec-id: conformance/build-observations/dependency-projects -->

要件は`PKG-01`、`PKG-02`とし、[manifest依存](../../build/packages.md#manifestとsource探索)、[packageとimport](../../language/v1/names-and-scopes.md#packageとimport)、[生成source入力](../../build/commands.md#入出力と失敗時)を適用する。\
各caseは独立した新しいdirectoryで行い、表の全fileをUTF-8、LF、末尾LFありで置く。\
表の`<br>`は共通復元規則でLFへ戻す。\
各caseのp/ond.tomlへ、[共有project入力](#project入力と初期状態)のlink・link.ram・link.exit tableを末尾に追記する。\
依存projectのmanifestにはlink設定を追加しない。\
pをcwdとしてcompileを行い、成功時はmanifestを位置引数に明示してlinkする。\
成功はcompile/linkの終了code 0、compile errorは終了code 1とstderr診断で比較する。

| Case | 追加fileと全内容 | 期待 |
| --- | --- | --- |
| dependency-library-main-function | `p/ond.toml`: <code>[project]<br>name="p"<br>[dependencies]<br>lib={path="../lib"}</code><br>`p/main.ond`: <code>package main<br>import "hogeLib"<br>func main(){_=hogeLib.Value()}</code><br>`lib/ond.toml`: <code>[project]<br>name="hogeLib"</code><br>`lib/main.ond`: <code>package hogeLib<br>func main(){}<br>func Value()->u32{return 1}</code> | compileとlinkが成功 |
| dependency-library-main-subpackage | `dependency-library-main-function`の`lib/main.ond`に代えて`lib/main/api.ond`: <code>package main<br>func main(){}<br>func Value()->u32{return 1}</code>を置き、`p/main.ond`を<code>package main<br>import other "hogeLib/main"<br>func main(){_=other.Value()}</code>へ置換 | compileとlinkが成功。依存packageはhogeLib/mainで、実行rootはp |
| dependency-application-root | `dependency-library-main-function`の`lib/ond.toml`のnameを`main`、`lib/main.ond`のpackageを`main`、`p/main.ond`のimportを`import other "main"`、呼出しを`other.Value()`へ置換 | compileで実行root importのcompile error |
| dependency-application-subpackage | `dependency-application-root`へ`lib/api/api.ond`: <code>package api<br>func Value()->u32{return 2}</code>を追加し、`p/main.ond`を<code>package main<br>import "main/api"<br>func main(){_=api.Value()}</code>へ置換 | compileとlinkが成功 |
| generated-path-project | `p/ond.toml`: <code>[project]<br>name="p"<br>[dependencies]<br>generated={path="../generated"}</code><br>`p/main.ond`: <code>package main<br>import "generated"<br>func main(){_=generated.Value()}</code><br>生成操作で`generated/ond.toml`: <code>[project]<br>name="generated"</code>と`generated/api.ond`: <code>package generated<br>func Value()->u32{return 3}</code>を書き終えてからcompile | compileとlinkが成功。生成directoryはcompile完了後に削除できる |
