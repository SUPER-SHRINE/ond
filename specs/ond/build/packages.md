# Packageとコンパイル単位
<!-- ond-spec-id: build/packages -->

## 責務
<!-- ond-spec-id: build/responsibilities -->

compilerはprojectのsourceを検査してpackage成果物を生成し、linkerは成果物とroot projectの設定から[実行可能バイナリ](../artifacts.md#実行可能バイナリ)を生成する。

| 役割 | 責務 |
| --- | --- |
| caller（呼出側） | 利用者またはビルドツールとしてcompiler/linkerを呼び出し、入力、配置値、正常終了後の制御経路を指定する |
| build tool（ビルドツール） | compiler/linkerを呼び出す。該当機能を提供する場合は追加sourceの生成、asset discovery、resource IDの管理を行う |
| loader（ローダー） | 実行可能バイナリの[ロード契約](../artifacts.md#ロード完了時の状態)を成立させてentryへ制御を渡す |
| 実行ツール | 実行環境側の方針に従ってprogramやtest入口を実行し、観測結果を扱う |
| 配布ツール | 実行可能バイナリとassetsを配布形式またはROMへまとめる |
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

省略可能な`[roles]`設定、projectごとの選択role、依存との境界は[project role契約](./roles.md)に従う。

`[dependencies]`のkeyはmanifest内の登録名であり、import pathの先頭には依存先`[project].name`を使う。\
import pathは[packageとimport](../language/v1/names-and-scopes.md#packageとimport)に従う。\
依存先を異なる登録名で参照した場合も、解決先directoryが同じなら同じ依存とする。\
同じproject名が異なるdirectoryを指す入力とsymlink依存はerrorとなる。

| Dependency値 | 解決 |
| --- | --- |
| `{ path = "../library" }` | manifest directory基準の相対pathまたは絶対path。解決先にond.tomlが必要 |
| `{ git = "https://github.com/owner/repository.git", rev = "..." }` | Git repositoryと任意のrev |
| 文字列 | URL形式ならGit、それ以外はpath |

Git依存は[Git依存lock v2](#git依存lock-v2)の規則で固定し、依存先manifestも再帰的に解決する。

### compileとlink設定
<!-- ond-spec-id: build/compile-and-link-configuration -->

project rootの`ond.toml`は、compileの保存先と[通常/test buildの実行可能バイナリ](../artifacts.md#実行可能バイナリ)の配置・起動・終了設定を次のtableで表す。\
`[compile]`は省略でき、`[link]`とその必須tableは実行可能バイナリの生成時に必要となる。\
compile単体では`[link]`を要求しない。\
設定を読む対象は操作対象のroot projectであり、依存projectのcompile/link設定を継承しない。

| Table | Key | 型・値 | 必須・省略時の値 |
| --- | --- | --- | --- |
| compile | output | 非空string。通常compileのCompilationManifest保存先 | 省略時`target/build.ondbuild` |
| compile | test-output | 非空string。test compileのCompilationManifest保存先 | 省略時`target/test.ondbuild` |
| compile | cache-dir | 非空string。package `.ondpkg`保存directory | 省略時`target/ond-cache` |
| link | output | 非空string。通常buildの実行可能バイナリ保存先 | 省略時`target/a.bin` |
| link | test-output | 非空string。test buildの実行可能バイナリ保存先 | 省略時`target/test.bin` |
| link | startup | string。`'self'`または`'external'` | 必須 |
| link | stack-size | integer。byte単位のstack予約量 | 必須 |
| link.rom | base | integer。ROMの先頭byte address | tableを置く場合は必須 |
| link.rom | size | integer。ROMのbyte容量 | tableを置く場合は必須 |
| link.ram | base | integer。RAMの先頭byte address | 必須 |
| link.ram | size | integer。RAMのbyte容量 | 必須 |
| link.exit | mode | string。`'loop'`または`'jump'` | 必須 |
| link.exit | address | integer。jumpの絶対address | modeがjumpの場合は必須、loopの場合はkeyを置かない |

`[link.rom]`の省略はstartupがexternalの場合に成立する。\
startupがselfの場合は`[link.rom]`を置き、baseを0とする。\
compile内のkeyは上表のcompile行、link直下は上表のlink行とrom・ram・exitの子tableだけとし、各子tableのkeyも対応する行の集合に限る。\
未知key、型不正、必須key欠落、未完の領域tableを設定errorとする。\
各数値はTOML integerで0〜4294967295とし、容量・alignment・領域の成立条件は[実行可能バイナリの配置契約](../artifacts.md#配置容量と出力範囲)に従う。\
ROM sizeとRAM sizeは正とし、指定領域の範囲端はu32範囲内、ROMとRAMは互いに分離する。\
ROMのbaseと範囲端は4-byte alignedとする。\
数値計算は数学的整数で行い、overflow、領域重複、容量不足はlink errorとなる。

modeがloopの場合、通常buildの`main`またはtest wrapperの対象functionから正常に戻った後の制御は、その終了処理内で反復する。\
modeがjumpの場合は、同じ正常return後、addressで指定した4-byte alignedな絶対addressへ制御を移す。\
終了先の実行可能性とその後の動作は実行環境が定める。

設定内の相対pathは、その`ond.toml`の親directoryを基準に解決する。\
project/inputの選択、`--test`によるcompile対象の選択、`-v`による表示は操作時の入力とし、`-o`による明示出力先は対応するoutput設定へ一時的に優先する。\
この一時指定は`ond.toml`を書き換えず、配置・起動・終了の永続値は上表から得る。\
command構文、設定元と入力manifestの選択は[コマンド契約](./commands.md)に従う。

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
依存sourceを含むproject全体で[role検査範囲](../language/v1/roles.md#検査範囲とimport)に従って型と名前を検査し、具体化したgeneric宣言も通常の型・関数成果物に含める。\
公開interfaceは成果物local型番号とnominal名を使い、[interface schema](../artifacts.md#公開interface)に従う。\
通常/test compileでsource集合とcache entryを区別する。

cache再利用と`frontend_ran`の意味は[cache契約](./cache.md)、実行入口とtest metadataは[test build](./test-build.md)に従う。

## Package成果物・独立compiler契約
<!-- ond-spec-id: build/package-artifact-contract -->

各package成果物、manifest、Objectの[版識別・schema](../artifacts.md#版識別と互換条件)を入力境界とする。\
[compile/link command](./commands.md)は明示した入出力と終了codeで成否を返し、[linking](../linking.md)は検証済み成果物からaddressを解決して実行可能バイナリを生成する。
