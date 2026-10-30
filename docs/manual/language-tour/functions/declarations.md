# 関数宣言

`func`で関数を宣言します。\
\
parameterには名前と型を書き、必要なら戻り値の型を指定します。

```ond title="main.ond"
package main

func add(left: u32, right: u32) -> u32 {
    return left + right
}

func main() {
    var total: u32 = add(3, 4)
    _ = total
}
```

## parameterと戻り値

`add`は`u32`型のparameterを二つ受け取り、`u32`を返します。

## 関数本体

関数本体はblockです。\
\
この例は`return`で宣言した戻り値型の値を返します。\
各経路は値を返すか、`trap`などで正常終端以外に到達しなければなりません。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、宣言規則は[宣言仕様](../../../../specs/ond/language/v1/declarations.md#関数)を参照してください。
