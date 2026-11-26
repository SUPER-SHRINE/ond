# 加算 `+`

二項の`+`は、同じ数値型の値を加算します。

```ond title="main.ond"
package main

func main() {
    var left: i32 = 12
    var right: i32 = 5
    var total: i32 = left + right
    _ = total
}
```

## 二つの値を足す

`left + right`の結果は`i32`で、例では17です。

## 型をそろえる

`f32`や`f64`の浮動小数点型、`q16`や`q32`の固定小数点型でも`+`を使えます。\
両辺の値の型をそろえてください。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../../getting-started/first-program.md)を参照してください。

型ごとの規則は[整数演算](../../../../../specs/ond/language/v1/expressions.md#integer-arithmetic)、[浮動小数点演算](../../../../../specs/ond/language/v1/expressions.md#floating-point-arithmetic)、[固定小数点演算](../../../../../specs/ond/language/v1/expressions.md#fixed-point-arithmetic)に定められています。
