# Ond ABI for Kagura v1
<!-- ond-spec-id: targets/kagura-v1/abi -->

本書は Ond for Kagura v1 の関数呼び出し規約を定義する。\
値の memory layout は [Ond for Kagura Machine Profile v1](./machine.md) に従う。

## 1. Registers
<!-- ond-spec-id: targets/kagura-v1/abi-registers -->

| Register | Role | Preservation |
| --- | --- | --- |
| `r0` | zero | fixed |
| `r1..r6` | argument / result | caller-save |
| `r7..r11` | general purpose | callee-save |
| `r12..r13` | temporary | caller-save |
| `r14` | stack pointer | fixed role |
| `r15` | link register | caller-save |

callee が使用した `r7..r11` は return 前に元の値へ戻す。\
caller は call を越えて必要な caller-save register を保存する。

関数内の長距離分岐のtaken経路では`r12/r13`がscratchとなり、生成元は分岐を越えるlive値をこれらのregisterから退避する。\
Objectの`CallSlot`/`JumpSlot`によるlinker thunkは`r12`だけを変更し、`r1..r11`、`r13`、SP、元のcallが設定した`r15`を保持する。\
`JumpSlot`の生成元もtaken経路の`r12`をdeadにする必要がある。\
通常のcall規約だけから関数内jumpでのclobberを推論してはならない。\
形式・予約領域・return経路は[linkingのslot契約](../../linking.md#callslotjumpslot-relocation)を参照する。

## 2. Call Boundary
<!-- ond-spec-id: targets/kagura-v1/abi-call-boundary -->

- stack は full descending とする
- loaderが設定する初期r14は[STACK_TOP](./machine.md#3-ram-layout)とし、call直前とcallee entryのr14も8-byte alignedとする
- callee entry 時の `r14` を `CFA` と呼ぶ
- return address は `r15` に置く
- stack 上の ABI word は 4 byte とする
- stack object の alignment は `min(natural_alignment, 4)` とする

## 3. Value Classification
<!-- ond-spec-id: targets/kagura-v1/abi-value-classification -->

| Class | Values | ABI words |
| --- | --- | ---: |
| Direct-1 | integer, `f32`, `bool`, raw pointer, function pointer | 1 |
| Indirect | array, struct | pointer 1 |

Direct-1 のうち integer、`bool`、pointer は値を下位 bit に置き、未使用の上位 bit を `0` にする。\
f32は[言語のbinary32](../../language/v1/expressions.md#floating-point-arithmetic)の32bit patternをそのまま置く。

Indirect argumentではcallerが値をcaller-owned temporaryへcopyし、その先頭pointerを渡す。\
temporaryと渡したpointerの有効期間はcalleeのreturnまでであり、calleeはその期間内にtemporaryを変更できる。\
callerの元の値はcall後も保持する。

## 4. Arguments
<!-- ond-spec-id: targets/kagura-v1/abi-arguments -->

source-level argument を宣言順に ABI word 列へ変換する。

- Direct-1 は 1 word を追加する
- Indirect は temporary の pointer 1 word を追加する

先頭 6 word を `r1..r6` に置き、残りを `CFA + 0`, `CFA + 4`, ... に置く。\
stack argument area は caller が確保し、全体を 8-byte 境界へ切り上げる。

memory return を使う関数では、return area の pointer を先頭の hidden argument word とする。\
user argument はその後ろへ続ける。

## 5. Return Values
<!-- ond-spec-id: targets/kagura-v1/abi-return-values -->

戻り値を宣言順に ABI word 列へ変換する。

- すべて Direct で合計 4 word 以下なら `r1..r4` で返す
- Indirect value を含む場合、または合計 5 word 以上なら、すべての戻り値を caller-owned return area へ書く

return area は戻り値を field とする record として配置する。\
Direct-1 は 4-byte slot、Indirect value は通常の memory layout を使い、各値を最大 4-byte の natural alignment へ揃える。

狭いDirect-1のslotも配置alignmentは元の型に従うため、word境界とは限らない（例: `([1]u8, u8, u16, i32)` はoffset 0/1/6/12）。\
狭いslotの値は型幅で読み書きし、未使用の上位byteは0として書く。\
各値の型幅とalignmentに従ったaccessで読み書きする。\
ゼロサイズのIndirect fieldはrecord内でsize=0を維持し、return area全体の物理予約は最低1 byteとする。

## 6. Stack Frame
<!-- ond-spec-id: targets/kagura-v1/abi-stack-frame -->

callee は `CFA` より低い address に、8-byte の倍数の frame を確保する。

```text
high address
CFA + 0       incoming stack arguments
CFA - 4       saved r15, when needed
              saved callee-save registers
              locals, spills, indirect copies
r14 + 0       outgoing stack arguments
low address
```

- nested call を行う関数は call 前に return address を保存し、return 前に `r15` へ戻す
- outgoing argument area は child call に必要な最大サイズを frame 内へ確保し、call ごとに再利用できる
- memory return area と child call の outgoing argument area は重ねない
- local scalar slot は 4 byteとし、aggregateは通常のmemory layoutを使う

標準 prologue と epilogue:

1. `old_sp - frame_size` を checked 計算し、`STACK_BOTTOM` 以上であることを確認する
2. `r14` から `frame_size` を減算する
3. 必要な `r15` と callee-save register を保存する
4. return 前に保存した register を復元する
5. `r14` に `frame_size` を加算し、`RET` する

`frame_size == 0` の関数は stack 操作と overflow check を省略できる。

## 7. Stack Overflow
<!-- ond-spec-id: targets/kagura-v1/abi-stack-overflow -->

frame を確保する関数は、減算の underflow または `new_sp < STACK_BOTTOM` を検出したら、呼び出しを追加せずその場でCPU trapを実行する。

debug sidecarの対応rangeには`StackOverflow`を記録する。\
この経路は停止時点のstack容量でtrapを完了して実行を終了する。

## 8. Bounds Error
<!-- ond-spec-id: targets/kagura-v1/abi-bounds-error -->

bounds checkの失敗は呼び出しを追加せず、その場でCPU trapを実行する。\
追加のstack frameやheap allocationに依存せず、debug sidecarの対応rangeには`BoundsCheck`を記録する。

## 9. Function Pointer
<!-- ond-spec-id: targets/kagura-v1/abi-function-pointer -->

function pointer は 4-byte aligned な 32-bit entry address とする。\
間接 call は `CALLR register, 0` を使い、direct call と同じ ABI に従う。

## 10. Stack frame容量
<!-- ond-spec-id: targets/kagura-v1/large-stack-frame -->

frameとincoming引数offsetは32-bit checked範囲で表し、signed-16-bitを超えるoffsetも成立する。\
slot予約、alignment、frameとincoming領域の合計が32-bit範囲を超える入力はcompile errorとなる。\
stack addressの計算はSP、既に設定された引数とアクセス対象外のlive値を保持する。

frame確保は32-bitのframe sizeで減算underflowと`STACK_BOTTOM`を検査し、成功後にSPの更新とframeへの書き込みを行う。\
return時はSP、callee-save、`r15`を復元し、`r1..r4`の戻り値を保持する。\
実行時のstack容量不足は[Stack Overflow](#7-stack-overflow)のfaultとなる。\
型のsizeが32-bit範囲に収まることと、配置先の実メモリ容量に収まることはそれぞれ検査する。
