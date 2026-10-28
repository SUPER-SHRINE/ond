# 関数型

関数型はparameterと戻り値の型でsignatureを表します。\
\
`type`で名前を付けられます。

```ond title="main.ond"
package main

func main() {
    var handler: func(u32) -> u32 = double
    _ = handler
}

func double(value: u32) -> u32 {
    return value * 2
}
```

## signatureを定義する

`func(u32) -> u32`は`u32`を一つ受け取り、`u32`を返す関数型です。\
\
parameter名は説明用に省略できます。

## 関数値との適合

`double`のsignatureが変数の関数型に一致するため、その関数値を代入できます。\
\
名前付き型の定義については[型定義](../declarations/type-definitions.md)を参照してください。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、関数型の規則は[型仕様](../../../../specs/ond/language/v1/types.md#型同一性)を参照してください。
