# セミコロン

セミコロンは文を明示的に区切ります。\
\
改行やファイル末尾では、条件を満たすtokenの後に文末が自動挿入されます。

```ond title="main.ond"
package main

func main() {
    var first: u32 = 1; var second: u32 = 2
    _ = first + second
}
```

## 同じ行の文を分ける

同じ行に複数の文を書く場合は、`;`で区切れます。

## 改行で区切る

通常は一つの文を一行に書き、改行による文末挿入を使えます。\
\
`)`と`}`の直前ではセミコロンを省略できます。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../getting-started/first-program.md)を参照してください。

文末挿入の条件は[字句仕様](../../../../specs/ond/language/v1/lexical.md#semicolonと改行)に定義されています。
