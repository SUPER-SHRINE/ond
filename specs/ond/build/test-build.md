# テストbuild
<!-- ond-spec-id: build/test-build -->

test buildのsource集合、可視環境、metadataと個別起動入口を定義する。

## Sourceと文脈
<!-- ond-spec-id: build/test-source-context -->

basenameが`test.ond`または`_test.ond`で終わるfileをtest sourceとする。\
通常buildの入力は通常source、test buildの入力は通常sourceと対象project全packageのtest sourceであり、import到達性によらず対象projectのtestを収集する。\
依存projectは通常sourceを入力として提供する。

test caseはtest source内の`Test` prefixを持つtop-level function宣言であり、signatureはparameter 0個、result 0個、型parameter 0個、receiverなしを成立条件とする。\
`Test` prefixのfunction/method宣言で条件に違反するとcompile errorとなる。\
他のfunctionはhelper、type/var/constはそれぞれの宣言として扱う。

| 解析対象 | 参照できる宣言 |
| --- | --- |
| 通常source | 自package通常source、通常import規則で公開される依存通常source |
| test source | 自package通常sourceのprivateを含む宣言、自packageのtest-only宣言、通常import規則で公開される依存通常source |

test-only宣言の可視範囲は同じpackageのtest文脈である。\
この環境に存在しない名前の参照はcompile errorとなる。\
共通helperは通常sourceのlibrary関数として宣言できる。\
test sourceのimportも[通常import規則](../language/v1/names-and-scopes.md#packageとimport)に従い、循環をcompile errorとする。\
LSPもこの通常/test参照環境を使い、packageごとの再利用と失効は[cache契約](./cache.md)に従う。

## Buildと起動入口
<!-- ond-spec-id: build/test-entrypoints -->

`ond compile --test`がtest buildを選ぶ。\
CompilationManifestの通常/test識別は`tests` fieldによって次のように定まる。

| 入力 | 意味 |
| --- | --- |
| `tests` field省略 | 通常build。通常startupはinitializer列、mainの順に実行 |
| `tests: {"cases": [...]}` | test build。各caseの個別wrapperをlink時に生成 |
| `tests: {"cases": []}` | case 0件のtest build |
| `tests: null`、未知のmode field | schema違反のerror |

tests objectのfieldは`cases`、case objectのfieldは`package`、`name`、`symbol`である。\
`package`は論理package path、`name`はtest function名、`symbol`は元test functionを指す。\
通常/testのsource集合とcompile modeをcache keyで区別する。\
test compileの既定出力は`project/target/test.ondbuild`、通常は`project/target/build.ondbuild`であり、明示`-o`が優先する。

実行projectの通常/test compileはrootの`main.ond`と`func main`を要求する。\
libraryのtest compileはlibrary package規則を使い、rootにsourceがある場合は`package project名`を宣言する。\
libraryにある`main.ond`もこのlibrary source規則に従う。\
libraryの通常compileは実行入口を要求する。

linkerはmanifestのtest caseごとに個別wrapperを生成し、そのguest addressをsidecarの`entry_point`に記録する。\
wrapperは対象testが属するpackageと依存packageを[言語の順序](../language/v1/execution.md#初期化とエントリ)で初期化し、対象test functionを1回呼び、正常終了経路へ戻る。\
test buildの入力は対象projectの全通常/test sourceであり、個別caseの選択は実行ツールが行う。

test modeの`LinkedImage.entry_point`はcase選択要求のtrapで停止する。\
個別caseの起動入口はsidecarに列挙するwrapperである。\
loaderはsection load、全globalのzero初期化とSP設定を行う。\
実行ツールは各caseの前に新しいmachine stateを用意する。\
initializer faultもそのcaseの結果として扱う。

## 起動情報と実行責務
<!-- ond-spec-id: build/test-execution-contract -->

test sidecarはLinkedImageと同じdirectoryに、imageの拡張子を`.ondtest`へ置換して生成する。\
top-level fieldは`build_id`と`cases`、case objectは`package`、`name`、guest address整数`entry_point`である。

```json
{
  "build_id": "<LinkedImage build_id>",
  "cases": [
    {"package": "math/vector", "name": "TestNormalize", "entry_point": 268435456}
  ]
}
```

file出力のtest buildはcase 0件でも`.ondtest`を生成する。\
case 0件の内容は`{"build_id":"<LinkedImage build_id>","cases":[]}`である。\
`.onddebug`は独立したsidecarとして通常/test双方のfile出力で生成する。\
`-o -`の出力はLinkedImage JSONであり、test metadataとdebug metadataを利用する場合はfile出力を選ぶ。

実行ツールはimageとtest sidecarの`build_id`一致を確認し、不一致なら実行を拒否する。\
選択方法は全case、論理package pathの完全一致、または論理package pathとfunction名双方の完全一致である。\
package選択は子packageを含むprefix検索ではなく、完全一致で照合する。\
実行順、machine stateの初期化、命令上限、fault/命令上限のcase結果化、後続caseへの継続は実行ツールが管理する。\
console、assertion、合否報告、device構成はuser codeと実行ツールが定義する。

## 受入確認
<!-- ond-spec-id: build/test-build-acceptance -->

適合検査は本章のsource分類、可視環境、case signature、schema、通常/test出力先、cache識別、wrapper初期化順、entry trapとsidecar guest addressを照合する。\
実行ツールはbuild_id、完全一致のcase選択、新しいmachine state、fault/命令上限後の結果記録と後続case継続を照合する。
