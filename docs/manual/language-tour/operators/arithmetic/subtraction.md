# 減算 `-`

二項の`-`は、同じ整数型または`f32`型の右辺を左辺から引きます。

```ond title="main.ond"
package main

func main() {
    var total: i32 = 20
    var used: i32 = 7
    var remaining: i32 = total - used
    _ = remaining
}
```

## 差を求める

`total - used`の結果は`i32`で、例では13です。

## 型をそろえる

両方のoperandは同じ型にします。\
\
`f32`同士の減算にも`-`を使えます。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../../getting-started/first-program.md)を参照してください。

整数の演算規則は[式仕様](../../../../../specs/ond/language/v1/expressions.md#integer-arithmetic)、`f32`は[浮動小数点演算](../../../../../specs/ond/language/v1/expressions.md#floating-point-arithmetic)を参照してください。
