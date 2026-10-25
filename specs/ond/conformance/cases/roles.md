# Roleの入力と期待結果
<!-- ond-spec-id: conformance/cases/roles -->

要件は`ROLE-01`〜`ROLE-07`とし、各節で対応する規範と観測操作を指定する。\
適用版はOnd Language v1、Contract revision 6、target kagura-v1、ABI ond-kagura-v1である。\
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
primary = "Release"
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
| ROLE-01.default | 既定のまま | compile | accept、選択Release |
| ROLE-01.override | 既定のまま | `ond compile . --role Debug` | accept、選択Debug |
| ROLE-01.unconfigured | 省略 | compile | accept、選択roleなし |
| ROLE-01.unconfigured-option | 省略 | `ond compile . --role Debug` | exit 1、role定義欠落 |
| ROLE-01.empty | <code>[roles]<br>names=[]<br>primary="Release"</code> | compile | exit 1、非空条件違反 |
| ROLE-01.duplicate | <code>[roles]<br>names=["Debug","Debug"]<br>primary="Debug"</code> | compile | exit 1、names重複 |
| ROLE-01.no-primary | <code>[roles]<br>names=["Debug"]</code> | compile | exit 1、primary欠落 |
| ROLE-01.no-names | <code>[roles]<br>primary="Debug"</code> | compile | exit 1、names欠落 |
| ROLE-01.primary-outside | <code>[roles]<br>names=["Debug"]<br>primary="Release"</code> | compile | exit 1、primaryがnamesの要素でない |
| ROLE-01.invalid-identifier | <code>[roles]<br>names=["Debug-build"]<br>primary="Debug-build"</code> | compile | exit 1、identifier不成立 |
| ROLE-01.identifier-whitespace | <code>[roles]<br>names=[" Debug "]<br>primary=" Debug "</code> | compile | exit 1、string全体がidentifierの綴りと一致しない |
| ROLE-01.keyword | <code>[roles]<br>names=["if"]<br>primary="if"</code> | compile | exit 1、keywordはidentifier tokenでない |
| ROLE-01.unknown-key | <code>[roles]<br>names=["Debug"]<br>primary="Debug"<br>default="Debug"</code> | compile | exit 1、未知key |
| ROLE-01.names-type | <code>[roles]<br>names="Debug"<br>primary="Debug"</code> | compile | exit 1、names型不正 |
| ROLE-01.element-type | <code>[roles]<br>names=[1]<br>primary="Debug"</code> | compile | exit 1、names要素型不正 |
| ROLE-01.primary-type | <code>[roles]<br>names=["Debug"]<br>primary=1</code> | compile | exit 1、primary型不正 |
| ROLE-01.case-distinction | <code>[roles]<br>names=["Debug","debug"]<br>primary="debug"</code> | `ond compile . --role Debug` | accept、選択Debug |
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
        role Debug { return cheat() } else { return 2 }
    } else {
        return 3
    }
}
func main() { Result = probe() }
```

各行は指定roleでcompile後、MemoryVmでmainを呼びResultを比較する。\
生成比較はObjectの宣言に対応するsymbol、公開interface、global storageと初期値、定数の生成実体、debugの実行区間を対象とする。\
文字列substringの出現有無を生成判定へ用いない。

| Case ID | 指定 | 期待Result | 生成期待 |
| --- | --- | ---: | --- |
| ROLE-02.debug | `--role Debug` | 99 | cheatとCheatDataを含み、probeのelse return 2/3の区間を含まない |
| ROLE-02.development | `--role Development` | 2 | cheatとCheatDataを含み、probeのreturn cheat()/3の区間を含まない |
| ROLE-02.release | 指定なし | 3 | cheat、Payload、CheatDataに由来するfunction、data、initializer、interface entryと実行区間を含まない |

`ROLE-02.empty-intersection`は上のsourceの`role Debug, Development`の宣言block内へ`role Release { func impossible(){ missing() } }`を1行追加し、各roleで独立にcompileするcaseとする。\
全compile操作をacceptし、impossibleに由来する生成対象は空となる。\
LSPではimpossible本体の`missing()`の直前で補完を要求し、対象集合が空のため宣言の候補集合が空であることを比較する。\
`ROLE-06.positions`はDebugとReleaseでprobeの選択returnのdebug位置を比較し、それぞれ原文の`return cheat()`と`return 3`の行・byte範囲に対応することを要求する。

## 全branch検査と非選択部分
<!-- ond-spec-id: conformance/roles/validation -->

要件`ROLE-03`、定義元は[構文](../../language/v1/roles.md#構文)と[検査範囲](../../language/v1/roles.md#検査範囲とimport)である。\
各行の入力を`package main`、入力欄、`func main(){}`の順にLFで結合してmain.ond全文とする。\
指定なしはReleaseを選び、失敗の原因位置は表の不成立tokenまたは式に対応させる。

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
    } else {
        // otherwise
        ;
    }
}
```

