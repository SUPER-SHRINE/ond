# new

`new(T)`は型`T`の値一つ分を確保し、zero valueで初期化した`*T`を返します。

```ond title="main.ond"
package main

type Counter struct {
    value: u32
}

func main() {
    var counter: *Counter = new(Counter)
    if counter == nil {
        return
    }
    counter.value = 1
    free(counter)
}
```

## 型を指定する

`new(Counter)`は`Counter`一つ分のstorageを確保し、`*Counter`を返します。

## zero valueから始める

確保に失敗した場合は`nil`が返るため、field accessの前に確認します。\
成功時の領域はzero valueで初期化されます。不要になれば`free`で解放します。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、`new`の規則は[組み込み仕様](../../../../specs/ond/language/v1/builtins.md#組み込み構文)を参照してください。
