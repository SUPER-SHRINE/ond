# index access

`[index]`で配列の要素を読み書きします。

```ond title="main.ond"
package main

func main() {
    var values: [3]u32 = [3]u32{4, 8, 12}
    values[1] = 9
    _ = values[0] + values[1]
}
```

## 要素を読む

`values[0]`は先頭要素の4を読みます。\
builtin配列では定数indexの範囲をcompile時に検査し、動的indexを実行時にbounds checkします。

## 要素を更新する

`values[1] = 9`は2番目の要素を更新します。\
builtin配列indexの型条件と範囲外動作は[型仕様](../../../../specs/ond/language/v1/types.md#配列とstruct)を参照してください。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)を参照してください。
