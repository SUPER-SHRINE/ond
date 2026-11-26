# 除算 `/`

`/`は、左辺を右辺で割ります。\
両辺には同じ数値型を使います。

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

## 浮動小数点と固定小数点の除算

同じ浮動小数点型や同じ固定小数点型の値同士も、`/`で割れます。\
結果の丸め方と除数がゼロのときの動作は型ごとに確認してください。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../../getting-started/first-program.md)を参照してください。

型ごとの規則は[整数演算](../../../../../specs/ond/language/v1/expressions.md#integer-arithmetic)、[浮動小数点演算](../../../../../specs/ond/language/v1/expressions.md#floating-point-arithmetic)、[固定小数点演算](../../../../../specs/ond/language/v1/expressions.md#fixed-point-arithmetic)に定められています。
