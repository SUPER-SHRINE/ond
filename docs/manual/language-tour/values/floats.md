# 小数の値

型文脈がない浮動小数点リテラルの型は`f32`です。\
`f64`や固定小数点の`q16`、`q32`を宣言型に使うと、その型で値を受け取れます。

```ond title="main.ond"
package main

func main() {
    var ratio: f32 = 1.5
    var scaled: f32 = ratio * 2.0
    _ = scaled
}
```

## 小数表記

`1.5`のように小数点を含めて値を書けます。

## 指数表記

`1e-2`のように指数を指定できます。\
\
リテラルの型の決まり方は[型仕様](../../../../specs/ond/language/v1/types.md#型同一性)を参照してください。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../getting-started/first-program.md)を参照してください。

リテラル構文は[字句仕様](../../../../specs/ond/language/v1/lexical.md#source形式)、演算と丸めは[式仕様](../../../../specs/ond/language/v1/expressions.md#floating-point-arithmetic)と[固定小数点演算](../../../../specs/ond/language/v1/expressions.md#fixed-point-arithmetic)を参照してください。
