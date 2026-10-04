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

`ond.toml`は空でよい。project root自体は常に`main` packageになる。

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

`_`は値を明示的に捨てるblank identifierである。現在のOndには標準出力用の組み込み関数や標準文字列型はない。画面表示、入力、asset、device APIはtarget向けのlibraryまたはSDKから提供される。

変更したpackageを確認したい場合は`-v`を付ける。

```console
ond compile . -v
```

## linkと実行

`ond link`はコンパイルmanifestを`LinkedImage`へ変換する。RAM範囲、stack size、正常終了後の戻り先などは機種SDKが決める値であり、Ond共通の既定値はない。通常はEnbuなどのSDKを通してlink・ROM生成・起動を行う。

次の例は、リリース検証で使用する仮のmemory layoutでLinkedImageを生成する。実機向けの値ではない。

```console
ond link target/build.ondbuild --ram 4096:65536 --stack 8192 --return-to 0 -o hello.ondimage
```

成功すると`hello.ondimage`と`hello.onddebug`が作られる。後者にはsource位置、trap理由、stack trace用のunwind情報が入り、対応するVMが任意で利用する。実際のcartridgeを作る場合は、対象機種のSDKが指定する値を使用する。

低レベルに直接linkする場合の引数は次で確認できる。

```console
ond help
```

CLIの厳密な入出力とlink設定は[パッケージビルド契約](../../specs/ond/package-build.md)を参照する。

## 次に読む

- 文法と型を学ぶ: [言語ツアー](language-tour.md)
- 複数ファイルやlibraryを使う: [プロジェクトとパッケージ](projects-and-packages.md)
- 正確な規則を調べる: [言語リファレンス案内](reference.md)
