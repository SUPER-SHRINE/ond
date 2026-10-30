# 不等値比較 `!=`

`!=`は二つの値が異なるかを比較し、`bool`を返します。

```ond title="main.ond"
package main

func main() {
    var expected: i32 = 12
    var actual: i32 = 9
    var changed: bool = expected != actual
    _ = changed
}
```

## 異なる値を見つける

例では12と9が異なるため、`changed`は`true`になります。

## operand型

整数は同じ整数型同士で比較します。\
\
`bool`、同じraw pointer型、同じfunction pointer型、`f32`にも使えます。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../../getting-started/first-program.md)を参照してください。

比較の条件は[式仕様](../../../../../specs/ond/language/v1/expressions.md#式と評価順)を参照してください。
