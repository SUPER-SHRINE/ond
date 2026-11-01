# field access

`.`でstruct値のfieldを読み書きします。

```ond title="main.ond"
package main

type Point struct {
    x: i32
    y: i32
}

func main() {
    var point: Point = Point{x: 2, y: 3}
    point.x = 5
    _ = point.x + point.y
}
```

## fieldを読む

`point.x`はx fieldの値を読みます。\
\
例では読み出したxとyを加算します。

## fieldを更新する

`point.x = 5`のように、代入可能なstruct値のfieldへ新しい値を格納できます。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、fieldの型と配置は[型仕様](../../../../specs/ond/language/v1/types.md#配列とstruct)を参照してください。
