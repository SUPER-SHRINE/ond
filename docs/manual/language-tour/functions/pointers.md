# 関数ポインター

関数ポインターは関数を値として保持し、変数を通じて呼び出せます。

```ond title="main.ond"
package main

func double(value: u32) -> u32 {
    return value * 2
}

func main() {
    var handler: func(u32) -> u32 = double
    var result: u32 = handler(6)
    _ = result
}
```

## 関数を値にする

`double`を`handler`に代入して保持します。\
\
関数のsignatureは変数型の`func(u32) -> u32`に一致します。

## 値から呼び出す

`handler(6)`は保持した関数を間接呼び出しし、12を返します。\
\
function pointerと関数型の詳細は[関数型](types.md)を参照してください。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、pointer型の規則は[型仕様](../../../../specs/ond/language/v1/types.md#型同一性)を参照してください。
