# 代入

`=`で変数へ値を代入します。\
\
左辺は代入可能な場所、右辺はその型に適合する値にします。

```ond title="main.ond"
package main

func main() {
    var count: u32 = 2
    count = 5
    _ = count
}
```

## 変数を更新する

`count = 5`で、変数の値を新しい値に置き換えます。

## 複数戻り値
関数の戻り値が複数ある場合の代入については[複数戻り値](../functions/multiple-results.md)を参照してください。

## 型を合わせる

右辺の値は左辺の型に適合する必要があります。\
型の異なる値を代入するときは、必要な変換を明示します。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../getting-started/first-program.md)を参照してください。

代入の条件と評価順は[文仕様](../../../../specs/ond/language/v1/statements.md#静的成立条件)を参照してください。
