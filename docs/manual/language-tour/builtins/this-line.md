# thisLine

`thisLine()`は現在のsource行番号を`u32`で返します。

```ond title="main.ond"
package main

func main() {
    var line: u32 = thisLine()
    _ = line
}
```

## 行番号を得る

戻り値は`u32`です。\
呼び出し位置に対応するsource行番号を返します。

## source上の位置

このbuiltinは現在のsource位置を表します。\
実行時に別のsource fileの位置を問い合わせる関数ではありません。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、builtinの契約は[組み込み仕様](../../../../specs/ond/language/v1/builtins.md#組み込み関数)を参照してください。
