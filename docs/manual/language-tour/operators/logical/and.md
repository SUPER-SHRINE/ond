# 論理積 `&&`

`&&`は二つの`bool`値の論理積を求めます。\
\
左辺がfalseなら右辺の値評価を省略します。

```ond title="main.ond"
package main

func main() {
    var hasFile: bool = true
    var isOpen: bool = true
    var canRead: bool = hasFile && isOpen
    _ = canRead
}
```

## 両方が真のとき

例では両辺がtrueなので、`canRead`もtrueになります。

## 短絡評価

左辺がfalseなら右辺は値評価されません。\
\
右辺も名前解決と型検査の対象です。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../../getting-started/first-program.md)を参照してください。

評価順と短絡条件は[式仕様](../../../../../specs/ond/language/v1/expressions.md#式と評価順)を参照してください。
