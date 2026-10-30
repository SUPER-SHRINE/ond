# コメント

コメントはソース中の説明です。\
\
`//`から行末まで、または`/*`から最初の`*/`までを書けます。

```ond title="main.ond"
// package宣言の前にもコメントを書けます
package main

func main() {
    /* 複数行に
       またがる説明 */
    var count: u32 = 3 // 行末にも書けます
    _ = count
}
```

## 行コメント

`//`以降は、その行の終わりまで字句として扱われません。

## ブロックコメント

`/*`と`*/`で囲みます。\
\
閉じる`*/`がないブロックコメントはエラーです。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../getting-started/first-program.md)を参照してください。

コメントと字句の規則は[字句仕様](../../../../specs/ond/language/v1/lexical.md#source形式)に定義されています。
