# 剰余 `%`

`%`は整数除算の余りを求めます。\
\
二つのoperandは同じ整数型にします。

```ond title="main.ond"
package main

func main() {
    var dividend: i32 = 17
    var divisor: i32 = 5
    var remainder: i32 = dividend % divisor
    _ = remainder
}
```

## 余りを求める

`17 % 5`は`i32`の2です。\
\
剰余は`f32`には使えません。

## 負の値

結果が0でない場合、剰余は被除数と同じ符号になります。\
\
divisionとremainderの端条件は仕様を参照してください。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../../getting-started/first-program.md)を参照してください。

剰余の意味とfault条件は[整数演算仕様](../../../../../specs/ond/language/v1/expressions.md#integer-arithmetic)に定義されています。
