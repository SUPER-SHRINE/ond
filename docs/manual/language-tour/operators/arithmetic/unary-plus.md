# 単項プラス `+`

単項の`+`は整数値または`f32`値の前に置き、値の符号をそのまま保ちます。

```ond title="main.ond"
package main

func main() {
    var value: i32 = 8
    var same: i32 = +value
    _ = same
}
```

## 値の前に置く

`+value`は一つのoperandを取る単項演算です。\
\
例では値8を保ちます。

## 二項加算との違い

`left + right`は二つの値を足す二項演算です。\
\
単項形式は一つの値に適用します。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../../getting-started/first-program.md)を参照してください。

単項演算の規則は[整数演算仕様](../../../../../specs/ond/language/v1/expressions.md#integer-arithmetic)と[浮動小数点演算](../../../../../specs/ond/language/v1/expressions.md#floating-point-arithmetic)を参照してください。
