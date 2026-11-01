# len

`len(array)`は配列の要素数を`u32`で返します。\
結果はcompile-time constantです。

```ond title="main.ond"
package main

func main() {
    var values: [3]u8 = [3]u8{1, 2, 3}
    var size: u32 = len(values)
    _ = size
}
```

## 配列の長さ

例の`len(values)`は3です。\
文字列リテラルも`[N]u8`の配列なので同じように使えます。

## custom len

名前付き型に`len` operatorを定義するとcustom operatorが先に選択されます。\
その呼び出しは通常callとして扱われます。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、builtinの条件は[組み込み仕様](../../../../specs/ond/language/v1/builtins.md#4-len)を参照してください。
