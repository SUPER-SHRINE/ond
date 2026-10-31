# sizeof

`sizeof(T)`は型`T`のsizeをbyte単位の`u32`で返します。

```ond title="main.ond"
package main

type Pair struct {
    left: u32
    right: u32
}

func main() {
    var byteCount: u32 = sizeof(Pair)
    _ = byteCount
}
```

## 型のsizeを調べる

`sizeof(Pair)`は`Pair`型のlayoutに基づくsizeです。\
結果はcompile-time constantです。

## 型をoperandにする

括弧内には値ではなく型を書きます。\
field配置とpaddingの規則はtargetのdata layoutに従います。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、builtinとlayoutは[組み込み仕様](../../../../specs/ond/language/v1/builtins.md#組み込み構文)を参照してください。
