# bitwise OR `|`

二項の`|`は整数の各bitについて、どちらかが1の位置を1にします。

```ond title="main.ond"
package main

func main() {
    var left: u8 = 0b1001
    var right: u8 = 0b0110
    var combined: u8 = left | right
    _ = combined
}
```

## bitをまとめる

例の`1001 | 0110`は`1111`です。\
\
`combined`は15になります。

## operand型

両方のoperandは同じ整数型にします。\
\
結果もその型です。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../../getting-started/first-program.md)を参照してください。

bitの演算規則は[整数演算仕様](../../../../../specs/ond/language/v1/expressions.md#integer-arithmetic)に定義されています。
