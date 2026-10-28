# 小なり比較 `<`

`<`は左辺が右辺より小さいかを比較し、`bool`を返します。

```ond title="main.ond"
package main

func main() {
    var current: i32 = 4
    var limit: i32 = 10
    var below: bool = current < limit
    _ = below
}
```

## 左辺と右辺を比べる

例では4が10より小さいため、`below`は`true`になります。

## 順序比較の型

整数は同じ整数型同士、`f32`同士で順序を比較できます。\
\
`bool`やpointerには順序比較を使いません。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../../getting-started/first-program.md)を参照してください。

比較の条件は[式仕様](../../../../../specs/ond/language/v1/expressions.md#式と評価順)を参照してください。
