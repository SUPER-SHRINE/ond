# trap

`trap`は理由を指定して、実行時faultを発生させる文です。

```ond title="main.ond"
package main

func main() {
    var count: u32 = 0
    if count == 0 {
        trap "count must be nonzero"
    }
}
```

## 理由を付ける

`trap`の後ろに文字列リテラルを置き、faultの理由を示します。

## 実行経路

例では条件がtrueとなるため、実行時にtrap文へ到達します。\
\
compileとlinkは実行環境でtrapが起きることを意味しません。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、trapの契約は[組み込み仕様](../../../../specs/ond/language/v1/builtins.md#8-trap)を参照してください。
