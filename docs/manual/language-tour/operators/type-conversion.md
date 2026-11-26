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

## リテラルと変数を変換する

`0.1 as f64`は、リテラルの値を`f64`の精度で受け取る指定です。\
先に`f32`の変数に入れてから`as f64`で変換すると、`f32`で丸めた値が変換元になります。

## 狭い整数とf32の往復変換

例えば`value`が実行時の`u16`の値なら、`ond compile`は`(value as f32) as u16`の結果に元の値を使います。\
適用条件と中間の`f32`値を保持する場合については、[実行時往復変換の仕様](../../../../specs/ond/language/v1/types.md#狭い整数とf32の実行時往復変換)を参照してください。

この最適化で必要なstack容量が減ると、同じstack設定で以前は`StackOverflow`になった箇所を通過し、その後の書き込みなどが実行される場合があります。\
stack容量の検査と、この往復変換を省く場合の観測規則は[Kagura ABI](../../../../specs/ond/targets/kagura-v1/abi.md#7-stack-overflow)に従います。
