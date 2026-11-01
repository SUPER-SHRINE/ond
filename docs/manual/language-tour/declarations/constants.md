# 定数

`const`でコンパイル時に決まる値へ名前を付けます。\
\
定数には評価可能な初期値が必要です。

```ond title="main.ond"
package main

const Limit: u32 = 12

func main() {
    var count: u32 = Limit
    _ = count
}
```

## 型を指定する

`Limit: u32`のように型を明記できます。\
\
初期値から型を推論する書き方もあります。

## 定数を使う

関数内の`count`は定数`Limit`の値で初期化されます。\
\
定数は再代入できません。

この例を試す共通の`ond.toml`とcompile/link手順は[最初のプログラム](../../getting-started/first-program.md)を参照してください。

定数の適格性は[宣言仕様](../../../../specs/ond/language/v1/declarations.md#宣言)に定義されています。
