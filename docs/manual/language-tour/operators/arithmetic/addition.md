# 加算 `+`

二項の`+`は、同じ整数型または`f32`型の値を加算します。

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

両方のoperandは同じ型にします。\
\
`f32`の加算にも`+`を使えます。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../../getting-started/first-program.md)を参照してください。

整数の演算規則は[式仕様](../../../../../specs/ond/language/v1/expressions.md#integer-arithmetic)、`f32`は[浮動小数点演算](../../../../../specs/ond/language/v1/expressions.md#floating-point-arithmetic)を参照してください。
