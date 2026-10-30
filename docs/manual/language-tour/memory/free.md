# free

`free(pointer)`は`alloc`または`new`が返した未解放の先頭pointerを解放します。

```ond title="main.ond"
package main

func main() {
    var pointer: *u32 = new(u32)
    if pointer == nil {
        return
    }
    *pointer = 42
    free(pointer)
}
```

## allocationを解放する

例では`new(u32)`の戻り値がnilでないことを確かめてから領域を使い、先頭pointerを`free`に渡しています。

## pointerを再利用しない

解放後のpointerは無効です。\
allocationを二重解放せず、`free`には確保元の先頭pointerを渡してください。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、解放の条件は[メモリ管理仕様](../../../../specs/ond/language/v1/execution.md#メモリ管理)を参照してください。
