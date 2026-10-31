# 乗算 `*`

二項の`*`は、同じ整数型または`f32`型の値を掛け合わせます。

```ond title="main.ond"
package main

func main() {
    var width: u32 = 6
    var height: u32 = 4
    var area: u32 = width * height
    _ = area
}
```

## 積を求める

`width * height`の結果は`u32`で、例では24です。

## 同じ型で計算する

両方のoperandは同じ型にします。\
\
`f32`同士の乗算にも`*`を使えます。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../../getting-started/first-program.md)を参照してください。

整数の演算規則は[式仕様](../../../../../specs/ond/language/v1/expressions.md#integer-arithmetic)、`f32`は[浮動小数点演算](../../../../../specs/ond/language/v1/expressions.md#floating-point-arithmetic)を参照してください。
