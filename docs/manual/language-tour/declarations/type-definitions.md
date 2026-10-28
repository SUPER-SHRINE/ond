# 型定義

`type`で既存の型を基底型とする、新しい名前付き型を定義します。

```ond title="main.ond"
package main

type UserId u32

func main() {
    var raw: u32 = 7
    var id: UserId = raw as UserId
    _ = id
}
```

## 名前付き型を定義する

`type UserId u32`は`UserId`を定義型として宣言し、`u32`を基底型にします。\
\
これは`UserId`という別名を作る宣言ではありません。

## 値を変換する

`raw as UserId`は`u32`値を明示的に`UserId`へ変換します。\
\
定義型と基底型の値は、型が異なるためそのまま代入せず、変換を記述します。

この例を試す共通の`ond.toml`とcompile/link手順は[最初のプログラム](../../getting-started/first-program.md)を参照してください。

型の同一性と変換は[型仕様](../../../../specs/ond/language/v1/types.md#型同一性)に定義されています。
