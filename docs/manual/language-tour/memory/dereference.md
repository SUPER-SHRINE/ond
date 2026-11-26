# dereference `*`

前置の`*`はpointerの指し先を読み書きします。

```ond title="main.ond"
package main

func main() {
    var value: u32 = 7
    var pointer: *u32 = &value
    *pointer = 9
    _ = value
}
```

## 指し先を読む

`*pointer`はpointerが指す`u32`値です。\
例では代入後に`value`が9になります。

## 読み書きの前提

dereferenceには有効な指し先が必要です。\
nil確認と領域の寿命管理は呼び出し側で行います。

Kaguraで`*u64`などの64ビット値を参照外しで読み書きすると、32ビットのバスアクセスが2回発生します。\
アクセスの順序や途中でフォルトした場合の動作は[raw pointerアクセス仕様](../../../../specs/ond/targets/kagura-v1/machine.md#6-raw-pointer-access)に定められています。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、pointer accessの条件は[型仕様](../../../../specs/ond/language/v1/types.md#nilとpointer)を参照してください。
