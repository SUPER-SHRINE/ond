# bitwise AND `&`

二項の`&`は整数の各bitについて、両方が1の位置だけを1にします。

```ond title="main.ond"
package main

func main() {
    var flags: u8 = 0b1101
    var mask: u8 = 0b0101
    var selected: u8 = flags & mask
    _ = selected
}
```

## bitを選ぶ

例の`1101 & 0101`は`0101`です。\
\
`selected`は`u8`型の5になります。

## operand型

両方のoperandは同じ整数型にします。\
\
bitwise演算に`bool`や`f32`は使いません。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../../getting-started/first-program.md)を参照してください。

bitの演算規則は[整数演算仕様](../../../../../specs/ond/language/v1/expressions.md#integer-arithmetic)に定義されています。
