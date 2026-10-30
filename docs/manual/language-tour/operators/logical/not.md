# 論理否定 `!`

`!`は一つの`bool`値を反転します。

```ond title="main.ond"
package main

func main() {
    var locked: bool = false
    var unlocked: bool = !locked
    _ = unlocked
}
```

## 真偽を反転する

`!locked`は`locked`がfalseなのでtrueになります。\
\
結果も`bool`です。

## operand型

単項`!`のoperandは`bool`でなければなりません。\
\
整数のbit反転には別の演算子`^`を使います。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../../getting-started/first-program.md)を参照してください。

論理演算は[式仕様](../../../../../specs/ond/language/v1/expressions.md#式と評価順)を参照してください。
