# 論理和 `||`

`||`は二つの`bool`値の論理和を求めます。\
\
左辺がtrueなら右辺の値評価を省略します。

```ond title="main.ond"
package main

func main() {
    var isAdmin: bool = false
    var ownsItem: bool = true
    var allowed: bool = isAdmin || ownsItem
    _ = allowed
}
```

## どちらかが真のとき

例では`ownsItem`がtrueなので、`allowed`はtrueになります。

## 短絡評価

左辺がtrueなら右辺は値評価されません。\
\
右辺も名前解決と型検査の対象です。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../../getting-started/first-program.md)を参照してください。

評価順と短絡条件は[式仕様](../../../../../specs/ond/language/v1/expressions.md#式と評価順)を参照してください。
