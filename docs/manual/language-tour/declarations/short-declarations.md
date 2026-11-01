# 短縮変数宣言

関数内では`:=`を使って変数を宣言できます。\
\
右辺の値から型が決まります。

```ond title="main.ond"
package main

func main() {
    count := 3
    count, label := 4, "ready"
    _ = count
    _ = label
}
```

## 名前と型を決める

最初の`count := 3`で`count`を宣言します。\
\
型文脈がない整数リテラル`3`の既定型は`i32`なので、`count`も`i32`になります。

## 既存の名前と新しい名前

2つ目の宣言では、既存の`count`へ値を代入し、新しい`label`を宣言します。\
\
左辺には未宣言の名前が少なくとも1つ必要です。

この例を試す共通の`ond.toml`とcompile/link手順は[最初のプログラム](../../getting-started/first-program.md)を参照してください。

短縮宣言の条件は[宣言仕様](../../../../specs/ond/language/v1/declarations.md#宣言)に定義されています。
