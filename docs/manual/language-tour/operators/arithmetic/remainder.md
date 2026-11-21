# 剰余 `%`

`%`は除算の余りを求めます。\
両辺には同じ整数型か、同じ固定小数点型を使います。

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
固定小数点でも、例えば`q32`の`1.5 % 1`は`0.5`になります。

## 負の値

結果が0でない場合、剰余は被除数と同じ符号になります。\
\
divisionとremainderの端条件は仕様を参照してください。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../../getting-started/first-program.md)を参照してください。

剰余の意味とfault条件は[整数演算仕様](../../../../../specs/ond/language/v1/expressions.md#integer-arithmetic)と[固定小数点演算仕様](../../../../../specs/ond/language/v1/expressions.md#fixed-point-arithmetic)に定義されています。
