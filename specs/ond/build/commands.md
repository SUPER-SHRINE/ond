# コンパイル・リンクコマンド
<!-- ond-spec-id: build/commands-spec -->

## コマンド
<!-- ond-spec-id: build/commands -->

compileはJSONのCompilationManifestとpackage成果物を生成し、linkは[実行可能バイナリ](../artifacts.md#実行可能バイナリ)を生成する。\
保存先と永続的な配置・起動・終了設定は、選択したprojectの[compileとlink設定](./packages.md#compileとlink設定)から得る。

```sh
ond compile .
ond compile ./game -o game.ondbuild
ond compile --test ./game
ond link --project ./game
ond link game.ondbuild --project ./game -o game.bin
ond compile . -o - | ond link - --project . -o game.bin
```

### ond compile
<!-- ond-spec-id: build/compile-command -->

`ond compile [PROJECT] [--test] [-o FILE|-] [-v]`

PROJECT省略時はcurrent directoryを選び、[manifestとsource探索](./packages.md#manifestとsource探索)に従ってprojectと依存sourceを収集する。\
通常compileは実行project、`--test`は実行projectまたはlibrary projectのtest buildを選ぶ。\
`--test`はmanifestと各artifactへ[test metadata](./test-build.md#buildと起動入口)を記録し、case 0件でも通常buildと区別する。

| Option/入力 | 契約 |
| --- | --- |
| `-o FILE` | CompilationManifestの出力file。設定の通常/test出力先へ一時的に優先 |
| `-o -` | CompilationManifestをstdoutへ出力 |
| 出力省略 | compile設定のoutputまたはtest-output |
| `-v` | 再生成/再利用package名をstderrへ表示 |

package `.ondpkg`の保存先はcompile.cache-dirとする。\
相対引数pathは呼出時current directory基準、設定の相対pathはond.tomlの親directory基準である。\
source入力はprojectのfile集合、成果物schemaは[artifact契約](../artifacts.md#packageartifactとcompilationmanifest)に従う。

### ond link
<!-- ond-spec-id: build/link-command -->

`ond link [FILE|-] [--project DIR] [-o FILE|-]`

`--project DIR`指定時はDIR直下、未指定時はcurrent directory直下の`ond.toml`を設定元とする。\
上位directoryや入力manifestの保存先から設定元を探索せず、そのond.tomlを読めない場合はlink errorとなる。\
FILE指定時はそのJSON CompilationManifestを読み、`-`指定時はstdinから読む。\
入力省略時は選択projectのcompile.outputを読む。\
入力省略によるcompileは行わず、test manifestを入力とする場合はFILEまたはstdinを明示する。

相対引数pathは呼出時current directory基準、設定の相対pathはond.tomlの親directory基準とする。\
ArtifactRef.pathは[file/stdin manifestの基準](../artifacts.md#manifest読出しと参照整合)に従う。\
通常/testはmanifestのtests fieldの有無から判定し、通常はlink.output、testはlink.test-outputを出力先とする。\
`-o FILE`はこの出力先へ一時的に優先し、`-o -`は起動方式に対応する実行可能バイナリ全bytesだけをstdoutへ書く。

file出力では、バイナリの拡張子を`.onddebug`へ置換したdebug sidecarを通常/test双方で生成し、test buildではさらに`.ondtest`を生成する。\
case 0件でもtest sidecarを生成する。\
stdout出力ではsidecarを生成しない。\
バイナリとsidecarの出力先pathが衝突する場合は、書込み前にlink errorとする。\
sidecarのschemaと識別子は[debug](../debug-metadata/v1.md)、[test build](./test-build.md)、[公開識別子](./cache.md#公開識別子)に従う。

manifest、artifact、compiler、依存、通常/test起動計画は[参照整合](../artifacts.md#manifest読出しと参照整合)に従って検証する。\
入力Object列はmanifest.packagesの順序を保持し、linkerは通常startupまたはtest wrapperと、要求されたruntime helperを一度ずつ供給する。\
entry/heap/stackのruntime symbol名は[Ond規約](../artifacts.md#link-symbolの名前と所属)に従う。\
配置とrelocationは[linking](../linking.md)を定義元とする。

### 入出力と失敗時
<!-- ond-spec-id: build/io-and-failure -->

通常ファイル出力は同一directoryの一時fileからrenameで公開する。\
compile/link失敗時は指定先の以前の成果物を保持する。\
ログ・診断はstderrのみ、成功exit 0、compile/link/file IO失敗exit 1、不正なcommand/引数exit 2とする。\
失敗結果は終了codeとstderr診断で表す。\
引数なし、`--help`/`-h`、`ond help`は使用法をstdoutへ表示して正常終了する。\
`--`以降は位置引数として扱う。

受理するoptionは各commandの構文に列挙したものとhelpだけであり、未知optionはexit 2となる。\
`--ram`、`--stack`、`--read-only`、`--return-to`、`--cache-dir`も未知optionとして扱う。\
同じoptionの重複、option値の欠落、構文を超える位置引数も引数errorとする。

追加sourceを生成するbuild toolは、一意な専用directoryに[manifest](./packages.md#manifestとsource探索)を持つprojectを生成し、利用側manifestのpath依存へ登録する。\
生成sourceを含むcompileの完了後に一時directoryを削除する。\
compileとlinkをpipe接続する場合、compileのstdoutのmanifestをlinkのstdinへ渡し、linkの設定元を上記のproject選択規則で指定する。
