# テストビルド仕様
<!-- ond-spec-id: build/test-build -->

本書はtest buildの規範仕様を定義する。

## 1. ソースと文脈
<!-- ond-spec-id: build/test-source-context -->

対象project内のテストsource fileは、basenameが`test.ond`のfile、または`_test.ond`で終わるfileとする。\
テストビルドは通常sourceと対象project内のテストsourceの両方をコンパイル・linkする。\
importで到達しないpackage内のテストsourceも対象project内なら列挙する。\
依存project内の`test.ond`と`_test.ond`は列挙しない。\
通常ビルドはテストsourceを除外する。

テストcaseは`test.ond`または`_test.ond`内の名前が`Test`で始まるfunction declarationだけを対象とする。\
対象はtop-level functionであり、引数、戻り値、type parameter、receiverを持たないものに限る。\
`Test`で始まるfunctionまたはmethod declarationがこの形式を満たさない場合はcompile errorとする。\
`Test`で始まらないtest source内のfunctionはhelperとして扱い、test caseとして起動しない。

function以外のdeclaration名が`Test`で始まってもtest caseとは判定しない。

test-only declarationは所属package内でのみ参照でき、名前がASCII大文字で始まっても他packageへ公開しない。\
test sourceは同じpackageのprivate declarationへアクセスできるが、通常sourceからtest-only declarationを参照できない。\
別packageのtest sourceからもtest-only declarationを参照できない。\
共通helperは通常sourceのlibrary functionとして宣言する。

test sourceから通常sourceへのimportは通常の規則に従い、循環importは通常どおりcompile errorとする。\
test-only declarationのための特別な循環import解決は導入しない。

LSPは`test.ond`と`_test.ond`を無視せず、通常sourceとtest sourceの文脈を区別する。\
通常sourceからtest宣言への参照を通常の可視性として扱わず、test sourceでは同一packageのprivate declarationを解決する。

packageごとの通常解析結果とtest解析結果の参照環境、失効、保持期間は、[パッケージ意味解析キャッシュ](./semantic-cache.md)に従う。

## 2. ビルドと起動入口
<!-- ond-spec-id: build/test-entrypoints -->

テストmodeは`ond compile --test`で選び、`ond link`にはtest mode flagを設けない。\
通常compileのCompilationManifestでは`tests` fieldを省略する。\
test compileでは`tests` fieldを持ち、その値は`cases` arrayを含むobjectとする。\
caseが0件でも`tests` fieldを省略せず、`{"cases":[]}`を保持する。\
test caseごとに`package`（パッケージの論理パス）、`name`（関数名）、`symbol`（test functionのsymbol）を記録する。\
`tests` objectは`cases`だけを持ち、case objectは`package`、`name`、`symbol`だけを持つ。\
通常compileとtest compileを区別する追加mode fieldや、`tests: null`は持たない。

`ond compile --test`の既定出力は`target/test.ondbuild`とし、通常compileの既定出力`target/build.ondbuild`を維持する。\
明示的な`-o`はどちらのmodeでも既定出力より優先する。

通常buildとtest buildのcacheを混同しないよう、cache keyはcompile modeを区別する。

LinkerはCompilationManifestの`tests` fieldの有無でstartupを出し分ける。\
fieldがなければ通常startupでinitializerと`main`を呼ぶ。\
fieldがあればtest startupを生成し、test startupでは`main`を呼ばない。\
`tests` fieldがある場合、各test function symbolを呼ぶ個別wrapperをlink時に生成してguest addressを確定し、test sidecarへ記録する。\
compile manifestの`symbol`は元のtest functionを指す。\
sidecarの`entry_point`はinitializerを実行してそのtest functionだけを呼ぶwrapperを指す。

通常ビルドは全ての通常sourceを対象にし、`test.ond`と`_test.ond`を除外する。\
通常entryはpackage initializerを順に呼び、最後に`main.main`を呼ぶ。

実行projectの通常ビルドとテストビルドでは、rootの`main.ond`と`func main`を必須とする。\
library projectはrootの`main.ond`と`func main`がなくても`ond compile --test`でテストビルドできる。\
library rootにsourceがある場合は`package project名`を宣言し、rootのテストcaseもそのpackageに属する。\
rootに`main.ond`という名前のlibrary sourceがあっても、`package main`や`func main`は要求しない。\
libraryの通常compileは実行入口を要求する。\
テストビルドでは通常の`main` functionを起動しない。

テストビルドは通常sourceと対象project内のテストsource全体をbuildし、個々のテストを選ぶcompile範囲filterやpackage別binaryを設けない。\
各入口は対象testが属するpackageと必要な依存packageのinitializerを既定の順序で呼び、その後に対象test functionだけを呼ぶ。\
通常の`main` functionは呼ばない。\
initializerがfaultした場合、そのテストcaseの結果として扱う。

