# メソッド

メソッドはreceiverを持つ関数です。\
\
名前付き型に対して宣言します。

```ond title="main.ond"
package main

type Counter struct {
    value: u32
}

func (counter: *Counter) Increment() {
    counter.value++
}

func (counter: Counter) Value() -> u32 {
    return counter.value
}

func main() {
    var counter: Counter = Counter{value: 4}
    counter.Increment()
    _ = counter.Value()
}
```

## receiverを書く

`func (counter: *Counter) Increment()`はpointer receiverです。\
\
method内で元の値のfieldを更新できます。

## methodを呼ぶ

`counter.Increment()`のように値の後ろにmethod名を書きます。\
\
value receiverの`Value`は値を読み取り、`u32`を返します。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、receiver条件は[型仕様](../../../../specs/ond/language/v1/types.md#method)を参照してください。
