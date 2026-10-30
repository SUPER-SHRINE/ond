# 改行

改行はトークンを区切り、文を終える位置では文末を作ります。\
\
式を続ける場合は、改行前を演算子など継続可能な位置にします。

```ond title="main.ond"
package main

func main() {
    var first: u32 = 1
    var second: u32 = 2
    _ = first +
        second
}
```

## 文を終える改行

変数宣言の後の改行で文が終わります。\
\
文末記号の自動挿入条件は直前のtokenで決まります。

## 式を続ける改行

この例では`+`の後で改行しているため、次の行の`second`が同じ式に続きます。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../getting-started/first-program.md)を参照してください。

改行と文末挿入は[字句仕様](../../../../specs/ond/language/v1/lexical.md#semicolonと改行)に定義されています。
