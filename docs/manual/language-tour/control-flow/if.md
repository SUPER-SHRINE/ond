# if

`if`は条件がtrueのときにblockを実行します。

```ond title="main.ond"
package main

func main() {
    var count: u32 = 3
    if count > 0 {
        count++
    } else {
        count = 1
    }
    _ = count
}
```

## 条件とblock

条件式は`bool`を返します。\
\
trueなら直後のblockを実行します。

## else分岐

条件がfalseなら`else`のblockを実行します。\
\
`else if`で条件を続けることもできます。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、条件式の規則は[文仕様](../../../../specs/ond/language/v1/statements.md)を参照してください。
