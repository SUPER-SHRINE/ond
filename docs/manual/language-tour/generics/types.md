# generic型

generic型は型parameterを使って、要素型などが異なる複数の型を定義します。

```ond title="main.ond"
package main

type Box[T] struct {
    value: T
}

func main() {
    var box: Box[u32] = Box[u32]{value: 9}
    _ = box.value
}
```

## 型parameterを使う

`Box[T]`のfieldはparameter `T`の型です。\
\
generic型の定義時に型argumentは指定しません。

## 型を具体化する

利用時は`Box[u32]`のように型argumentを明示します。\
\
struct literalの型にも同じargumentを記述します。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、generic型の規則は[型仕様](../../../../specs/ond/language/v1/types.md#型parameter)を参照してください。
