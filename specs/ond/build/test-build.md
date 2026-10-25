# テストbuild
<!-- ond-spec-id: build/test-build -->

test buildのsource集合、可視環境、生成metadataと個別起動入口を定義する。\
testの実行方法と合否判定は実行環境の責務である。

## Sourceと文脈
<!-- ond-spec-id: build/test-source-context -->

basenameが`test.ond`または`_test.ond`で終わるfileをtest sourceとする。\
通常buildの入力は通常source、test buildの入力は通常sourceと対象project全packageのtest sourceであり、import到達性によらず対象projectのtestを収集する。\
依存projectは通常sourceを入力として提供する。

source集合内の宣言、文、importとtest caseは[role規則](../language/v1/roles.md)で選択し、操作対象と依存それぞれの選択roleは[compile時の選択](./roles.md#compile時の選択)に従う。\
本章の通常/test可視性はrole可視性と同時に適用する。

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

metadataとsidecarの型は次の表で定義し、[JSON wire形式](../artifacts.md#json-wire形式)を適用する。

| 型 | Field |
| --- | --- |
| TestMetadata | `cases: TestCase[]` |
| TestCase | `package: string`, `name: string`, `symbol: string` |
| TestSidecar | `build_id: string`, `cases: TestEntry[]` |
| TestEntry | `package: string`, `name: string`, `entry_point: u32` |

packageは論理package path、nameはtest function名、symbolは元function、entry_pointはwrapperのguest addressとする。\
通常/testのsource集合とcompile modeをcache keyで区別する。\
通常/test compileの出力先と明示`-o`の優先関係は、[compile設定](./packages.md#compileとlink設定)に従う。

実行projectの通常/test compileはrootの`main.ond`と`func main`を要求する。\
libraryのtest compileはlibrary package規則を使い、rootにsourceがある場合は`package project名`を宣言する。\
libraryにある`main.ond`もこのlibrary source規則に従う。\
libraryの通常compileは実行入口を要求する。

linkerはmanifestのtest caseごとに個別wrapperを生成し、そのguest addressをsidecarの`entry_point`に記録する。\
wrapperは対象testが属するpackageと依存packageを[言語の順序](../language/v1/execution.md#初期化とエントリ)で初期化し、対象test functionを1回呼び、正常return時は[link.exit](./packages.md#compileとlink設定)で指定した終了経路へ制御を移す。\
test buildの入力は対象projectの全通常/test sourceである。

test buildも通常buildと同じ[ONDX形式](../artifacts.md#標準実行バイナリ)とlink.startupのself/externalを使う。\
ヘッダのentry_addressはcase選択要求のtrapで停止し、個別caseを自動実行しない。\
個別caseの起動入口はsidecarに列挙するwrapperである。\
このtrapはcase選択を要求するguardであり、testの成功を通知しない。\
各wrapperの成立前提は、[ロード契約](../artifacts.md#ロード完了時の状態)のsection byte列・zero初期化とSP初期値を満たすmemory/register状態とする。\
対象package・依存packageのglobalとruntime storageはそのロード初期値を持ち、言語initializerの適用前の状態にある。\
この状態から、共通ヘッダのentry_addressに代えて選択したwrapperのentry_pointへ制御を渡す。\
selfの前置起動処理も共通ヘッダのentryへ制御を渡すため、resetだけで個別caseの実行を開始しない。

## 起動情報と実行責務
<!-- ond-spec-id: build/test-execution-contract -->

test sidecarの保存とstdout出力の扱いは[link command](./commands.md#ond-link)に従う。\
内容は上のTestSidecar型とする。

以下のJSONと後続の空case表現では、`<binary build_id>`を対象バイナリ全bytesの[64桁識別子](./cache.md#公開識別子)へ置換して内容を構成する。

```json
{
  "build_id": "<binary build_id>",
  "cases": [
    {"package": "math/vector", "name": "TestNormalize", "entry_point": 268435456}
  ]
}
```

file出力のtest buildはcase 0件でも`.ondtest`を生成する。\
case 0件の内容は`{"build_id":"<binary build_id>","cases":[]}`である。\
`.onddebug`は独立したsidecarとして通常/test双方のfile出力で生成する。\
通常/testの出力先は[compileとlink設定](./packages.md#compileとlink設定)に従う。

sidecarの識別子照合と不一致時の使用拒否は[公開識別子](./cache.md#公開識別子)に従う。\
個別入口の選択方法、実行順、machine stateの準備方法、命令上限、faultや正常returnの解釈、後続caseへの継続、assertion、console、合否報告、device構成はuser codeと実行環境が定める。\
本章が提供するcase一覧とwrapperの生成契約は、これらの実行方針に依存しない。

## 受入確認
<!-- ond-spec-id: build/test-build-acceptance -->

適合検査は本章のsource分類、可視環境、case signature、schema、通常/test出力先、cache識別、wrapper初期化順、共通entryのguardとsidecar guest addressを照合する。\
wrapperの実行を観測する適合caseは、初期state、操作、観測点と比較方法をそのcaseで指定する。\
この適合検査の手順は、利用者のtest実行方法や合否判定を定める契約へ追加しない。
