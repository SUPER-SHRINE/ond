# Packageとコンパイル単位
<!-- ond-spec-id: build/packages -->

## 責務
<!-- ond-spec-id: build/responsibilities -->

compilerはprojectのsourceを検査してpackage成果物を生成し、linkerは成果物と呼出側が指定する[LinkPlan](../artifacts.md#linkplanとlinkedimage)からLinkedImageを生成する。

| 役割 | 責務 |
| --- | --- |
| caller（呼出側） | 利用者またはビルドツールとしてcompiler/linkerを呼び出し、入力、配置値、正常終了後の制御経路を指定する |
| build tool（ビルドツール） | compiler/linkerを呼び出す。該当機能を提供する場合は追加sourceの生成、asset discovery、resource IDの管理を行う |
| loader（ローダー） | LinkedImageのsectionをloadし、data末尾から各sectionのmemory_sizeまでをzero初期化し、SPを設定して通常runtime entryへ制御を渡す。test wrapperの起動契約は[test build](./test-build.md)に従う |
| 実行ツール | 通常programを実行する。test時はcaseを選択し、machine state、命令上限、case結果を管理する |
| 配布ツール | LinkedImageとassetsを配布形式またはROMへまとめる |
| 実行環境 | 実機またはVMとしてmemory map、device、正常終了後の制御経路などを構成する |

SDK（Software Development Kit）は、特定の実行環境向けにライブラリ・ツール・文書をまとめた開発キットである。\
各SDKが含める機能は、そのkitの文書が定める。\
上記の役割は同じtoolが複数を兼ねられる。\
OndはCLI/APIから直接利用できる。\
sourceのpackage/import/公開名は[言語規則](../language/v1/names-and-scopes.md)、成果物fieldは[artifact契約](../artifacts.md)を定義元とする。

## Manifestとsource探索
<!-- ond-spec-id: build/manifest-and-source-discovery -->

`ond.toml`があるdirectoryをproject rootとし、全projectが`[project].name`へOndの有効なidentifierを宣言する。\
実行projectのroot packageとentryは[言語規則](../language/v1/execution.md#初期化とエントリ)に従う。\
library rootのpackage構成は[package規則](../language/v1/names-and-scopes.md#packageとimport)に従い、libraryの編集・依存利用とtest compileはこのpackageを扱う。

`[dependencies]`のkeyはmanifest内の登録名であり、import pathの先頭には依存先`[project].name`を使う。\
import pathは[packageとimport](../language/v1/names-and-scopes.md#packageとimport)に従う。\
依存先を異なる登録名で参照した場合も、解決先directoryが同じなら同じ依存とする。\
同じproject名が異なるdirectoryを指す入力とsymlink依存はerrorとなる。

| Dependency値 | 解決 |
| --- | --- |
| `{ path = "../library" }` | manifest directory基準の相対pathまたは絶対path。解決先にond.tomlが必要 |
| `{ git = "https://github.com/owner/repository.git", rev = "..." }` | Git repositoryと任意のrev |
| 文字列 | URL形式ならGit、それ以外はpath |

Git依存は次節のlock規則で固定し、依存先manifestも再帰的に解決する。

### Git依存lock v2
<!-- ond-spec-id: build/git-lock-v2 -->

実行対象projectの`ond.lock`はUTF-8 TOMLとし、top-level fieldは`version: integer = 2`と`package: LockedGit[]`だけとする。\
packageは`[[package]]`のarray of tablesであり、各entryは次のfieldを持つ閉schemaとする。

| Field | 型・成立条件 |
| --- | --- |
| owner | string。依存を宣言したprojectの[project].name |
| name | string。そのmanifestの[dependencies]登録名 |
| git | string。manifestのGit URLをdecodeした文字列と完全一致 |
| rev | 省略可能な非空string。指定時はmanifestの要求revと完全一致 |
| commit | 完全な小文字hexのcommit object ID。SHA-1 repositoryは40桁、SHA-256 repositoryは64桁 |

rev以外は必須で、rev省略はmanifestのrev省略に対応する。\
URLとrevの比較で文字列正規化を行わず、entryのidentityは(owner,name)の組とする。\
各identityは一意で、保存順はowner/nameのUnicode scalar辞書順とする。\
path依存はlockへ収録せず、依存graph全体のGit edgeだけを収録する。\
Git edgeが0件の保存内容は`version = 2`と`package = []`の2fieldとする。

lockがない初回は、rev指定時はそのrevision、rev省略時はremote default branchのHEADをcommitへ解決する。\
依存graph全体の解決に成功した後、同じdirectory内の一時fileの書込み完了後renameしてlockを保存する。\
Git checkoutの保存先はprojectの`.ond/dependencies`とする。

lockがある場合は必要な全Git edgeについてowner/name、git、revの有無と値が一致し、commitが当該repositoryのcommitとして解決できることを要求する。\
常に記録commitを使用し、version不一致、未知field、field型不正、identity重複、必要entry欠落、余剰entry、commit不正・読出不能はerrorとなる。\
更新する操作は既存lockを削除した状態での再解決とする。

### Source探索境界
<!-- ond-spec-id: build/source-discovery-boundaries -->

source探索はproject境界とignore設定に従う。\
`[resolve].ignore`はmanifest directoryからの相対directory列であり、既定値は`[".git", ".ond", "target"]`である。\
absolute path、`..`、globを含むignore入力はerrorとなる。\
`.ond/`は常にsource探索から除き、子directoryの別のond.tomlで探索を区切る。\
依存先は自身のignore設定に従う。

依存sourceは各依存projectのnamespaceで収集し、import文字列をdecodeして到達するpackageを採用する。

## コンパイル単位と成果物
<!-- ond-spec-id: build/implemented-units-and-constraints -->

1 packageにつき1個のUTF-8 JSON `.ondpkg`を生成し、CompilationManifestが各fileを参照する。\
section dataはBase64文字列、他のfieldは通常のJSONである。\
依存sourceを含むproject全体で型と名前を検査し、具体化したgeneric宣言も通常の型・関数成果物に含める。\
公開interfaceは成果物local型番号とnominal名を使い、[interface schema](../artifacts.md#公開interface)に従う。\
通常/test compileでsource集合とcache entryを区別する。

cache再利用と`frontend_ran`の意味は[cache契約](./cache.md)、実行入口とtest metadataは[test build](./test-build.md)に従う。

## Package成果物・独立compiler契約
<!-- ond-spec-id: build/package-artifact-contract -->

各package成果物、manifest、ObjectとLinkedImageの[版識別・schema](../artifacts.md#版識別と互換条件)を入力境界とする。\
[compile/link command](./commands.md)は明示した入出力と終了codeで成否を返し、[linking](../linking.md)は検証済み成果物から解決済みimageを作る。
