# struct

`struct`は複数の名前付きfieldをまとめた型です。

```ond title="main.ond"
package main

type Point struct {
    x: i32
    y: i32
}

func main() {
    var origin: Point = Point{x: 0, y: 0}
    _ = origin.x
}
```

## fieldを定義する

`Point`は`x`と`y`という`i32` fieldを持ちます。\
\
各fieldに名前と型を指定します。

## 値を初期化する

struct literalではfield名を指定します。\
\
fieldを省略した場合、そのfieldは型のzero valueになります。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、structの型規則は[型仕様](../../../../specs/ond/language/v1/types.md#配列とstruct)を参照してください。
