# 配列

配列は要素数と要素型が固定された値です。\
\
型は`[N]T`の形で書きます。

```ond title="main.ond"
package main

func main() {
    var scores: [3]u32 = [3]u32{8, 13, 21}
    _ = scores[0]
}
```

## 要素型と長さ

`[3]u32`は`u32`要素を3個持ちます。\
\
配列値は代入時に値としてcopyされます。

## 初期値を並べる

`[3]u32{8, 13, 21}`で要素を順に初期化します。\
\
要素を省略した場合はzero valueになります。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、配列型は[型仕様](../../../../specs/ond/language/v1/types.md#配列とstruct)を参照してください。
