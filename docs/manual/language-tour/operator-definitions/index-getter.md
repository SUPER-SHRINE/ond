# index getter `[]`

`operator ... [](index)`は名前付き型にindex読み取り構文を定義します。

```ond title="main.ond"
package main

type List struct { values: [2]i32 }

operator (list: List) [](index: u32) -> i32 {
    return list.values[index]
}

func main() {
    var list: List = List{values: [2]i32{20, 22}}
    var value: i32 = list[1]
    _ = value
}
```

## getterを定義する

operatorはindexを受け取り、`i32`値を返します。\
例では内部配列の値を返すため、その配列の通常のbounds checkが適用されます。

## index構文で読む

`list[1]`はcustom getterを呼び、22を返します。\
custom getter一般にbuiltin配列と同じbounds保証があるという意味ではありません。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、解決条件は[operator仕様](../../../../specs/ond/language/v1/expressions.md#user-defined-operator)を参照してください。