候補の比較は以下の4関数に限定し、他のbuiltin等の候補はこの表の比較対象から外す。\
初期buildの選択はReleaseとし、次にsourceを編集せずDebugへ変更して同じ比較を繰り返す。

| Case ID | 位置 | 含む候補 | 除外する候補 |
| --- | --- | --- | --- |
| ROLE-05.ordinary | ordinary | shared | debugOnly、commonDev、allRoles |
| ROLE-05.debug | debug | shared、debugOnly、commonDev、allRoles | なし |
| ROLE-05.multi | multi | shared、commonDev、allRoles | debugOnly |
| ROLE-05.else | otherwise | shared、allRoles | debugOnly、commonDev |

Coreの各caseは既定manifestを使い、`package main`、次表の入力、`func main(){}`をLFで結合してmain.ondを作る。\
選択roleはDebugとし、別指定のある行だけ上書きする。

| Case ID | 入力 | 期待結果 |
| --- | --- | --- |
| ROLE-04.normal-hidden | <code>role Debug {func cheat(){}}; func ordinary(){cheat()}</code> | reject、ordinaryのcheatが未解決 |
| ROLE-04.all-hidden | <code>role Debug, Development, Release {func cheat(){}}; func ordinary(){cheat()}</code> | reject、全role指定でも通常文脈から不可視 |
| ROLE-04.subset | <code>role Debug, Development {func sharedRole(){}}; role Debug {func call(){sharedRole()}}</code> | accept |
| ROLE-04.not-subset | <code>role Debug {func only(){}}; role Debug, Development {func call(){only()}}</code> | reject、Cを選択Debugだけへ狭めない |
| ROLE-04.disjoint-name | <code>role Debug {func f(){}} else {func f(){}}</code> | accept、各branchのfは別宣言 |
| ROLE-04.overlap-name | <code>role Debug {func f(){}}; role Debug, Development {func f(){}}</code> | reject、実効対象集合の交差 |
| ROLE-04.no-binding-union | <code>role Debug {func f(){}}; role Development {func f(){}}; role Debug, Development {func call(){f()}}</code> | reject、異なるfを合成できない |
| ROLE-04.inherited-body | <code>role Debug {func only(){ };func call(){only()}}</code> | accept、call本体はDebug文脈 |
| ROLE-04.definition-context | <code>role Debug {func only(){}}; func generic[T](v:T){only()}; role Debug {func call(){generic[i32](1)}}</code> | reject、generic定義は通常文脈 |
| ROLE-04.local-hidden | <code>func f(){role Debug {var x:i32=1}; x=2}</code> | reject、xのblock外参照 |
| ROLE-04.local-shadow | <code>func f(){var x:i32=1; role Debug {var x:i32=2; x=3}; x=4}</code> | accept、内外のxは別binding |

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

## 依存primaryとtest軸
<!-- ond-spec-id: conformance/roles/dependencies-and-tests -->

