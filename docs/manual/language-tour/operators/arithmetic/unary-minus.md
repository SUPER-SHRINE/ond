# 単項マイナス `-`

単項の`-`は一つの整数値または`f32`値に適用します。

```ond title="main.ond"
package main

func main() {
    var value: i32 = 8
    var negative: i32 = -value
    _ = negative
}
```

## 値の符号を反転する

`-value`は一つのoperandに適用します。\
\
例の`i32`結果は-8です。

## 二項減算との違い

`left - right`は右辺を左辺から引く二項演算です。\
\
単項形式は一つの値に適用します。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../../getting-started/first-program.md)を参照してください。

単項演算の規則は[整数演算仕様](../../../../../specs/ond/language/v1/expressions.md#integer-arithmetic)と[浮動小数点演算](../../../../../specs/ond/language/v1/expressions.md#floating-point-arithmetic)を参照してください。
