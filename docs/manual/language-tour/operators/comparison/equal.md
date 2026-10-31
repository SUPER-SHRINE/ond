# 等値比較 `==`

`==`は二つの値が等しいかを比較し、`bool`を返します。

```ond title="main.ond"
package main

func main() {
    var expected: i32 = 12
    var actual: i32 = 12
    var matches: bool = expected == actual
    _ = matches
}
```

## 値を比較する

例では両辺が等しいため、`matches`は`true`になります。

## operand型

整数は同じ整数型同士で比較します。\
\
`bool`、同じraw pointer型、同じfunction pointer型は等値比較ができます。\
\
`f32`にも適用できます。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../../getting-started/first-program.md)を参照してください。

比較の条件は[式仕様](../../../../../specs/ond/language/v1/expressions.md#式と評価順)を参照してください。
