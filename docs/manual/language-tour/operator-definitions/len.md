# len operator定義

`operator (receiver) len() -> u32`で名前付き型への`len(value)`を定義できます。

```ond title="main.ond"
package main

type List struct { values: [2]i32 }

operator (list: List) len() -> u32 {
    return len(list.values)
}

func main() {
    var list: List = List{values: [2]i32{20, 22}}
    var size: u32 = len(list)
    _ = size
}
```

## u32を返す

`len` operatorは`u32`を一つ返します。\
この例では内部配列のbuiltin `len`が2を返します。

## builtin lenとの違い

名前付き型に適合するcustom operatorが先に選ばれます。\
custom `len`は通常callとして扱われ、builtin配列`len`のconstant性とは異なります。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、解決規則は[operator仕様](../../../../specs/ond/language/v1/expressions.md#user-defined-operator)を参照してください。
