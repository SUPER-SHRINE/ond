# Roleの入力と期待結果
<!-- ond-spec-id: conformance/cases/roles -->

要件は`ROLE-01`〜`ROLE-10`とし、各節で対応する規範と観測操作を指定する。\
適用版はOnd Language v1、Contract revision 7、target kagura-v1、ABI ond-kagura-v1である。\
各行は独立したprojectで、既存cacheと依存projectは、そのcaseが指定する場合だけ置く。\
入力はUTF-8、LF区切り、末尾LFありとし、`<br>`はLF、HTML entityは元の文字へ復号する。\
特記しないcompile操作はproject rootで`ond compile .`を実行し、成功はexit 0、source/config入力の失敗はexit 1と原因診断を比較する。\
拒否時は以前の成功成果物を変更しないことも照合する。\
既定manifestは次の全文、既定sourceは`main.ond`の`package main`と`func main(){}`の2行である。

```toml
[project]
name = "game"
[roles]
names = ["Debug", "Development", "Release"]
```

MemoryVm操作は[case共通契約](./README.md#入力と期待結果の読み方)のfreshな状態から記載関数を引数なしで1回呼び、返値を比較する。\
LSP操作は新規sessionで入力fileを開き、明記した位置の候補集合の包含・除外と名前解決を比較する。\
補完順位や表示文言は比較対象としない。

## 設定とCLI
<!-- ond-spec-id: conformance/roles/configuration -->

要件`ROLE-01`、定義元は[設定](../../build/roles.md#manifest設定)と[CLI](../../build/commands.md#入出力と失敗時)である。\
次表の設定欄は既定manifestの`[roles]`から末尾までを置き換える。\
省略はそのtable全体を取り除く操作であり、sourceは既定sourceを使う。

| Case ID | 設定 | 操作 | 期待結果 |
| --- | --- | --- | --- |
| ROLE-01.default | 既定のまま | compile | accept、role未指定の構成 |
| ROLE-01.override | 既定のまま | `ond compile . --role Debug` | accept、選択Debug |
| ROLE-01.unconfigured | 省略 | compile | accept、role未指定の構成 |
| ROLE-01.unconfigured-option | 省略 | `ond compile . --role Debug` | exit 1、role定義欠落 |
| ROLE-01.empty | <code>[roles]<br>names=[]</code> | compile | exit 1、非空条件違反 |
| ROLE-01.duplicate | <code>[roles]<br>names=["Debug","Debug"]</code> | compile | exit 1、names重複 |
| ROLE-01.names-only | <code>[roles]<br>names=["Debug"]</code> | compile | accept、role未指定の構成 |
| ROLE-01.no-names | <code>[roles]</code> | compile | exit 1、names欠落 |
| ROLE-01.invalid-identifier | <code>[roles]<br>names=["Debug-build"]</code> | compile | exit 1、identifier不成立 |
| ROLE-01.identifier-whitespace | <code>[roles]<br>names=[" Debug "]</code> | compile | exit 1、string全体がidentifierの綴りと一致しない |
| ROLE-01.keyword | <code>[roles]<br>names=["if"]</code> | compile | exit 1、keywordはidentifier tokenでない |
| ROLE-01.unknown-key | <code>[roles]<br>names=["Debug"]<br>default="Debug"</code> | compile | exit 1、未知key |
| ROLE-01.names-type | <code>[roles]<br>names="Debug"</code> | compile | exit 1、names型不正 |
| ROLE-01.element-type | <code>[roles]<br>names=[1]</code> | compile | exit 1、names要素型不正 |
| ROLE-01.case-distinction | <code>[roles]<br>names=["Debug","debug"]</code> | `ond compile . --role Debug` | accept、選択Debug |
| ROLE-01.unknown-option-value | 既定のまま | `ond compile . --role debug` | exit 1、namesにないNAME |
| ROLE-01.repeated-option | 既定のまま | `ond compile . --role Debug --role Release` | exit 2、option重複 |
| ROLE-01.missing-option-value | 既定のまま | `ond compile . --role` | exit 2、値欠落 |
| ROLE-01.link-option | 既定のまま | `ond link --role Debug` | exit 2、未知option |

## Role選択と生成
<!-- ond-spec-id: conformance/roles/selection -->

要件`ROLE-02`と`ROLE-06`、定義元は[対象集合](../../language/v1/roles.md#対象集合と選択)と[生成](../../build/roles.md#生成と位置)である。\
次のsourceをmain.ond全文とする。

```ond
package main
var Result: i32
role Debug, Development {
    func cheat() -> i32 { return 99 }
    const Payload = [3]u8{17, 23, 31}
    var CheatData: i32 = 44
}
func probe() -> i32 {
    role Debug, Development {
        role Debug { return cheat() }
        role Development { return 2 }
    }
    role Release { return 3 }
    return 0
}
func main() { Result = probe() }
```

各行は指定roleでcompile後、MemoryVmでmainを呼びResultを比較する。\
生成比較はObjectの宣言に対応するsymbol、公開interface、global storageと初期値、定数の生成実体、debugの実行区間を対象とする。\
文字列substringの出現有無を生成判定へ用いない。

| Case ID | 指定 | 期待Result | 生成期待 |
| --- | --- | ---: | --- |
| ROLE-02.primary | 指定なし | 0 | probeの共通return 0を使い、role block由来のfunction・data・実行区間を含まない |
| ROLE-02.debug | `--role Debug` | 99 | cheatとCheatDataを含み、probeのreturn 2/3の区間を含まない |
| ROLE-02.development | `--role Development` | 2 | cheatとCheatDataを含み、probeのreturn cheat()/3の区間を含まない |
| ROLE-02.release | `--role Release` | 3 | cheat、Payload、CheatDataに由来するfunction、data、initializer、interface entryと実行区間を含まない |

`ROLE-02.empty-intersection`は上のsourceの`role Debug, Development`の宣言block内へ`role Release { func impossible(){ missing() } }`を1行追加し、role未指定の構成と各名前付きroleで独立にcompileするcaseとする。\
全compile操作をacceptし、impossibleに由来する生成対象は空となる。\
LSPではimpossible本体の`missing()`の直前で補完を要求し、対象集合が空のため宣言の候補集合が空であることを比較する。\
`ROLE-06.positions`はDebugとReleaseでprobeの選択returnのdebug位置を比較し、それぞれ原文の`return cheat()`と`return 3`の行・byte範囲に対応することを要求する。

## 全role block検査と非選択部分
<!-- ond-spec-id: conformance/roles/validation -->

要件`ROLE-03`、定義元は[構文](../../language/v1/roles.md#構文)と[検査範囲](../../language/v1/roles.md#検査範囲とimport)である。\
各行の入力を`package main`、入力欄、`func main(){}`の順にLFで結合してmain.ond全文とする。\
指定なしはrole未指定の構成とし、失敗の原因位置は表の不成立tokenまたは式に対応させる。

| Case ID | 入力 | 指定 | 期待結果 |
| --- | --- | --- | --- |
| ROLE-03.unknown-name-off | <code>role Debug { func f(){ missing() } }</code> | なし | accept |
| ROLE-03.unknown-name-on | 同上 | `--role Debug` | reject、missing |
| ROLE-03.type-off | <code>role Debug { var Wrong: bool = 1 }</code> | なし | accept |
| ROLE-03.type-on | 同上 | `--role Debug` | reject、型不一致 |
| ROLE-03.constant-off | <code>role Debug { const Wide: u8 = 256 }</code> | なし | accept |
| ROLE-03.constant-on | 同上 | `--role Debug` | reject、範囲外定数 |
| ROLE-03.syntax-off | <code>role Debug { var x: = 1 }</code> | なし | reject、型構文欠落 |
| ROLE-03.lexical-off | <code>role Debug { const x = 0xQ }</code> | なし | reject、不正数値token |
| ROLE-03.typo-off | <code>role Debug { role Debgu { func f(){} } }</code> | なし | reject、Debgu |
| ROLE-03.typo-empty | <code>role Debug { role Release { role Debgu {} } }</code> | なし | reject、Debgu |
| ROLE-03.empty-list | <code>role {}</code> | なし | reject、RoleNames欠落 |
| ROLE-03.repeated-name | <code>role Debug, Debug {}</code> | なし | reject、同一リスト内重複 |
| ROLE-03.trailing-comma | <code>role Debug, {}</code> | なし | reject、末尾comma |
| ROLE-03.package-inside | <code>role Debug { package main }</code> | なし | reject、package clause位置 |
| ROLE-03.unconfigured-syntax | <code>role Debug {}</code> | roles table省略 | reject、role定義欠落 |
| ROLE-03.contextual-name | <code>func role(){}; func call(){role()}</code> | なし | accept、roleは通常の関数binding |

## 条件付きimport
<!-- ond-spec-id: conformance/roles/imports -->

要件`ROLE-03`と`ROLE-04`、定義元は[import検査](../../language/v1/roles.md#検査範囲とimport)と[role可視性](../../language/v1/roles.md#名前解決とscope)である。\
次のmain.ondを使い、他のsourceと依存を置かない。

```ond
package main
role Debug { import "missing/tools" }
func main() {}
```

| Case ID | 操作・入力差分 | 期待結果 |
| --- | --- | --- |
| ROLE-03.import-off-primary | --role指定なしでcompile | accept、role内importを採用せずmissing/toolsを解決しない |
| ROLE-03.import-off | Releaseでcompile | accept、missing/toolsは到達package、直接import mapと循環検査に含まれない |
| ROLE-03.import-on | Debugでcompile | reject、import pathを解決できない |
| ROLE-03.import-order | main.ondのfunc mainをrole行より前へ移しReleaseでcompile | reject、Debug投影におけるimport順序違反 |

`ROLE-04.import-visibility`のmain.ond全文とtools/t.ond全文は次のとおりである。

```ond
package main
role Debug { import "game/tools" }
role Debug { func invoke() -> i32 { return tools.Value() } }
func main() {}
```

```ond
package tools
func Value() -> i32 { return 7 }
```

DebugのCoreはaccept、invokeのMemoryVm返値はi32の7となる。\
LSPではinvokeの`tools`位置でalias toolsを候補に含める。\
同じmain.ond末尾へ`func ordinary(){ tools.Value() }`を追加した独立case `ROLE-04.import-hidden`はDebugでrejectとなり、ordinary内の補完候補にもtoolsを含めない。

## 名前解決とscopeと補完
<!-- ond-spec-id: conformance/roles/visibility -->

要件`ROLE-04`と`ROLE-05`、定義元は[名前解決](../../language/v1/roles.md#名前解決とscope)と[LSP](../../language/v1/roles.md#lspの補完と診断)である。\
次のsourceをmain.ond全文とし、コメントで示した空文位置の直前をLSPカーソル位置とする。

```ond
package main
func shared() {}
role Debug { func debugOnly() {} }
role Debug, Development { func commonDev() {} }
role Debug, Development, Release { func allRoles() {} }
func main() {
    // ordinary
    ;
    role Debug {
        // debug
        ;
    }
    role Debug, Development {
        // multi
        ;
    }
    role Release {
        // release
        ;
    }
}
```

候補の比較は以下の4関数に限定し、他のbuiltin等の候補はこの表の比較対象から外す。\
初期buildはrole未指定の構成とし、次にsourceを編集せずDebug、Releaseへ順に変更して同じ比較を繰り返す。

| Case ID | 位置 | 含む候補 | 除外する候補 |
| --- | --- | --- | --- |
| ROLE-05.ordinary | ordinary | shared | debugOnly、commonDev、allRoles |
| ROLE-05.debug | debug | shared、debugOnly、commonDev、allRoles | なし |
| ROLE-05.multi | multi | shared、commonDev、allRoles | debugOnly |
| ROLE-05.release | release | shared、allRoles | debugOnly、commonDev |

Coreの各caseは既定manifestを使い、`package main`、次表の入力、`func main(){}`をLFで結合してmain.ondを作る。\
選択roleはDebugとし、別指定のある行だけ上書きする。

| Case ID | 入力 | 期待結果 |
| --- | --- | --- |
| ROLE-04.normal-hidden | <code>role Debug {func cheat(){}}; func ordinary(){cheat()}</code> | reject、ordinaryのcheatが未解決 |
| ROLE-04.all-hidden | <code>role Debug, Development, Release {func cheat(){}}; func ordinary(){cheat()}</code> | reject、全role指定でも通常文脈から不可視 |
| ROLE-04.subset | <code>role Debug, Development {func sharedRole(){}}; role Debug {func call(){sharedRole()}}</code> | accept |
| ROLE-04.not-subset | <code>role Debug {func only(){}}; role Debug, Development {func call(){only()}}</code> | reject、Cを選択Debugだけへ狭めない |
| ROLE-04.disjoint-name | <code>role Debug {func f(){}}; role Development {func f(){}}</code> | accept、各role blockのfは別宣言 |
| ROLE-04.overlap-name | <code>role Debug {func f(){}}; role Debug, Development {func f(){}}</code> | reject、実効対象集合の交差 |
| ROLE-04.no-binding-union | <code>role Debug {func f(){}}; role Development {func f(){}}; role Debug, Development {func call(){f()}}</code> | reject、異なるfを合成できない |
| ROLE-04.inherited-body | <code>role Debug {func only(){ };func call(){only()}}</code> | accept、call本体はDebug文脈 |
| ROLE-04.definition-context | <code>role Debug {func only(){}}; func generic[T](v:T){only()}; role Debug {func call(){generic[i32](1)}}</code> | reject、generic定義は通常文脈 |
| ROLE-04.local-hidden | <code>func f(){role Debug {var x:i32=1}; x=2}</code> | reject、xのblock外参照 |
| ROLE-04.local-shadow | <code>func f(){var x:i32=1; role Debug {var x:i32=2; x=3}; x=4}</code> | accept、内外のxは別binding |
| ROLE-04.common-no-override | <code>func f(){}; role Debug {func f(){}}</code> | reject、Debugでも共通宣言fが存在し、追加宣言は上書きできない |
| ROLE-04.common-duplicate-without-roles | <code>func f(){}; func f(){}</code> | roles tableと--role指定を省略してreject、共通宣言の重複 |

`ROLE-04.qualified-hidden`は次の2fileとし、Debugでcompileする。

```ond
// main.ond
package main
import "game/tools"
func main() { tools.Hidden() }
```

```ond
// tools/t.ond
package tools
role Debug { func Hidden() {} }
```

結果はmainの通常文脈からHiddenを解決できずrejectとなる。\
`ROLE-04.qualified-visible`はmainの呼出しを`role Debug { tools.Hidden() }`へ置き換えた独立caseで、acceptを期待する。

## 依存role
<!-- ond-spec-id: conformance/roles/dependencies-and-tests -->

要件`ROLE-02`と`ROLE-04`、定義元は[project境界](../../build/roles.md#project境界)、[選択](../../build/roles.md#compile時の選択)である。\
directory gameとlibraryを兄弟として置き、game/ond.tomlは既定manifestへ次のtableを追加する。

```toml
[dependencies]
support = { path = "../library" }
```

library/ond.toml全文は次のとおりとする。

```toml
[project]
name = "library"
[roles]
names = ["Internal", "Public"]
```

game/main.ond全文:

```ond
package main
import "library"
var Result: i32
func main(){ Result = library.Value[i32](7) }
```

library/lib.ond全文:

```ond
package library
func Value[T](x:T) -> i32 {
    role Internal { return 11 }
    return 22
}
```

各game compile後にMemoryVmでmainを呼びResultを比較する。

| Case ID | 操作・入力差分 | 期待結果 |
| --- | --- | --- |
| ROLE-02.dependency-primary | gameを`--role Debug`でcompile | accept、Result=22、libraryはrole未指定の構成、generic本体のInternal blockは非選択 |
| ROLE-02.dependency-independent | gameの指定をReleaseに変更 | accept、Result=22、libraryはrole未指定の構成を維持 |
| ROLE-02.dependency-added-role | gameのproject tableへfinal=true、support entryへrole="Internal"を追加しgameをDebugでcompile | accept、Result=11、libraryの選択Internal、generic本体のInternal blockを選択 |
| ROLE-02.library-option | libraryで`ond compile . --role Public` | exit 2、libraryへの指定 |

## Hashとcacheの失効
<!-- ond-spec-id: conformance/roles/cache -->

要件`ROLE-07`、定義元は[公開識別子](../../build/cache.md#公開識別子)と[role cache入力](../../build/roles.md#cache入力)である。\
既定manifestと既定sourceを用い、compiler_idを同一値に固定したhash比較では、source、compiler、Contractとgenerationを同じ入力から再構成する。\
cache操作では同じcompilerを用い、各行の最初はcacheなしでcompileし、記載の変更後に同じprocessまたはcache保存先で再操作する。

| Case ID | 操作 | 比較する期待結果 |
| --- | --- | --- |
| ROLE-07.selected-hash | 同じsourceをrole未指定・Debug・Releaseの3構成でcompile | generationが等しくてもroles.selectedがnull・Debug・Releaseと異なるため、各構成のPackageBuildInput bytesとpackage build_idが異なる |
| ROLE-07.role-input | DebugのPackageBuildInputをcanonical化 | roles fieldは`{"names":["Debug","Development","Release"],"selected":"Debug"}`、他のfieldは公開入力規則から構成 |
| ROLE-07.primary-input | 指定なしのPackageBuildInputをcanonical化 | roles fieldは`{"names":["Debug","Development","Release"],"selected":null}` |
| ROLE-07.unconfigured-hash | [HASH-01.package-empty](./package-hash.md#hash-01package-empty)を実行 | roles fieldを省略し、既定の完全入力540 bytesと指定SHA-256を維持 |
| ROLE-07.dependency-role-reset | 前節のdependency-added-roleでwarm後、gameのsupport.roleを削除 | 依存をrole未指定の構成へ戻し、本体解析とbuild_idを失効させ、Resultは11から22へ変化 |
| ROLE-07.configuration | 既定入力をwarm後、namesへShippingを追加 | 役割宣言集合を含む入力と索引を再計算し、以前の一致を前提にfrontendを省略しない |
| ROLE-07.raw-source | Releaseでcompileし、非選択のDebug role blockの空白だけを編集 | SourceHashは原文全bytesの変更を反映し、変化前のhashを再利用しない |
| ROLE-07.failure-cache | ROLE-03.unknown-name-onをDebugで解析後、source変更なしでReleaseへ切替 | Releaseはaccept、Debugの失敗診断を流用しない |
| ROLE-07.lsp-context | ROLE-05の入力でdebug、multi、ordinaryの順に補完 | 各位置はROLE-05表に一致し、前位置の候補を他文脈へ流用しない |

ROLE-07.raw-sourceの完全なmain.ond初期入力は`package main`、`role Debug { func f(){} }`、`func main(){}`の3行とし、変更は`func f`の間のspaceを2個へ増やす操作とする。\
ROLE-07.unconfigured-hashだけは参照先caseのmanifest、sourceと論理compiler_idをそのまま使う。

## Final設定と依存edgeの入力
<!-- ond-spec-id: conformance/roles/final-edge-inputs -->

要件`ROLE-08`と`ROLE-09`、定義元は[Final設定](../../build/roles.md#final設定)と[依存edgeのrole選択](../../build/roles.md#依存edgeのrole選択)である。\
この節以降のcaseはcompileを使い、runnerとchildを兄弟directoryとして置く。\
入力欄に差分を記したcaseは、以下の完全入力へその差分だけを適用した独立projectとする。\
初期状態にはcache、ond.lock、生成成果物、その他の依存を置かない。\
操作はrunnerをcurrent directoryとして`ond compile .`を実行し、受理したcaseはMemoryVmでmainを1回呼んでResultを比較する。\
拒否したcaseはexit 1と表の原因を比較し、manifestの原因位置は該当key・値または依存entryに対応させる。

runner/ond.toml全文:

```toml
[project]
name = "runner"
final = true
[dependencies]
child = { path = "../child", role = "Inspect" }
```

runner/main.ond全文:

```ond
package main
import "child"
var Result: i32
func main() { Result = child.Value[i32](7) }
```

child/ond.toml全文:

```toml
[project]
name = "child"
[roles]
names = ["Inspect"]
```

child/lib.ond全文:

```ond
package child
func Value[T](x:T) -> i32 {
    role Inspect { return helper() }
    return 10
}
func CommonApi() -> i32 { return 1 }
role Inspect {
    func helper() -> i32 { return 20 }
    func InspectApi() -> i32 { return 2 }
}
```

| Case ID | 入力差分・操作 | 仕様由来の期待結果 |
| --- | --- | --- |
| ROLE-08.final-bool | 差分なし | accept、runnerはrole未指定の構成、childはInspect、Result=20 |
| ROLE-08.final-false | runner.finalをfalse、childのrole fieldを削除 | accept、Result=10 |
| ROLE-08.final-default | runner.finalの行とchildのrole fieldを削除 | accept、final=false、Result=10 |
| ROLE-08.final-type | runner.finalをstring `"true"`へ変更 | reject、runner/ond.tomlのproject.finalの型 |
| ROLE-09.root-primary | runner/ond.toml末尾へ<code>[roles]<br>names=["Debug","Release"]</code>を追加 | accept、runnerはrole未指定の構成、childはInspect、Result=20 |
| ROLE-09.root-nonprimary | 前行と同じmanifestで`ond compile . --role Debug` | accept、runnerはDebug、childはInspect、Result=20 |
| ROLE-09.nonfinal-default-role | runner.finalをfalseへ変更 | reject、runner/ond.tomlのchild設定にroleを明示 |
| ROLE-09.nonfinal-own-primary | ROLE-09.root-primaryのrunner.finalをfalseへ変更 | reject、同じchild設定 |
| ROLE-09.nonfinal-own-nonprimary | 前行の入力で`ond compile . --role Debug` | reject、同じchild設定 |
| ROLE-09.omitted-role | runner.finalをfalse、childのrole fieldを削除 | accept、childはrole未指定の構成、Result=10 |
| ROLE-09.string-dependency | runner.finalをfalse、依存値をstring `"../child"`へ置換 | accept、childはrole未指定の構成、Result=10 |
| ROLE-09.unknown-role | childのroleを`"inspect"`へ変更 | reject、runner/ond.tomlのchild.roleは未知名 |
| ROLE-09.role-type | childのroleをinteger `1`へ変更 | reject、runner/ond.tomlのchild.roleの型 |
| ROLE-09.roleless-target | child/ond.tomlからroles tableを削除し、child/lib.ondを下記の共通sourceへ置換 | reject、runner/ond.tomlのchild.roleに対応する定義がない |
| ROLE-09.roleless-default | 前行の入力からrunnerのchild.roleを削除 | accept、childはrole未指定の構成、Result=10 |
| ROLE-09.unimported-edge | runner.finalをfalse、runner/main.ondを下記のimportなしsourceへ置換 | reject、importしていなくてもrunnerのchild設定にroleを明示 |
| ROLE-09.inactive-import | runner.finalをfalse、runnerのrolesをROLE-09.root-primaryと同じ値へ設定、runner/main.ondを下記の非選択import sourceへ置換 | reject、非選択importでも同じchild設定を検証 |

ROLE-09.roleless-targetとroleless-defaultのchild/lib.ond全文:

```ond
package child
func Value[T](x:T) -> i32 { return 10 }
```

ROLE-09.unimported-edgeのrunner/main.ond全文:

```ond
package main
func main() {}
```

ROLE-09.inactive-importのrunner/main.ond全文:

```ond
package main
role Debug { import "child" }
func main() {}
```

## Final参照の拒否
<!-- ond-spec-id: conformance/roles/final-dependency-rejection -->

要件`ROLE-08`、定義元は[登録依存graphの成立条件](../../build/roles.md#登録依存graphの成立条件)である。\
各caseは前節の完全入力を使い、記載した差分を適用する。\
childのproject tableへ`final = true`を追加し、child/sub/s.ondを次の全文で追加する。

```ond
package sub
func Read() -> i32 { return 30 }
```

| Case ID | 入力差分・操作 | 期待結果と原因位置 |
| --- | --- | --- |
| ROLE-08.final-root-reference | 上記の変更だけでrunnerをcompile | reject、runner/ond.tomlのchild entry、依存元runner・依存先child・childのfinal設定を診断 |
| ROLE-08.final-subpackage-reference | runner/main.ondのimportを`"child/sub"`、mainの代入右辺を`sub.Read()`へ置換 | reject、同じ登録edgeとfinal設定 |
| ROLE-08.final-unimported | runner/main.ondを前節のimportなしsourceへ置換 | reject、同じ登録edgeとfinal設定 |
| ROLE-08.final-alias | runner/ond.tomlの依存登録名childをalternateへ変更し、pathとsourceは維持 | reject、alternate entryと同じchildのfinal設定 |

`ROLE-08.final-git`はfinal-root-referenceのchild全fileを、絶対path `C:/ond-role-fixtures/child`のローカルGit repositoryへ配置する。\
そのrepositoryの初期状態は、当該fileを収録したcommitへtag `fixture`が解決し、working treeがcleanである状態とする。\
runner/ond.tomlの依存entryだけを次へ置き換え、ond.lockのないrunnerをcompileする。

```toml
child = { git = "file:///C:/ond-role-fixtures/child", rev = "fixture", role = "Inspect" }
```

期待はexit 1であり、Gitで解決したchildをfinalのため依存先へ登録できない旨と、runnerのchild entry・childのfinal設定を診断する。\
このcaseのGit repositoryは上記ローカル入力であり、remote serviceを初期状態に含めない。

`ROLE-08.same-project-import`は前節のrunnerとchildの完全入力から開始し、childはfinal省略のまま、runner/local/l.ondを次の全文で追加する。

```ond
package local
func Read() -> i32 { return 5 }
```

runner/main.ond全文を次へ置換してcompileする。

```ond
package main
import "child"
import "runner/local"
var Result: i32
func main() { Result = child.Value[i32](7) + local.Read() }
```

期待はaccept、childの選択Inspect、main実行後のResult=25である。

## 依存経路の整合
<!-- ond-spec-id: conformance/roles/dependency-path-agreement -->

要件`ROLE-08`と`ROLE-10`、定義元は[登録依存graphの成立条件](../../build/roles.md#登録依存graphの成立条件)である。\
この節の完全入力は、最初のfinal/edge節のchild/ond.tomlとchild/lib.ondを変更せず使い、runnerとbridgeをchildと兄弟directoryに置く。\
初期状態とcompile操作は同節とし、runnerとbridgeの全入力を以下に示す。

runner/ond.toml全文:

```toml
[project]
name = "runner"
final = true
[dependencies]
direct = { path = "../child", role = "Inspect" }
middle = { path = "../bridge" }
```

runner/main.ond全文:

```ond
package main
import "child"
import "bridge"
var Result: i32
func main() { Result = child.Value[i32](7) + bridge.Read() }
```

bridge/ond.toml全文:

```toml
[project]
name = "bridge"
[dependencies]
leaf = { path = "../child" }
```

bridge/lib.ond全文:

```ond
package bridge
import "child"
func Read() -> i32 { return child.Value[i32](1) }
```

| Case ID | 入力差分 | 期待結果 |
| --- | --- | --- |
| ROLE-10.direct-transitive-conflict | 差分なし | reject、競合project child、Inspect指定のrunner --direct→ childとrole未指定のrunner --middle→ bridge --leaf→ childの両経路を診断 |
| ROLE-10.unimported-conflict | runner/main.ondをimportなしsourceへ置換 | reject、同じproject・roleと両経路を診断 |
| ROLE-10.same-directory-alias | runnerのmiddle entryを`{ path = "../child" }`へ、main.ondをimportなしsourceへ置換 | reject、runner --direct→ childのInspect指定とrunner --middle→ childのrole未指定を同じprojectの競合として診断 |
| ROLE-10.resolved-agreement | runnerのdirect.role fieldを削除 | accept、両経路のchildはrole未指定の構成、main実行後Result=20 |
| ROLE-10.named-agreement | runnerのmiddle entryを`{ path = "../child", role = "Inspect" }`へ、runner/main.ondを最初のfinal/edge節のsourceへ置換 | accept、両登録名ともchildのInspect、main実行後Result=20 |
| ROLE-08.final-intermediate | bridgeのproject tableへ`final=true`、leaf entryへ`role="Inspect"`を追加 | reject、runnerのmiddle entryがfinal project bridgeを登録する違反、依存元runner・依存先bridge・bridge.finalを診断 |

ROLE-10.unimported-conflictとsame-directory-aliasのrunner/main.ond全文:

```ond
package main
func main() {}
```

`ROLE-10.diamond-agreement`は次の4projectを全入力とする。\
childの2fileはこの節の完全入力と同じ、bridgeの2fileも変更せず使い、otherを同じ親directoryに追加する。

runner/ond.toml全文:

```toml
[project]
name = "runner"
final = true
[dependencies]
left = { path = "../bridge" }
right = { path = "../other" }
```

runner/main.ond全文:

```ond
package main
import "bridge"
import "other"
var Result: i32
func main() { Result = bridge.Read() + other.Read() }
```

other/ond.toml全文:

```toml
[project]
name = "other"
[dependencies]
leaf = { path = "../child" }
```

other/lib.ond全文:

```ond
package other
import "child"
func Read() -> i32 { return child.Value[i32](2) }
```

期待はaccept、runner --left→ bridge --leaf→ childとrunner --right→ other --leaf→ childがともにrole未指定の構成となり、main実行後Result=20を得る。

## 依存APIとLSPとcache
<!-- ond-spec-id: conformance/roles/dependency-api-cache -->

要件`ROLE-07`と`ROLE-09`、定義元は[project境界](../../build/roles.md#project境界)、[公開hash](../../build/cache.md#公開識別子)、[LSP](../../language/v1/roles.md#lspの補完と診断)と[cache入力](../../build/roles.md#cache入力)である。\
各caseはfinal/edge節のrunnerとchildの完全入力から開始し、同じcompiler、解析sessionとcache保存先で操作列を実行する。\
初回はcoldとし、2回目に変更なしのcompileを挟んでwarmな状態を作る。\
LSPではrunner/main.ondの`child.Value`のdot直後をカーソル位置とし、比較する公開候補はValue、CommonApi、InspectApiに限定する。

| Case ID | 操作列 | 期待結果 |
| --- | --- | --- |
| ROLE-09.selected-api | 変更なしでcompile、main実行、LSP補完 | accept、childはInspect、Result=20、共通候補ValueとCommonApiに加えInspectApiを含む、Valueの具体化はInspect blockでhelperを解決 |
| ROLE-07.dependency-selected-input | 上のcompileでchildのPackageBuildInputをcanonical化 | rolesは`{"names":["Inspect"],"selected":"Inspect"}`、source hashは原文bytesから構成 |
| ROLE-07.edge-role-invalidation | warm後、runnerのchild.role fieldを削除して再compile・main実行・LSP補完 | accept、childのroles.selectedはnull、Result=10、候補ValueとCommonApiを維持しInspectApiを含まない、旧role解析・診断・索引を失効させ、childと参照元のbuild_idを再計算 |
| ROLE-07.final-invalidation | warm後、runner.finalだけfalseへ変更して再compile | exit 1、現在のchild entryに非finalのrole明示違反を診断、以前の成功成果物を保持 |
| ROLE-07.target-final-invalidation | warm後、childのproject tableへfinal=trueを追加して再compile | exit 1、runnerのchild entryとchild.finalを診断、cache一致を理由に登録graph検証を省略しない |
| ROLE-07.selected-failure-invalidation | warm後、runnerのchild.roleをUnknownへ変更してcompileし、その後role fieldを削除して再compile | Unknownはexit 1で依存entryを診断、修正後はrole未指定の構成でaccept、Result=10、過去のrole解析と失敗診断を流用しない |

上表でsourceを変更する操作はなく、各sourceのcontent_hashは原文全bytesから同じ値を得る。\
manifest変更と依存の選択roleはそれぞれ公開入力規則へ反映し、最終的な出力の比較に特定の内部cache key形式を要求しない。
