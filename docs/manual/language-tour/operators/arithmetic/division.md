# 除算 `/`

`/`は、同じ整数型または`f32`型の左辺を右辺で割ります。

```ond title="main.ond"
package main

func main() {
    var dividend: i32 = 17
    var divisor: i32 = 5
    var quotient: i32 = dividend / divisor
    _ = quotient
}
```

## 整数の商

整数除算の商は0方向へ丸められます。\
\
例の`17 / 5`は`i32`の3です。

## `f32`の除算

`f32`同士の除算も`/`で表します。\
\
整数の商の丸めとは異なる浮動小数点演算です。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../../getting-started/first-program.md)を参照してください。

ゼロ除算を含む整数の規則は[式仕様](../../../../../specs/ond/language/v1/expressions.md#integer-arithmetic)、`f32`は[浮動小数点演算](../../../../../specs/ond/language/v1/expressions.md#floating-point-arithmetic)を参照してください。
