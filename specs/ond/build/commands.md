# コンパイル・リンクコマンド
<!-- ond-spec-id: build/commands-spec -->

## コマンド
<!-- ond-spec-id: build/commands -->

compileはJSONのCompilationManifestとpackage成果物を生成し、linkは[実行可能バイナリ](../artifacts.md#実行可能バイナリ)を生成する。\
保存先と永続的な配置・起動・終了設定は、選択したprojectの[compileとlink設定](./packages.md#compileとlink設定)から得る。

```sh
ond compile .
ond compile ./game -o game.ondbuild
ond link --project ./game
ond link game.ondbuild --project ./game -o game.bin
ond compile . -o - | ond link - --project . -o game.bin
```

### ond init
<!-- ond-spec-id: build/init-command -->

`ond init`

commandはcurrent directoryに`ond.toml`と`main.ond`を作る。\
manifestのproject名は編集可能な`app`とし、必須link設定とoptional設定のcomment例を含む。\
sourceは`package main`と空の`main`関数を含む。

有効なlink設定はstartup `external`、stack size 8192 byte、RAM base 4096・size 65536 byte、exit mode `loop`とする。\
数値は設定例であり、利用環境に応じて編集する。\
compile pathとlink outputには既定値、`[resolve].ignore`には既定のlist、`[project].final`には既定の`false`をcomment例で示す。\
role、ROM、dependency path、Git設定は例である。\
dependency path、Git URL、revisionは使用前に編集する。

optional ROM例は有効なRAM範囲と重ならない。\
startupと領域の成立条件、およびexit modeとaddressの条件は[compileとlink設定](./packages.md#compileとlink設定)に従う。\
例ではstartup `self`へ変更する場合にROM baseを0とし、RAM範囲を移動するよう示す。\
exit mode `jump`へ変更する場合は`loop`を置き換え、`address`を加えるよう示す。

作成先のいずれかがfile、directory、symbolic linkとして存在する場合、commandはどちらのtargetも作らず失敗する。\
後続のfile作成またはwriteが失敗した場合、この実行で作ったfileを可能な範囲で削除する。\
成功時はstdoutとstderrへ何も出力せずexit 0、file IO失敗時はstderrへ診断を出しexit 1となる。\
commandは`-h`/`--help`以外の位置引数とoptionを受理しない。

### ond compile
<!-- ond-spec-id: build/compile-command -->

`ond compile [PROJECT] [--role NAME] [-o FILE|-] [-v]`

PROJECT省略時はcurrent directoryを選び、[manifestとsource探索](./packages.md#manifestとsource探索)に従ってprojectと依存sourceを収集する。\
compileは実行projectを対象とする。

| Option/入力 | 契約 |
| --- | --- |
| `-o FILE` | CompilationManifestの出力file。設定の出力先へ一時的に優先 |
| `-o -` | CompilationManifestをstdoutへ出力 |
| 出力省略 | compile設定のoutput |
| `-v` | 再生成/再利用package名をstderrへ表示 |
| `--role NAME` | 操作対象の実行projectの選択roleを指定。指定可能な対象と省略時の選択は[role選択](./roles.md#compile時の選択)に従う |

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
入力省略によるcompileは行わない。

相対引数pathは呼出時current directory基準、設定の相対pathはond.tomlの親directory基準とする。\
ArtifactRef.pathは[file/stdin manifestの基準](../artifacts.md#manifest読出しと参照整合)に従う。\
link.outputを出力先とする。\
`-o FILE`はこの出力先へ一時的に優先し、`-o -`は起動方式に対応する実行可能バイナリ全bytesだけをstdoutへ書く。

file出力では、バイナリの拡張子を`.onddebug`へ置換したdebug sidecarを生成する。\
stdout出力ではsidecarを生成しない。\
バイナリとsidecarの出力先pathが衝突する場合は、書込み前にlink errorとする。\
sidecarのschemaと識別子は[debug](../debug-metadata/v1.md)、[公開識別子](./cache.md#公開識別子)に従う。

manifest、artifact、compiler、依存、起動計画は[参照整合](../artifacts.md#manifest読出しと参照整合)に従って検証する。\
入力Object列はmanifest.packagesの順序を保持し、linkerはstartupと、要求されたruntime helperを一度ずつ供給する。\
entry/heap/stackのruntime symbol名は[Ond規約](../artifacts.md#link-symbolの名前と所属)に従う。\
配置とrelocationは[linking](../linking.md)を定義元とする。

### 入出力と失敗時
<!-- ond-spec-id: build/io-and-failure -->

compile/linkの通常file出力は同一directoryの一時fileからrenameで公開する。\
compile/link失敗時は指定先の以前の成果物を保持する。\
ログ・診断はstderrのみ、成功exit 0、init/compile/link/file IO失敗exit 1、不正なcommand/引数exit 2とする。\
失敗結果は終了codeとstderr診断で表す。\
引数なし、`--help`/`-h`、`ond help`は使用法をstdoutへ表示して正常終了する。\
`--`以降は位置引数として扱う。

受理するoptionは各commandの構文に列挙したものとhelpだけであり、未知optionはexit 2となる。\
`--ram`、`--stack`、`--read-only`、`--return-to`、`--cache-dir`も未知optionとして扱う。\
同じoptionの重複、option値の欠落、構文を超える位置引数も引数errorとする。

`--role`の重複、値欠落とlibrary projectへの指定はexit 2、role設定不正、role定義欠落とnamesにないNAMEの指定はcompile入力のerrorとしてexit 1とする。

追加sourceを生成するbuild toolは、一意な専用directoryに[manifest](./packages.md#manifestとsource探索)を持つprojectを生成し、利用側manifestのpath依存へ登録する。\
生成sourceを含むcompileの完了後に一時directoryを削除する。\
compileとlinkをpipe接続する場合、compileのstdoutのmanifestをlinkのstdinへ渡し、linkの設定元を上記のproject選択規則で指定する。
