# 右シフト `>>`

`>>`は整数のbitを右へ移動し、左辺の型で結果を返します。

```ond title="main.ond"
package main

func main() {
    var bits: u8 = 12
    var shifted: u8 = bits >> 2
    _ = shifted
}
```

## bitを右へ動かす

`12`は`00001100`です。\
\
2 bit右へ移動すると`00000011`になり、結果は3です。

## 型による上位bit

左辺と結果の型は同じ整数型です。\
\
unsigned型では上位bitに0が入り、signed型では元の最上位bitで埋められます。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../../getting-started/first-program.md)を参照してください。

shiftの動作は[整数演算仕様](../../../../../specs/ond/language/v1/expressions.md#integer-arithmetic)に定義されています。
