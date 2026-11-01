# 小なりイコール比較 `<=`

`<=`は左辺が右辺以下かを比較し、`bool`を返します。

```ond title="main.ond"
package main

func main() {
    var current: i32 = 10
    var limit: i32 = 10
    var inRange: bool = current <= limit
    _ = inRange
}
```

## 等しい場合を含める

両辺が等しい場合も条件は真です。\
\
例では`inRange`が`true`になります。

## 順序比較の型

整数は同じ整数型同士、`f32`同士で比較できます。\
\
結果は`bool`です。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../../getting-started/first-program.md)を参照してください。

比較の条件は[式仕様](../../../../../specs/ond/language/v1/expressions.md#式と評価順)を参照してください。
