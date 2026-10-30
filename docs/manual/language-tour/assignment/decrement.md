# デクリメント

`--`は整数または`f32`の変数を1減らす後置文です。

```ond title="main.ond"
package main

func main() {
    var count: u32 = 2
    count--
    _ = count
}
```

## 文として使う

`count--`はstatementとして単独で書きます。\
\
式の値を返す演算子ではありません。

## 変数を1減らす

この文は`count -= 1`に相当します。\
\
対象はintegerまたは`f32`型です。

この例の共通manifestとcompile/link手順は[最初のプログラム](../../getting-started/first-program.md)を参照してください。

decrement文の条件は[宣言仕様](../../../../specs/ond/language/v1/declarations.md#宣言)に定義されています。
