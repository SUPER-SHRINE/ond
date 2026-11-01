# alignof

`alignof(T)`は型`T`のalignmentを`u32`で返します。

```ond title="main.ond"
package main

func main() {
    var alignment: u32 = alignof(u32)
    _ = alignment
}
```

## alignmentを調べる

`alignof(u32)`は`u32`型のalignmentを返します。\
結果はcompile-time constantです。

## 型をoperandにする

括弧内には値ではなく型を書きます。\
型layoutの具体値はtargetのdata layoutで定まります。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、builtinは[組み込み仕様](../../../../specs/ond/language/v1/builtins.md#組み込み構文)を参照してください。
