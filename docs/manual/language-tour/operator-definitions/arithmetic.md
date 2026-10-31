# 算術operator定義

`operator`で名前付き型に算術構文を定義できます。

```ond title="main.ond"
package main

type Count struct { value: i32 }

operator (left: Count) +(right: Count) -> Count {
    return Count{value: left.value + right.value}
}

func main() {
    var sum: Count = Count{value: 20} + Count{value: 22}
    _ = sum.value
}
```

## operatorを宣言する

この`+`は`Count`を二つ受け取り、`Count`を一つ返します。\
operatorの宣言はreceiver型と同じpackageに置きます。

## 式で使う

`Count{value: 20} + Count{value: 22}`は定義したoperatorを呼び出します。\
結果の値のfieldは42です。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、適用条件は[operator仕様](../../../../specs/ond/language/v1/expressions.md#user-defined-operator)を参照してください。
