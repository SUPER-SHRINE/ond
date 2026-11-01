# 型変換 `as`

`as`で式の値を指定した型へ明示的に変換します。

```ond title="main.ond"
package main

func main() {
    var wide: u32 = 42
    var narrow: u8 = wide as u8
    _ = narrow
}
```

## 変換先を書く

`wide as u8`は`wide`の値を`u8`へ変換します。\
\
変換後の値を格納する変数も`u8`型です。

## 変換の条件

変換可能な型と定数値の範囲条件は型ごとに定められています。\
\
runtime値の変換では、変換後の値の規則を確認してください。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../getting-started/first-program.md)を参照してください。

変換の成立条件は[型仕様](../../../../specs/ond/language/v1/types.md#型同一性)に定義されています。
