# asm（Kagura v1）

`asm`はKagura v1向けの命令列を記述し、registerとOnd値の間で値を受け渡します。

```ond title="main.ond"
package main

func main() {
    var input: u32 = 41
    asm (r1 <- input) {
        ADD r1, r1, r0, 1
    }
    _ = input

    var output: u32 = 0
    asm (r1 -> output) {
        ADD r1, r0, r0, 42
    }
    _ = output

    var value: u32 = 41
    asm (r1 <-> value) {
        ADD r1, r1, r0, 1
    }
    _ = value

    var entry: u32 = 0x1000
    asm noreturn (r1 <- entry) {
        JZ r0, r0, r1, 0
    }
}
```

## 入力binding

`r1 <- input`は開始前の`input`をregisterへ読み込みます。\
命令本文で`r1`を書き換えても、入力だけのbindingでは`input`へ書き戻しません。

## 出力binding

`r1 -> output`は正常継続時にregisterの値を`output`へ書き込みます。\
出力専用registerは開始時に未定義なので、例では`ADD`で値`42`を定義してから使います。

## 双方向binding

`r1 <-> value`は実行前の`value`をregisterへ読み込み、正常継続時に値を書き戻します。\
この例では`value`の`41`を読み込み、`42`を書き戻します。

## `asm noreturn`で外部へ移る

`asm noreturn`は直後のOnd codeへ継続しないことを表します。\
上の例では`r1`へ移行先addressを渡し、`JZ`で外部へ移ります。

この例を実行するには、移行先のcodeを`0x1000`へ配置し、そのentryが必要とするstackや引数を用意しておきます。\
この配置や準備は通常のlink配置では自動的に保証されません。\
`asm noreturn`では`defer`が実行されないため、必要なcleanupは移行前に済ませます。

## targetと命令構文

命令本文はKagura v1固有です。\
この例の`ADD`はKagura命令構文に従い、他のtargetに共通するOnd構文ではありません。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、bindingは[inline assembly仕様](../../../../specs/ond/language/v1/inline-assembly.md)、命令形式は[Kagura v1仕様](../../../../specs/ond/targets/kagura-v1/inline-assembly.md)を参照してください。
