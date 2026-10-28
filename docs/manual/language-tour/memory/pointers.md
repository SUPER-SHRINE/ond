# pointer型

pointer型は`*T`の形で、型`T`の値を指します。

```ond title="main.ond"
package main

func main() {
    var value: u32 = 7
    var pointer: *u32 = &value
    _ = pointer
}
```

## pointer型を書く

`*u32`は`u32`を指すraw pointer型です。\
pointerは値として変数に保持できます。

## 指し先と寿命

例ではpointerがlocal変数`value`を指します。\
指し先の有効期間や読み書き範囲は利用側が守ります。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、pointer型の規則は[型仕様](../../../../specs/ond/language/v1/types.md#nilとpointer)を参照してください。
