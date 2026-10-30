# for

`for`は条件がtrueの間、blockを繰り返します。

```ond title="main.ond"
package main

func main() {
    var count: u32 = 0
    for count < 3 {
        count++
    }
    _ = count
}
```

## 条件で繰り返す

各反復の前に`count < 3`を評価し、falseになるとloopを終えます。

## loop内の更新

block内で`count++`を実行して条件を変化させます。\
\
条件のない`for`で無限loopを書くこともできます。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、loopの規則は[文仕様](../../../../specs/ond/language/v1/statements.md)を参照してください。
