# break

`break`は実行中の最も内側のloopを終了します。

```ond title="main.ond"
package main

func main() {
    var count: u32 = 0
    for {
        if count == 3 {
            break
        }
        count++
    }
    _ = count
}
```

## loopを終える

`count`が3になると`break`が実行され、loop直後の文へ進みます。

## 適用できる位置

`break`はloop内で使います。\
\
nested loopでは最も内側のloopから抜けます。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、breakの位置条件は[文仕様](../../../../specs/ond/language/v1/statements.md)を参照してください。
