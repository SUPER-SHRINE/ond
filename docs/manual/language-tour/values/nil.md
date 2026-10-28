# nil値

`nil`はpointerまたは関数pointer型のnil値です。\
\
型を決める文脈が必要です。

```ond title="main.ond"
package main

func main() {
    var address: *u32 = nil
    var empty: *u32 = nil
    _ = address == empty
}
```

## 型を指定する

変数の型を`*u32`と指定することで、初期値`nil`の型が決まります。\
\
型文脈のない`nil`は値として使えません。

## nilを比較する

同じ型のpointer値は`==`や`!=`で比較できます。\
\
nil pointerを参照外しする前に、nilでないことを確認してください。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../getting-started/first-program.md)を参照してください。

nilの型条件は[型仕様](../../../../specs/ond/language/v1/types.md#型同一性)に定義されています。
