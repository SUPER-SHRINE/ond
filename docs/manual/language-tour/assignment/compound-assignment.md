# 複合代入

複合代入は二項演算と代入を一つの文にまとめます。

```ond title="main.ond"
package main

func main() {
    var count: u32 = 3
    count += 4
    count *= 2
    _ = count
}
```

## 演算して代入する

`count += 4`は加算結果を`count`へ代入します。\
\
`count *= 2`も同じ形で乗算します。

## 対応する演算子

`+=`や`*=`のほか、`-=`, `/=`, `%=`, `&=`, `|=`, `^=`, `&^=`, `<<=`, `>>=`を使えます。\
\
各式は対応する演算の型条件を満たす必要があります。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../getting-started/first-program.md)を参照してください。

複合代入の成立条件は[文仕様](../../../../specs/ond/language/v1/statements.md#静的成立条件)を参照してください。
