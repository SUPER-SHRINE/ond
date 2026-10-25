# Project role
<!-- ond-spec-id: build/roles -->

本章はOnd Language v1のproject role設定、compile時の選択とproject境界を定義する。\
sourceのrole構文、可視性と補完は[言語のrole規則](../language/v1/roles.md)に従う。

## Manifest設定
<!-- ond-spec-id: build/role-configuration -->

projectは`ond.toml`の省略可能な`[roles]` tableでrole集合を宣言する。

```toml
[roles]
names = ["Debug", "Development", "Release"]
primary = "Release"
```

| Key | 型と成立条件 |
| --- | --- |
| names | 非空のstring arrayで、各string全体が[字句規則](../language/v1/lexical.md#source形式)のidentifier token 1個の綴りと完全一致し、要素間に重複がない |
| primary | stringで、namesの要素と完全一致する |

tableを置く場合は両keyを必須とし、未知key、型不正、必須key欠落、上表の成立条件違反を設定errorとする。\
role名の同一性はUnicode符号位置列の一致で決まり、大文字小文字を区別し、正規化しない。\
`names`の順序は選択の優先度を持たず、暗黙のrole名と暗黙のprimaryは存在しない。

`[roles]`を省略したprojectはroleなしprojectである。\
そのprojectには選択roleを設けず、通常のsourceを[package契約](./packages.md)と[通常/test規則](./test-build.md)で処理する。\
roleなしprojectにおけるrole構文または`--role`の使用は、role定義の欠落を原因とするerrorとなる。

## Compile時の選択
<!-- ond-spec-id: build/role-selection -->

rolesを宣言した各projectのcompileでは、次の表により一つの選択roleを決める。

| Compile対象 | 選択role |
| --- | --- |
| 操作対象の実行project | `--role NAME`を指定した場合はNAME、未指定の場合は自身のprimary |
| 操作対象のlibrary projectのtest compile | 自身のprimary |
| 依存として利用するlibrary project | 自身のprimary |

`--role`は操作対象が実行projectである場合にだけ成立する。\
library projectのtest compileへの指定は引数errorとし、値を無視しない。\
指定NAMEが対象projectのnamesに属さない場合はcompile入力のerrorとなる。\
同一projectの各packageには同じ選択roleを適用し、通常/testのsource分類は選択roleと独立した軸として維持する。

## Project境界
<!-- ond-spec-id: build/role-project-boundary -->

role名とprimaryは宣言したprojectに属する。\
呼出元の選択roleを依存へ伝播せず、同じ綴りのroleもproject間で対応づけない。\
依存がroleなしprojectなら、その依存にはroleなしprojectの規則を適用する。

依存先のprimaryにより選択した公開宣言は、利用側からその依存の通常の公開APIとして参照できる。\
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
package成果物、生成集合、import graph、公開interface、process内意味解析、LSP索引と診断の再利用は、参照するprojectのroles設定と選択roleが一致することを成立条件とする。\
依存自身のprimaryまたはnamesの変更も、その依存とそこから得るAPIの参照環境を変更する。\
通常/test文脈とsource上のrole対象集合の異なる解析結果を同一結果として再利用せず、空の候補集合や失敗診断にも同じ規則を適用する。
