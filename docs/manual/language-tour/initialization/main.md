# main

実行projectのroot packageにはentry関数`main`を一つ宣言します。

```ond title="main.ond"
package main

func main() {
    var ready: bool = true
    _ = ready
}
```

## signature

`main`はparameterも戻り値も持ちません。\
実行projectではこのsignatureの関数を一つ宣言します。

## entryの位置

この関数はrootの`main`packageに置きます。\
library projectには`main`関数が必要ない場合があります。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、entryと終了codeは[実行仕様](../../../../specs/ond/language/v1/execution.md#初期化とエントリ)を参照してください。