test modeの`LinkedImage.entry_point`は、test caseを選択して起動する必要を示すtrapで停止する。\
このentry pointから`main`、test function、initializerを実行しない。\
sidecarに列挙されたcase entry pointだけを個別起動入口として使う。

機種SDKのsection load、zero初期化、SP設定の責任境界は維持し、Ondはhost pointerを起動情報として公開しない。\
case entry pointは正常終了経路へ戻る。\
各caseの実行前に新しいmachine stateを用意する。\
initializer faultまたはcase fault/命令上限の後も実行ツールはcase結果を記録し、後続caseへ進む。

## 3. 起動情報と実行責務
<!-- ond-spec-id: build/test-execution-contract -->

test sidecarはLinkedImageと同じdirectoryに、imageの拡張子を`.ondtest`へ置換して生成する。\
例えば`game.ondimage`に対応するfileは`game.ondtest`とする。\
JSONはtop-levelの`build_id`と`cases`だけを持ち、case objectはパッケージ論理パスの`package`、test function名の`name`、guest address整数の`entry_point`だけを持つ。

```json
{
  "build_id": "<LinkedImage build_id>",
  "cases": [
    {"package": "math/vector", "name": "TestNormalize", "entry_point": 268435456}
  ]
}
```

test metadataが存在する場合、test caseが0件でも空のtest sidecarを生成する。\
通常compileのmanifestにtest metadataがない場合はtest sidecarを生成しない。\
test caseが0件の場合は`{"build_id":"<LinkedImage build_id>","cases":[]}`を出力する。\
debug sidecar`.onddebug`はtest sidecarと独立しており、file出力では通常build・test buildの両方で生成する。

`-o -`はLinkedImageだけをstdoutへ出力し、sidecarを生成しない。\
test metadataを実行ツールへ渡す必要がある利用者はfile出力を使う。\
linkのstdoutに新しいJSON wrapperは設けない。

Gakudenなどの実行ツールはLinkedImageとtest sidecarの`build_id`が一致しなければ実行を拒否する。\
一致する場合、全case、指定したパッケージの論理パスと完全一致するcase、または指定したパッケージの論理パスとfunction名の両方が完全一致するcaseを選ぶ。\
パッケージの論理パスは子packageを含めず完全一致で照合する。\
実行順、machine stateの再初期化、命令上限、faultのcase結果化、後続caseへの継続は実行ツールの責務とする。

Ondはtest functionを実行せず、console出力、assertion、合否報告、device構成も所有しない。\
これらはuser codeと実行ツールが決める。

## 4. 受入確認
<!-- ond-spec-id: build/test-build-acceptance -->

- 通常compileが`test.ond`と`_test.ond`を除外し、`target/build.ondbuild`を既定出力とする
- `--test`compileが対象project全packageのtestを列挙し、依存projectのtestを除外して`target/test.ondbuild`を既定出力とする
- 明示`-o`が通常/test両方の既定出力より優先する
- `Test` prefixのfunctionだけをcaseとし、不適切な`Test` function/method signatureをcompile errorにする
- test-only declarationは所属package内だけで可視とする
- 通常sourceや別packageからtest-only declarationを参照できない
- test sourceから同packageのprivate declarationへアクセスできる
- test sourceから通常sourceへのimportは許可し、循環importは既存規則で拒否する
- 実行projectを対象とするcompileではrootの`main.ond`と`func main`を必須とし、libraryの`--test`では不要とする
- 通常manifestで`tests` fieldが不在、test manifestで`{"cases":[]}`または複数caseとなり、追加mode fieldやnullを使わない
- 通常/test cacheを混同しない
- test manifestの`symbol`が元test functionを指し、linkが個別wrapperのguest `entry_point`を確定する
- test modeのLinkedImage entrypointが選択要求のtrapで停止し、main/test/initializerを実行しない
- test wrapperが対象packageと依存packageを初期化して対象testだけを呼ぶ
- `.ondtest`がtop-level `build_id`と`cases`だけを持つ
- `.ondtest`のcaseが`package`/`name`/`entry_point`だけを持つ
- imageとのbuild_id不一致は実行拒否される
- case 0件でも`.ondtest`を生成する
- `.onddebug`を`.ondtest`から独立してfile出力し、`-o -`はLinkedImageだけを出力する
- 実行ツールが全case、完全一致のpackage論理path、または完全一致のpackage論理pathとfunctionで選び、新しいmachine stateで実行してfault/命令上限後も継続する
- LSPがtest sourceを扱い、通常/test文脈を区別する
