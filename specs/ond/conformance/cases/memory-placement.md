# Stackとstartup storageの外部観測
<!-- ond-spec-id: conformance/cases/memory-placement -->

要件は`MEM-02`、case IDは`MEM-02.stack-startup-lifetimes`とする。\
規範定義元は[storage配置と生存期間](../../language/v1/execution.md#メモリ管理)、[globalのObject storage](../../artifacts.md#object)、[defer](../../language/v1/statements.md#defer)、[Kagura配置](../../targets/kagura-v1/machine.md#3-ram-layout)、[data layout](../../targets/kagura-v1/machine.md#5-data-layout)、[callとstack](../../targets/kagura-v1/abi.md#2-call-boundary)である。

## 入力と初期状態
<!-- ond-spec-id: conformance/memory-placement/input -->

Ond Language v1と[固定Kagura profile](../../targets/kagura-v1/README.md#参照するkagura規範)を適用する。\
manifest、通常build、LinkPlan、fresh RAM/CPU、INPUT/OUTPUT、終了通知と正常終了命令は[Kagura共通実行契約](./kagura-integers.md)に従う。\
追加libraryと依存projectは持たず、`main.ond`は次の全文とする。

```ond
package main
var Global: u32 = 3
var Escaped: *u32
var Child: *u32
var Trace: u32
func output() -> *u32 { return 536871168 as u32 as *u32 }
func nested(parent: *u32) {
    var child: u32 = 11
    Child = &child
    output()[2] = Child as u32
    defer {
        *Child += 1
        output()[5] = *Child
        Trace = Trace * 10 + 2
    }
    *parent += child
    Trace = Trace * 10 + 1
}
func visit() -> u32 {
    var local: u32 = 7
    Escaped = &local
    output()[1] = Escaped as u32
    defer {
        *Escaped += 1
        output()[4] = *Escaped
        Trace = Trace * 10 + 4
    }
    nested(&local)
    Trace = Trace * 10 + 3
    return local
}
func main() {
    output()[0] = (&Global) as u32
    var result = visit()
    output()[3] = result
    output()[6] = Trace
    output()[7] = Global
}
```

## 操作と比較
<!-- ond-spec-id: conformance/memory-placement/observations -->

compileとlinkを成功させ、LinkedImageのsection範囲とstack_bottom/topを読み、fresh machineでentryから正常終了まで実行する。\
命令上限は200,000とし、終了通知、SPとcallee-save registerの復元は共通契約で比較する。\
OUTPUTへ完了したu32 storeを観測し、次の条件をすべて照合する。\
各pointerの格納先u32領域は半開区間[p,p+4)として比較し、加算はchecked u32とする。

| 観測 | 期待 |
| --- | --- |
| OUTPUT word 0のpointer G | Gは4-byte aligned。対応するLinkedImageのBss sectionの半開区間に[G,G+4)が包含され、stack予約範囲と分離する |
| OUTPUT word 1へのstoreのpointer L | Lは4-byte aligned。[L,L+4)は[stack_bottom,stack_top)内にあり、このstore完了時のSP以上に位置する |
| OUTPUT word 2へのstoreのpointer C | Cは4-byte aligned。[C,C+4)は[stack_bottom,stack_top)内にあり、このstore完了時のSP以上に位置する。LとCの4byte領域は互いに分離する |
| OUTPUT words 3〜7 | `[18,19,12,1234,3]`とu32 little-endianで完全一致 |

pointer L/Cはそれぞれのinvocation中に記録し、Cの読書きはnestedのdefer完了まで、Lの読書きはvisitのdefer完了までに行われる。\
返値18はvisitのdeferより前に確定し、deferは保持したlocal pointerから値を19へ更新する。\
addressの照合は上記の範囲・alignment・分離条件で行う。\
invocation終了後のL/Cは格納済みの数値として範囲比較に用いる。
