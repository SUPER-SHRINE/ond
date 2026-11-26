# 乗算 `*`

二項の`*`は、同じ数値型の値を掛け合わせます。

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

同じ浮動小数点型や同じ固定小数点型の値同士も、`*`で積を求められます。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../../getting-started/first-program.md)を参照してください。

型ごとの規則は[整数演算](../../../../../specs/ond/language/v1/expressions.md#integer-arithmetic)、[浮動小数点演算](../../../../../specs/ond/language/v1/expressions.md#floating-point-arithmetic)、[固定小数点演算](../../../../../specs/ond/language/v1/expressions.md#fixed-point-arithmetic)に定められています。
