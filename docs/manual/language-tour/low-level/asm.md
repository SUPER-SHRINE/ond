# asm（Kagura v1）

`asm`はKagura v1向けの命令列を記述し、registerとOnd値の間で値を受け渡します。

```ond title="main.ond"
package main

func main() {
    var value: u32 = 41
    asm (r1 <-> value) {
        ADD r1, r1, r0, 1
    }
    _ = value
}
```

## 双方向binding

`r1 <-> value`は実行前の`value`をregisterへ読み込み、正常継続時にregister値を`value`へ書き戻します。

## targetと命令構文

命令本文はKagura v1固有です。\
この例の`ADD`はKagura命令構文に従い、他のtargetに共通するOnd構文ではありません。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、bindingは[inline assembly仕様](../../../../specs/ond/language/v1/inline-assembly.md)、命令形式は[Kagura v1仕様](../../../../specs/ond/targets/kagura-v1/inline-assembly.md)を参照してください。