要件`ROLE-02`と`ROLE-04`、定義元は[project境界](../../build/roles.md#project境界)、[選択](../../build/roles.md#compile時の選択)と[test build](../../build/test-build.md)である。\
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
primary = "Public"
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
role Internal {
    func Value[T](x:T) -> i32 { return 11 }
} else {
    func helper() -> i32 { return 22 }
    func Value[T](x:T) -> i32 { return helper() }
}
```

各game compile後にMemoryVmでmainを呼びResultを比較する。

| Case ID | 操作・入力差分 | 期待結果 |
| --- | --- | --- |
| ROLE-02.dependency-primary | gameを`--role Debug`でcompile | accept、Result=22、libraryの選択Public、generic本体はPublic文脈 |
| ROLE-02.dependency-independent | gameの指定をReleaseに変更 | accept、Result=22、gameにPublicを宣言する必要なし |
| ROLE-02.dependency-change | libraryのprimaryだけInternalへ変更しgameをDebugでcompile | accept、Result=11、libraryとgameの参照環境・build_idを再計算 |
| ROLE-02.library-option | libraryで`ond compile --test . --role Public` | exit 2、libraryへの指定 |

test軸のcaseは独立projectとし、libraryの上のmanifest、空のlib.ond（`package library`だけ）と次のlib_test.ondを使う。

```ond
package library
role Internal { func TestInternal(){} } else { func TestPublic(){} }
```

`ROLE-02.library-test`はlibraryで`ond compile --test .`を実行し、acceptとtest metadataのcaseがTestPublicだけであることを比較する。\
`ROLE-05.test-visibility`はlib_test.ondのPublic branchへ`func testHelper(){}`を追加し、lib.ondを次の全文へ置換する。

```ond
package library
role Public { func ordinary(){ testHelper() } }
```

test compileは通常sourceからtest-only名を参照できないためrejectし、ordinaryの同位置におけるLSP候補にもtestHelperを含めない。\
test source内のPublic branchではtestHelperを候補に含める。

## Hashとcacheの失効
<!-- ond-spec-id: conformance/roles/cache -->

要件`ROLE-07`、定義元は[公開識別子](../../build/cache.md#公開識別子)と[role cache入力](../../build/roles.md#cache入力)である。\
既定manifestと既定sourceを用い、compiler_idを同一値に固定したhash比較では、source、compiler、Contractとgenerationを同じ入力から再構成する。\
cache操作では同じcompilerを用い、各行の最初はcacheなしでcompileし、記載の変更後に同じprocessまたはcache保存先で再操作する。

| Case ID | 操作 | 比較する期待結果 |
| --- | --- | --- |
| ROLE-07.selected-hash | 同じsourceをDebugとReleaseでcompile | generationが等しくてもroles.selectedが異なるためPackageBuildInput bytesとpackage build_idが異なる |
| ROLE-07.role-input | DebugのPackageBuildInputをcanonical化 | roles fieldは`{"names":["Debug","Development","Release"],"primary":"Release","selected":"Debug"}`、他のfieldは公開入力規則から構成 |
| ROLE-07.unconfigured-hash | [HASH-01.package-empty](./package-hash.md#hash-01package-empty)を実行 | roles fieldを省略し、既定の完全入力556 bytesと指定SHA-256を維持 |
| ROLE-07.dependency-primary | 前節のdependency-primaryでwarm後、library.primaryだけInternalへ変更 | 依存API、本体解析とbuild_idを失効させ、Resultは22から11へ変化 |
| ROLE-07.configuration | 既定入力をwarm後、namesへShippingを追加 | 役割宣言集合を含む入力と索引を再計算し、以前の一致を前提にfrontendを省略しない |
| ROLE-07.raw-source | Releaseでcompileし、非選択Debug branchの空白だけを編集 | SourceHashは原文全bytesの変更を反映し、変化前のhashを再利用しない |
| ROLE-07.failure-cache | ROLE-03.unknown-name-onをDebugで解析後、source変更なしでReleaseへ切替 | Releaseはaccept、Debugの失敗診断を流用しない |
| ROLE-07.lsp-context | ROLE-05の入力でdebug、multi、ordinaryの順に補完 | 各位置はROLE-05表に一致し、前位置の候補を他文脈へ流用しない |

ROLE-07.raw-sourceの完全なmain.ond初期入力は`package main`、`role Debug { func f(){} }`、`func main(){}`の3行とし、変更は`func f`の間のspaceを2個へ増やす操作とする。\
ROLE-07.unconfigured-hashだけは参照先caseのmanifest、sourceと論理compiler_idをそのまま使う。
