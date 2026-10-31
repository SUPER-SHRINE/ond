# 複数戻り値

関数は固定個数の値を複数返せます。\
\
戻り値型を括弧内に列挙します。

```ond title="main.ond"
package main

func bounds(value: u32) -> (u32, u32) {
    return 0, value
}

func main() {
    low, high := bounds(8)
    _ = low
    _ = high
}
```

## 値を並べて返す

`bounds`は`u32`を二つ返します。\
\
returnする値の数と型は宣言と対応します。

## call結果を受け取る

複数戻り値のcallは、同数の左辺へ展開できます。\
\
ここでは`low`と`high`に結果を受けます。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、複数戻り値の規則は[宣言仕様](../../../../specs/ond/language/v1/declarations.md#関数)を参照してください。
