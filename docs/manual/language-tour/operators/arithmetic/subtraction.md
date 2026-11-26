# 減算 `-`

二項の`-`は、右辺を左辺から引きます。\
両辺には同じ数値型を使います。

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

同じ浮動小数点型や同じ固定小数点型の値同士も、`-`で差を求められます。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../../getting-started/first-program.md)を参照してください。

型ごとの規則は[整数演算](../../../../../specs/ond/language/v1/expressions.md#integer-arithmetic)、[浮動小数点演算](../../../../../specs/ond/language/v1/expressions.md#floating-point-arithmetic)、[固定小数点演算](../../../../../specs/ond/language/v1/expressions.md#fixed-point-arithmetic)に定められています。
