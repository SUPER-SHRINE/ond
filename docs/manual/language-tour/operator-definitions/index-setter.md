# index setter `[]=`

`operator ... []=(index, value)`は名前付き型にindex代入構文を定義します。

```ond title="main.ond"
package main

type List struct { values: [2]i32 }

operator (list: *List) []=(index: u32, value: i32) {
    list.values[index] = value
}

func main() {
    var list: List = List{values: [2]i32{20, 22}}
    list[1] = 42
    _ = list.values[1]
}
```

## setterを定義する

receiverは`*List`で、indexと代入する値を受け取ります。\
operator本体は内部配列のelementへ代入します。

## index代入で使う

`list[1] = 42`はcustom setterを呼び出します。\
この例の内部配列への代入にはbuiltin配列のbounds checkが適用されます。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、setterの条件は[operator仕様](../../../../specs/ond/language/v1/expressions.md#user-defined-operator)を参照してください。
