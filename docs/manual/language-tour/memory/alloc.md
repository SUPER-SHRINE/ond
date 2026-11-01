# alloc

`alloc[T](count)`は`T`型の要素を指定数確保し、先頭を`*T`として返します。\
確保領域は未初期化です。

```ond title="main.ond"
package main

func main() {
    var pointer: *u32 = alloc[u32](2)
    if pointer == nil {
        return
    }
    *pointer = 5
    free(pointer)
}
```

## 型と要素数を指定する

`alloc[u32](2)`は`u32`要素を2つ分確保します。\
countは`u32`です。

## 初期化して解放する

allocationに成功しない場合、戻り値は`nil`です。dereferenceする前にnilを確認し、読む前に必要な値を書き込みます。\
不要になった領域は先頭pointerを`free`へ渡します。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、確保条件は[組み込み仕様](../../../../specs/ond/language/v1/builtins.md#2-alloc)を参照してください。
