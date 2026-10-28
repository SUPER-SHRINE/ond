# AND-NOT `&^`

`&^`は左辺のうち、右辺で指定したbitを消します。

```ond title="main.ond"
package main

func main() {
    var flags: u8 = 0b1111
    var clearMask: u8 = 0b0101
    var remaining: u8 = flags &^ clearMask
    _ = remaining
}
```

## bitを消す

例の`1111 &^ 0101`は`1010`です。\
\
`remaining`は10になります。

## 同じ整数型を使う

左右のoperandは同じ整数型にします。\
\
結果もその型になります。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../../getting-started/first-program.md)を参照してください。

演算のbitごとの定義は[整数演算仕様](../../../../../specs/ond/language/v1/expressions.md#integer-arithmetic)を参照してください。
