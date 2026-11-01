# 大なり比較 `>`

`>`は左辺が右辺より大きいかを比較し、`bool`を返します。

```ond title="main.ond"
package main

func main() {
    var score: i32 = 18
    var passing: i32 = 15
    var passed: bool = score > passing
    _ = passed
}
```

## 大きさを比べる

例では18が15より大きいため、`passed`は`true`になります。

## 順序比較の型

整数は同じ整数型同士、`f32`同士で順序を比較できます。\
\
結果の型は`bool`です。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../../getting-started/first-program.md)を参照してください。

比較の条件は[式仕様](../../../../../specs/ond/language/v1/expressions.md#式と評価順)を参照してください。
