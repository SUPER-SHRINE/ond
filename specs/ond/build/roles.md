# Project role
<!-- ond-spec-id: build/roles -->

本章はOnd Language v1のproject role設定、compile時の選択とproject境界を定義する。\
sourceのrole構文、可視性と補完は[言語のrole規則](../language/v1/roles.md)に従う。

## Manifest設定
<!-- ond-spec-id: build/role-configuration -->

projectは`ond.toml`の省略可能な`[roles]` tableで名前付き追加roleの集合を宣言する。

```toml
[roles]
names = ["Debug", "Development", "Release"]
```

| Key | 型と成立条件 |
| --- | --- |
| names | 非空のstring arrayで、各string全体が[字句規則](../language/v1/lexical.md#source形式)のidentifier token 1個の綴りと完全一致し、要素間に重複がない |

tableを置く場合はnamesを必須とし、key集合はnamesだけとする。\
未知key、型不正、必須key欠落、上表の成立条件違反は設定errorとなる。\
role名の同一性はUnicode符号位置列の一致で決まり、大文字小文字を区別し、正規化しない。\
`names`の順序は選択の優先度を持たない。

role指定の有無に応じたsourceの選択は[言語の選択規則](../language/v1/roles.md#対象集合と選択)に従う。\
`[roles]`を省略したprojectの名前付きrole集合は空となり、role未指定の構成で[package契約](./packages.md)と[通常/test規則](./test-build.md)に従い処理する。

## Compile時の選択
<!-- ond-spec-id: build/role-selection -->

各projectのcompileでは、次の表によりrole指定の有無と、指定する名前付きroleを決める。

| Compile対象 | 選択role |
| --- | --- |
| 操作対象の実行project | `--role NAME`を指定した場合は宣言済みNAME、未指定の場合はrole未指定の構成 |
| 操作対象のlibrary projectのtest compile | role未指定の構成 |
| 依存として利用するlibrary project | [依存edgeのrole選択](#依存edgeのrole選択)で解決したrole |

`--role`は操作対象が実行projectである場合にだけ成立する。\
library projectのtest compileへの指定は引数errorとし、値を無視しない。\
指定NAMEが対象projectのnamesに属さない場合はcompile入力のerrorとなる。\
同一projectの各packageには同じ選択roleを適用し、通常/testのsource分類は選択roleと独立した軸として維持する。

## Final設定
<!-- ond-spec-id: build/role-final-project -->

`ond.toml`の`[project].final`は省略可能なboolであり、省略時はfalseとする。\
bool以外の値は設定errorとなる。\
finalがtrueのprojectをfinal projectと呼ぶ。

```toml
[project]
name = "game"
final = true
```

finalは、そのprojectを[依存先として登録できる条件](#登録依存graphの成立条件)と、直接依存に対する[role指定の成立条件](#依存edgeのrole選択)を定める。\
自身のroles宣言と構成の選択は本章のそれぞれの規則に従い、finalと独立に成立する。\
追加roleを宣言しないprojectもfinalをtrueにできる。\
finalの値は実行projectとlibrary projectの分類、package名と実行入口の成立条件を変更しない。

## 依存edgeのrole選択
<!-- ond-spec-id: build/role-dependency-selection -->

依存元project Pのmanifestに登録した直接依存先project Qへのedgeは、[依存の詳細形式](./packages.md#manifestとsource探索)に省略可能な`role` fieldを持てる。\
roleを置く場合はstringとし、Qがrolesを宣言し、そのnamesの要素と完全一致することを成立条件とする。\
roleの型不正とQの未宣言role名は、その依存設定を原因とするcompile入力のerrorとなる。

| 依存設定 | Qの選択 |
| --- | --- |
| roleを明示 | 指定したQのrole |
| roleを省略 | Qのrole未指定の構成 |

Pのfinalがtrueの場合、Qの宣言済み名前付きroleを明示でき、role fieldの省略も成立する。\
Pのfinalがfalseの場合はrole fieldを省略することを成立条件とする。\
この条件への違反はPの依存設定を原因とするcompile入力のerrorとなる。

PとQの選択roleおよびrole名の関係は[project境界規則](#project境界)に従う。\
Qから先の直接依存はQ自身のfinalと依存設定を使って同じ規則で解決する。

## 登録依存graphの成立条件
<!-- ond-spec-id: build/role-dependency-graph -->

compile対象rootのmanifestから再帰的に解決した登録依存graphの全edgeに、本節と[依存edgeのrole選択](#依存edgeのrole選択)の成立条件を適用する。\
projectの同一性、path・Gitの解決と名前空間は[package契約](./packages.md#manifestとsource探索)に従う。\
別の登録名も解決先が同じなら同じprojectであり、各経路で別のroleを持つprojectへ分裂させない。

各edgeの依存先はfinalがfalseであることを成立条件とする。\
final projectを依存先とする登録は、そのprojectのroot packageと全subpackageを含めて拒否する。\
path、Git、別登録名とimport pathによる同じprojectへの参照にもこの条件を適用し、違反は登録元の依存設定を原因とするcompile入力のerrorとなる。\
診断は依存元、依存先と依存先のfinal設定を特定する。\
同じproject内の通常importにはproject間edgeを設けず、[言語のpackage規則](../language/v1/names-and-scopes.md#packageとimport)を適用する。

同一buildで同じ解決projectへ至るすべての依存経路は、解決後の選択roleが一致することを成立条件とする。\
role未指定の構成同士、または同じ名前付きroleを指定した構成同士を一致とする。\
不一致はcompile入力のerrorであり、診断は競合するproject、各選択role、rootからそのprojectまでの両依存経路と、それぞれの依存設定を特定する。\
依存経路はrootからのproject列と各edgeの登録名で表し、role指定の有無と指定したrole名を示す。

以上のgraph検証はsourceのimport到達性とrole blockの選択によらず行う。\
登録したedgeをsourceからimportしない場合も、非選択のrole blockだけでimportする場合も検証対象となる。\
sourceの採用・構文検査は[既存の探索境界](./packages.md#source探索境界)に従い、本節のmanifest検証によって非採用sourceを追加の検査対象にしない。

## Project境界
<!-- ond-spec-id: build/role-project-boundary -->

名前付きroleの宣言と構成の選択は、そのprojectに属する。\
呼出元の選択roleを依存へ伝播せず、同じ綴りのroleもproject間で対応づけない。\
各依存の構成は[依存edgeの選択規則](#依存edgeのrole選択)から得る。

[依存edgeで解決したrole](#依存edgeのrole選択)により選択した公開宣言は、利用側からその依存の通常の公開APIとして参照できる。\
利用側のrole宣言に依存内部のrole名を要求しない。\
同じprojectの別packageを参照する場合は[言語のrole可視性](../language/v1/roles.md#名前解決とscope)を適用する。\
依存のgeneric宣言の本体、型、定数とその具体化は、依存自身の選択role、定義位置のrole文脈、通常/test文脈を維持し、利用側の文脈へ置換しない。

## 生成と位置
<!-- ond-spec-id: build/role-generation -->

生成対象は[role選択](../language/v1/roles.md#対象集合と選択)後の宣言、文とimportで構成する。\
非選択部分からfunction、global、型の実体、initializer、静的data、relocation、公開interface entry、test case、debugの実行区間を生成しない。\
選択部分の宣言と文は元のsource上の順序を保ち、entry成立条件と初期化順はその列へ[実行規則](../language/v1/execution.md#初期化とエントリ)を適用して求める。

診断とdebug metadataのfile、行、byte範囲は原文の位置を指す。\
role選択で除いた文字数や行数により後続source位置を詰めない。\
非選択部分の生成除外はその構文要素に由来する生成対象の除外であり、出力bytesに特定の文字列や数列が出現しないという保証は定めない。

## Cache入力
<!-- ond-spec-id: build/role-cache-input -->

role設定、選択roleと文脈を反映する公開hash入力は[PackageBuildInput](./cache.md#公開識別子)を定義元とする。\
package成果物、生成集合、import graph、公開interface、process内意味解析、LSP索引と診断の再利用は、参照するprojectのfinal・roles設定、登録依存graphの各edge設定と解決した選択roleが一致することを成立条件とする。\
依存のnames、明示roleとfinalの変更は、その依存とそこから得るAPIの参照環境へ反映する。\
cacheが一致する場合も、現在のmanifestから[登録依存graphの成立条件](#登録依存graphの成立条件)を検証し、過去の成功結果を根拠に検証を省略しない。\
通常/test文脈とsource上のrole対象集合の異なる解析結果を同一結果として再利用せず、空の候補集合や失敗診断にも同じ規則を適用する。
