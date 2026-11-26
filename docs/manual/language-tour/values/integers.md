# 整数値

整数リテラルは10進、2進、8進、16進で記述できます。\
型の文脈がない場合の既定型は`i32`です。

```ond title="main.ond"
package main

func main() {
    var decimal: u32 = 42
    var hexadecimal: u32 = 0x2a
    var binary: u32 = 0b101010
    var large: u64 = 4294967296
    var negative: i64 = -4294967296
    _ = decimal + hexadecimal + binary
    _ = large
    _ = negative
}
```

## 基数を選ぶ

`0x`は16進数、`0b`は2進数の接頭辞です。\
\
8進数は先頭の`0`または`0o`で表します。

## 型と値の範囲

32ビットで表せない整数には、符号の有無に応じて`u64`や`i64`を選びます。\
例の`4294967296`は`u64`、`-4294967296`は`i64`型の値です。\
初期値は指定した型で表せる範囲に収めてください。

各整数型の範囲は[型の一覧](../../../../specs/ond/language/v1/types.md#型)で確認できます。\
共通のマニフェストとビルド手順は[最初のプログラム](../../getting-started/first-program.md)を参照してください。

リテラルの綴りは[字句仕様](../../../../specs/ond/language/v1/lexical.md#source形式)、値と型は[型仕様](../../../../specs/ond/language/v1/types.md#型同一性)を参照してください。
