# return

`return`は関数の実行を終え、宣言された戻り値を呼び出し元へ返します。

```ond title="main.ond"
package main

func double(value: u32) -> u32 {
    return value * 2
}

func main() {
    var result: u32 = double(5)
    _ = result
}
```

## 値を返す

`double`は`u32`を一つ返します。\
\
returnする値の型は関数宣言の戻り値型に一致させます。

## 戻り値のない関数

戻り値の型を省略した関数は値を返しません。\
\
returnで値を返す場合は、関数signatureに戻り値型が必要です。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、return規則は[宣言仕様](../../../../specs/ond/language/v1/declarations.md#関数)を参照してください。
