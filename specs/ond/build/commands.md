# コンパイル・リンクコマンド
<!-- ond-spec-id: build/commands-spec -->

## コマンド
<!-- ond-spec-id: build/commands -->

操作はcommand引数、入力はfileとし、linkではstdinもmanifest fileの代替に使える。\
成果物形式は[JSON/Base64 schema](../artifacts.md)に従う。

```sh
ond compile .
ond compile ./game --library-dir ./generated -o game.ondbuild
ond compile --test ./game
ond compile --test ./game -o game-test.ondbuild
ond link game.ondbuild --ram 0x10002000:0x03FFE000 --stack 1MiB --return-to 0x10000000 -o game.ondimage
ond compile . -o - | ond link - --ram 0x10002000:0x03FFE000 --stack 1MiB --return-to 0x10000000 -o game.ondimage
```

### ond compile
<!-- ond-spec-id: build/compile-command -->

`ond compile [project] [--test] [-o FILE|-] [--library-dir DIR]... [--cache-dir DIR] [-v]`

`--test`はmanifestへtest metadataを記録し、link時のtest sidecar生成を指定する。

project省略時はcurrent directoryを選び、[manifestとsource探索](./packages.md#manifestとsource探索)に従ってprojectと依存sourceを収集する。\
通常compileは実行project、`--test`は実行projectまたはlibrary projectのtest buildを選ぶ。

| Option/入力 | 契約 |
| --- | --- |
| `-o FILE` | CompilationManifestの出力file。明示値が既定値に優先 |
| `-o -` | CompilationManifestをstdoutへ出力 |
| 出力省略 | 通常はproject/target/build.ondbuild、testはproject/target/test.ondbuild |
| `--library-dir DIR` | 複数指定可能な追加package探索root |
| `OND_PATH` | OS標準path-list区切りの追加library root列。Windowsは`;`、Unix系は`:` |
| `--cache-dir DIR` | package `.ondpkg`保存先。既定はproject/target/ond-cache |
| `-v` | 再生成/再利用package名をstderrへ表示 |

`--library-dir`と`OND_PATH`を共に指定した場合は両方を用い、[同名package整合検査](./packages.md#manifestとsource探索)を適用する。\
相対引数pathは呼出時current directory基準であり、既定cache/outputはproject基準である。\
compileのsource入力はprojectのfile集合である。\
通常manifestは`tests`を省略し、test manifestはcase 0件でも`tests: {"cases":[]}`を持つ。\
schemaは[artifact契約](../artifacts.md#packageartifactとcompilationmanifest)、testのsource・entryは[test build](./test-build.md)で定義する。

### ond link
<!-- ond-spec-id: build/link-command -->

`ond link FILE|- --ram BASE:SIZE --stack SIZE --return-to ADDRESS [--read-only BASE:SIZE] [-o FILE|-]`

- FILEはcompileのmanifest
  - -ならstdinから読む
- startupはmanifestの`tests` fieldの有無から通常/testを選ぶ
- 標準出力へ出す場合の出力はLinkedImageそのもの
  - fileへ出す場合はLinkedImageと、拡張子を`.onddebug`へ置換したdebug sidecarを生成する
  - 省略時はカレントディレクトリの`a.ondimage`と`a.onddebug`
- `tests` fieldがあるmanifestではfile出力時に`.ondtest` sidecarも生成する
  - `cases`が空でもsidecarを生成する
- `.onddebug`は`.ondtest`から独立し、file出力では通常/test buildの双方で生成する
  - `.ondtest`のschemaは[テストビルド仕様](./test-build.md)に従う
- LinkedImageとdebug sidecarは同じ`build_id`を持つ
  - `build_id`は空の`build_id`を持つLinkedImageのcanonical JSON bytesのSHA-256である
- `-o -`ではpipe互換性のためLinkedImageだけをstdoutへ書き、出力を実行imageのJSONに保つ
  - sidecarを利用する実行ツールへ渡す場合はfile出力を選ぶ
- --ramはRAM領域、--stackはその内側に予約するstack量、--return-toはmain正常終了後の絶対ジャンプ先
これらは呼出側が決定する必須入力である
- --read-onlyを指定するとText/Rodataを別領域へ配置する
  - boot-bank用profileでも使用する
- 数値は10進または0x付き16進
  - KiB/MiB suffixを受理し、u32範囲外を拒否する
- entry/heap/stackのruntime symbolはOnd規約の名前へ設定する
- contract/target/ABI/依存関係/初期化順はmanifestとパッケージ成果物から読出・検証する
- compilerは共通startupを作り、要求されたruntime helperを一度だけ供給してリンクする

### 入出力と失敗時
<!-- ond-spec-id: build/io-and-failure -->

-o -はstdout。\
通常ファイル出力は同一ディレクトリの一時ファイルからrenameで公開する。\
compile/link失敗時は指定先の以前の成果物を保持する。\
ログ・診断はstderrのみ。\
成功exit 0、コンパイル／リンク／ファイルIO失敗exit 1、\
不正なコマンド／引数exit 2。\
失敗結果は終了codeとstderr診断で表す。\
引数なし、--help/-h、ond helpは使用法をstdoutに表示して正常終了する。\
--以降は位置引数として扱う。

追加sourceを生成するbuild toolは、一意な専用一時ディレクトリへsourceを生成し、--library-dirで渡す。\
専用directoryにsourceを生成して利用者のsourceと分ける。\
生成sourceを含むcompileの完了後に一時ディレクトリを削除する。\
compileとlinkをpipe接続するbuild toolはcompileのstdoutからmanifestを受け取り、そのmanifestだけをlinkのstdinに渡す。
