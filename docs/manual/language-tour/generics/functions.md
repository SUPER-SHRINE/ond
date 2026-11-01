# generic関数

generic関数は型parameterを受け取り、異なる型に対して同じ処理を使えます。

```ond title="main.ond"
package main

func Identity[T](value: T) -> T {
    return value
}

func main() {
    var result: u32 = Identity[u32](7)
    _ = result
}
```

## 型parameterを宣言する

`Identity[T]`の`T`は関数parameterと戻り値に使う型parameterです。

## 型argumentを指定する

call時に`Identity[u32](7)`のように型argumentを明示します。\
\
型argumentと宣言parameterの個数を一致させます。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、generic規則は[型仕様](../../../../specs/ond/language/v1/types.md#型parameter)を参照してください。
