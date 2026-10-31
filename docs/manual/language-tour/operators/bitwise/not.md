# bitwise NOT `^`

単項の`^`は整数の各bitを反転します。

```ond title="main.ond"
package main

func main() {
    var bits: u8 = 0b00001111
    var inverted: u8 = ^bits
    _ = inverted
}
```

## 各bitを反転する

`u8`では`00001111`の反転結果は`11110000`、つまり240です。

## 単項演算として書く

`^bits`は一つの整数operandを取ります。\
\
論理値を反転する`!`とは異なります。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../../getting-started/first-program.md)を参照してください。

bit演算の規則は[整数演算仕様](../../../../../specs/ond/language/v1/expressions.md#integer-arithmetic)に定義されています。
