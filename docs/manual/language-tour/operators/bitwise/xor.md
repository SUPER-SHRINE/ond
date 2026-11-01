# bitwise XOR `^`

二項の`^`は整数の各bitについて、左右の値が異なる位置だけを1にします。

```ond title="main.ond"
package main

func main() {
    var left: u8 = 0b1100
    var right: u8 = 0b1010
    var difference: u8 = left ^ right
    _ = difference
}
```

## 異なるbitを残す

例の`1100 ^ 1010`は`0110`です。\
\
結果の`difference`は6になります。

## 単項形式との違い

二項形式は2つの整数を比較してbitを求めます。\
\
単項の`^value`は各bitを反転します。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../../getting-started/first-program.md)を参照してください。

bit演算の規則は[整数演算仕様](../../../../../specs/ond/language/v1/expressions.md#integer-arithmetic)に定義されています。
