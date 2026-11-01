# continue

`continue`は現在のloop反復の残りを飛ばし、次の反復へ進みます。

```ond title="main.ond"
package main

func main() {
    var count: u32 = 0
    for count < 4 {
        count++
        if count == 2 {
            continue
        }
        _ = count
    }
}
```

## 次の反復へ進む

`count`が2のとき、その反復の後続文を飛ばします。

## 適用できる位置

`continue`はloop内で使い、loop自体は終了しません。\
\
loop条件は次の反復で改めて評価されます。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、continueの位置条件は[文仕様](../../../../specs/ond/language/v1/statements.md)を参照してください。
