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

## compileとlinkで理由を確認する

共通の[最初のプログラム](../../getting-started/first-program.md)と同じmanifestを用意し、この例のdirectoryで`ond compile .`、続けて`ond link --project .`を実行します。\
両commandはexit `0`で終了し、`.ond/target/a.bin`と`.ond/target/a.onddebug`が生成されます。\
debug sidecarには`count must be nonzero`の理由が記録されます。

compileとlinkはprogramを実行しません。\
この例では`count`が0で条件を満たすため、実行時にはtrap文へ到達します。

trap理由とdebug metadataの定義は[組み込み仕様](../../../../specs/ond/language/v1/builtins.md#8-trap)および[debug metadata仕様](../../../../specs/ond/debug-metadata/v1.md)を参照してください。
