# 小数の値

型文脈がない浮動小数点リテラルの型は`f32`です。\
`f64`や固定小数点の`q16`、`q32`を宣言型に使うと、その型で値を受け取れます。

```ond title="main.ond"
package main

func main() {
    var ratio: f32 = 1.5
    var scaled: f32 = ratio * 2.0
    var precise: f64 = 1.5
    var offset: q32 = 1.5
    _ = scaled
    _ = precise * 2.0
    _ = offset * 2.0
}
```

## 小数表記

`1.5`のように小数点を含めて値を書けます。

## 小数を保持する型を選ぶ

例では`1.5`を`f32`、`f64`、`q32`の各変数に代入しています。\
`f64`は倍精度浮動小数点型で、`q16`と`q32`は小数部がそれぞれ16ビット、32ビットの固定小数点型です。\
型ごとの表現や演算時の丸めは[浮動小数点演算](../../../../specs/ond/language/v1/expressions.md#floating-point-arithmetic)と[固定小数点演算](../../../../specs/ond/language/v1/expressions.md#fixed-point-arithmetic)を参照してください。

## 指数表記

`1e-2`のように指数を指定できます。\
リテラルの構文は[字句仕様](../../../../specs/ond/language/v1/lexical.md#source形式)、型の決まり方は[型仕様](../../../../specs/ond/language/v1/types.md#型同一性)を参照してください。

共通のマニフェストとビルド手順は[最初のプログラム](../../getting-started/first-program.md)にまとまっています。
