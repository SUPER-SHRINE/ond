# 変数

`var`で再代入できる変数を宣言します。\
\
型と初期値を指定すると、宣言した時点で値が決まります。

```ond title="main.ond"
package main

func main() {
    var count: u32
    inferred := 3
    var measured = 4
    _ = count
    _ = inferred
    _ = measured
}
```

## 型を指定する

`var count: u32`のように初期値を省略すると、`count`は型のzero valueで初期化されます。

## 初期値から型を推論する

`var measured = 4`は初期値から型を推論し、`measured`の型は`i32`になります。\
\
`inferred := 3`のような短縮宣言については[短縮変数宣言](short-declarations.md)を参照してください。

この例を試す共通の`ond.toml`とcompile/link手順は[最初のプログラム](../../getting-started/first-program.md)を参照してください。

宣言の条件は[宣言仕様](../../../../specs/ond/language/v1/declarations.md#宣言)に定義されています。
