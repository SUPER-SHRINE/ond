# はじめに

この章では、Ondコンパイラをbuildし、最小のプロジェクトをコンパイルする。

## コンパイラを用意する

このリポジトリのrootで次を実行する。

```console
cargo build -p ond-cli --bin ond --locked
```

生成される実行ファイルはWindowsでは`target/debug/ond.exe`、Unix系では`target/debug/ond`である。以降の例では、実行ファイルへPATHが通っているものとして`ond`と表記する。

コマンド一覧は次のように確認できる。

```console
ond help
```

## 最小のプロジェクト

空のディレクトリに、次の2ファイルを作る。

```text
hello/
├── ond.toml
└── main.ond
```

`ond.toml`にはproject名が必要である。\
名前にはOndの識別子を使う。\
実行projectのrootは`main` packageになる。

```toml
[project]
name = "hello"
```

`main.ond`にはentry functionを定義する。

```ond
package main

func main() {
}
```

project directoryでコンパイルする。

```console
ond compile .
```

成功すると`target/build.ondbuild`が作られる。これはpackage成果物とlink情報をまとめたコンパイルmanifestであり、そのまま実行するファイルではない。

## 最初の処理を書く

Ondの整数リテラルは、型が文脈から決まらない場合は`i32`になる。異なる整数型の間では暗黙変換を行わず、`as`で明示する。

```ond
package main

func sumTo(limit: u32) -> u32 {
    var total: u32 = 0
    var current: u32 = 1

    for current <= limit {
        total += current
        current++
    }

    return total
}

func main() {
    var answer: u32 = sumTo(10 as u32)
    _ = answer
}
```

`_`は値を明示的に捨てるblank identifierである。\
Ondは標準出力の組み込み関数や標準文字列型を定義しない。\
画面表示、入力、asset、device APIにはtarget向けlibraryのAPIを利用する。\
SDKは対象実行環境向けのlibraryやtoolをまとめる開発キットであり、具体的な構成は各SDKの文書で確認する。\
用語と責務の定義は[build/package仕様](../../specs/ond/build/packages.md#責務)を参照する。

変更したpackageを確認したい場合は`-v`を付ける。

```console
ond compile . -v
```

## linkと実行

`ond link`はコンパイルmanifestを`LinkedImage`へ変換する。\
呼出側はRAM範囲、stack size、正常終了後の戻り先を必須引数として、対象実行環境に合わせて明示する。\
各処理の役割は[責務の定義](../../specs/ond/build/packages.md#責務)を参照する。

例として、次の構成を持つマシン向けにlinkする場合を示す。

| 項目 | 値 |
| --- | --- |
| RAM開始address | `0x1000`（4096） |
| RAM容量 | 64 KiB（65536） |
| RAM内のstack予約量 | 8 KiB（8192） |
| 正常終了後の戻り先 | `0x0`（0） |

この例では、`0x0`に実行環境の終了処理入口があると仮定する。\
実際の値は利用する実行環境に合わせて指定する。

```console
ond link target/build.ondbuild --ram 4096:65536 --stack 8192 --return-to 0 -o hello.ondimage
```

成功すると`hello.ondimage`と`hello.onddebug`が作られる。\
後者にはsource位置、trap理由、stack trace用のunwind情報が入り、対応するVMが任意で利用する。\
`ond link`はLinkedImageを生成し、起動時はloaderが[責務の定義](../../specs/ond/build/packages.md#責務)に従ってruntime entryへ制御を渡す。\
配布ツールはLinkedImageとassetsを対象形式へまとめる。

linkの引数一覧は次で確認できる。

```console
ond help
```

CLIの厳密な入出力とlink設定は[パッケージビルド契約](../../specs/ond/build/commands.md)を参照する。

## 次に読む

- 文法と型を学ぶ: [言語ツアー](language-tour.md)
- 複数ファイルやlibraryを使う: [プロジェクトとパッケージ](projects-and-packages.md)
- 正確な規則を調べる: [言語リファレンス案内](reference.md)
