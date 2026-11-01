# 整数値

整数リテラルは10進、2進、8進、16進で記述できます。\
\
型の文脈がない整数リテラルの既定型は`i32`です。

```ond title="main.ond"
package main

func main() {
    var decimal: u32 = 42
    var hexadecimal: u32 = 0x2a
    var binary: u32 = 0b101010
    _ = decimal + hexadecimal + binary
}
```

## 基数を選ぶ

`0x`は16進数、`0b`は2進数の接頭辞です。\
\
8進数は先頭の`0`または`0o`で表します。

## 型と値の範囲

整数リテラルは型文脈で解釈され、要求される整数型で表せる範囲を満たす必要があります。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../getting-started/first-program.md)を参照してください。

リテラルの綴りは[字句仕様](../../../../specs/ond/language/v1/lexical.md#source形式)、値と型は[型仕様](../../../../specs/ond/language/v1/types.md#型同一性)を参照してください。
