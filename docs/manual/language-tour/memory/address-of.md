# address-of `&`

前置の`&`はaddressableな値のaddressを取り、pointerを作ります。

```ond title="main.ond"
package main

func main() {
    var value: u32 = 7
    var pointer: *u32 = &value
    _ = pointer
}
```

## 変数のaddressを取る

`&value`の型は`*u32`です。\
ここではlocal変数のstorageを指します。

## addressableな値

addressを取る対象はaddressableでなければなりません。\
pointer型自体の説明は[pointer型](pointers.md)を参照してください。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、address演算の条件は[式仕様](../../../../specs/ond/language/v1/expressions.md)を参照してください。
