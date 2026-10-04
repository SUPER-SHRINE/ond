# Ond ABI for Kagura v1

本書は Ond for Kagura v1 の関数呼び出し規約を定義する。値の memory layout は [Ond for Kagura Machine Profile v1](./machine.md) に従う。

## 1. Registers

| Register | Role | Preservation |
| --- | --- | --- |
| `r0` | zero | fixed |
| `r1..r6` | argument / result | caller-save |
| `r7..r11` | general purpose | callee-save |
| `r12..r13` | temporary | caller-save |
| `r14` | stack pointer | fixed role |
| `r15` | link register | caller-save |

callee が使用した `r7..r11` は return 前に元の値へ戻す。caller は call を越えて必要な caller-save register を保存する。

MIR backendの関数内分岐拡張は、taken経路で`r12/r13`をscratchとして使用する。backendはこれらに分岐を越えるlive値を置かない。
Objectの`CallSlot`/`JumpSlot`によるlinker thunkは`r12`だけを変更し、`r1..r11`、`r13`、SP、元のcallが設定した`r15`を保持する。
`JumpSlot`の生成元もtaken経路の`r12`をdeadにする必要がある。通常のcall規約だけから関数内jumpでのclobberを推論してはならない。
形式・予約領域・return経路は[backend契約 §11](./mir-backend.md#11-長距離分岐とobjectの中継slot)を参照する。

## 2. Call Boundary

- stack は full descending とする。
- call 直前と callee entry の `r14` は 8-byte aligned とする。
- callee entry 時の `r14` を `CFA` と呼ぶ。
- return address は `r15` に置く。
- stack 上の ABI word は 4 byte とする。
- stack object の alignment は `min(natural_alignment, 4)` とする。

## 3. Value Classification

| Class | Values | ABI words |
| --- | --- | ---: |
| Direct-1 | integer, `f32`, `bool`, raw pointer, function pointer | 1 |
| Indirect | array, struct, interface | pointer 1 |

Direct-1 のうち integer、`bool`、pointer は値を下位 bit に置き、未使用の上位 bit を `0` にする。`f32` は IEEE 754 binary32 の 32-bit pattern をそのまま置く。

Indirect argument では caller が値を caller-owned temporary へコピーし、その先頭 pointer を渡す。callee は call 中に temporary を変更できるが、return 後に pointer を保持してはならない。caller の元の値は変更されない。

interfaceのmemory valueはdata pointer、vtable pointerの順の2 word aggregateである。interface methodのvtable entryは4-byte function pointerで、呼出し時はdata pointerを消去型`*u8`の第1引数として渡し、残りをsource methodの引数順に続ける。concrete pointer型との違いはABI上はいずれもDirect-1 pointerであり、entry function内では元のreceiver型として扱う。

## 4. Arguments

source-level argument を宣言順に ABI word 列へ変換する。

- Direct-1 は 1 word を追加する。
- Indirect は temporary の pointer 1 word を追加する。

先頭 6 word を `r1..r6` に置き、残りを `CFA + 0`, `CFA + 4`, ... に置く。stack argument area は caller が確保し、全体を 8-byte 境界へ切り上げる。

memory return を使う関数では、return area の pointer を先頭の hidden argument word とする。user argument はその後ろへ続ける。

## 5. Return Values

戻り値を宣言順に ABI word 列へ変換する。

- すべて Direct で合計 4 word 以下なら `r1..r4` で返す。
- Indirect value を含む場合、または合計 5 word 以上なら、すべての戻り値を caller-owned return area へ書く。

return area は戻り値を field とする record として配置する。Direct-1 は 4-byte slot、Indirect value は通常の memory layout を使い、各値を最大 4-byte の natural alignment へ揃える。

狭いDirect-1のslotも配置alignmentは元の型に従うため、word境界とは限らない（例: `([1]u8, u8, u16, i32)` はoffset 0/1/6/12）。狭いslotの値は型幅で読み書きし、未使用の上位byteは0として書く。未整列のLDW/STWを発行しない。ゼロサイズのIndirect fieldはrecord内でsize=0を維持し、return area全体の物理予約は最低1 byteとする。

## 6. Stack Frame

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

- nested call を行う関数は call 前に return address を保存し、return 前に `r15` へ戻す。
- outgoing argument area は child call に必要な最大サイズを frame 内へ確保し、call ごとに再利用できる。
- memory return area と child call の outgoing argument area は重ねない。
- local scalar slot は 4 byteとし、aggregateは通常のmemory layoutを使う。

標準 prologue と epilogue:

1. `old_sp - frame_size` を checked 計算し、`STACK_BOTTOM` 以上であることを確認する。
2. `r14` から `frame_size` を減算する。
3. 必要な `r15` と callee-save register を保存する。
4. return 前に保存した register を復元する。
5. `r14` に `frame_size` を加算し、`RET` する。

`frame_size == 0` の関数は stack 操作と overflow check を省略できる。

## 7. Stack Overflow

frame を確保する関数は、減算の underflow または `new_sp < STACK_BOTTOM` を検出したら、呼び出しを追加せずその場でCPU trapを実行する。

debug sidecarの対応rangeには`StackOverflow`を記録する。この経路は追加のstackを使わず、returnしない。

## 8. Bounds Error

bounds checkの失敗は呼び出しを追加せず、その場でCPU trapを実行する。追加のstack frameやheap allocationに依存せず、debug sidecarの対応rangeには`BoundsCheck`を記録する。

## 9. Function Pointer

function pointer は 4-byte aligned な 32-bit entry address とする。間接 call は `CALLR register, 0` を使い、direct call と同じ ABI に従う。
