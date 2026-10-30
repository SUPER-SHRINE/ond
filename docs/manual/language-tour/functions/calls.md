# 関数呼び出し

関数名の後ろに括弧でargumentを並べて呼び出します。

```ond title="main.ond"
package main

func multiply(value: u32, factor: u32) -> u32 {
    return value * factor
}

func main() {
    var result: u32 = multiply(6, 7)
    _ = result
}
```

## argumentを渡す

`multiply(6, 7)`は左から順にargumentを渡します。\
\
個数と型は関数signatureに一致します。

## 結果を受け取る

この関数は`u32`を一つ返すため、call結果を`result`へ格納できます。\
\
戻り値のない関数も呼び出せます。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、callとsignatureは[宣言仕様](../../../../specs/ond/language/v1/declarations.md#関数)を参照してください。
