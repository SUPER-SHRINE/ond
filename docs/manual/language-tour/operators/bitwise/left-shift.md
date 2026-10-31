# 左シフト `<<`

`<<`は整数のbitを左へ移動し、左辺の型で結果を返します。

```ond title="main.ond"
package main

func main() {
    var bits: u8 = 3
    var shifted: u8 = bits << 2
    _ = shifted
}
```

## bitを左へ動かす

`3`は`00000011`です。\
\
2 bit左へ移動すると`00001100`になり、結果は12です。

## shift count

左辺と結果の型は同じ整数型です。\
\
shift countは`u32`で、整数リテラルはこの位置で`u32`として扱われます。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../../getting-started/first-program.md)を参照してください。

countとbit幅の規則は[整数演算仕様](../../../../../specs/ond/language/v1/expressions.md#integer-arithmetic)に定義されています。
