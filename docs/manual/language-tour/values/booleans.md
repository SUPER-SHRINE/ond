# 真偽値

`true`と`false`は`bool`型の値です。\
\
比較式などの結果も真偽値になります。

```ond title="main.ond"
package main

func main() {
    var ready: bool = true
    var finished: bool = false
    var canStart: bool = ready && !finished
    _ = canStart
}
```

## 真と偽を書く

`true`と`false`をそのまま値として使えます。

## 真偽値を組み合わせる

`!`は値を反転し、`&&`は両辺が真の場合に真になります。\
\
論理演算の短絡規則は式仕様に従います。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../getting-started/first-program.md)を参照してください。

型と演算は[型仕様](../../../../specs/ond/language/v1/types.md#型同一性)と[式仕様](../../../../specs/ond/language/v1/expressions.md#式と評価順)に定義されています。
