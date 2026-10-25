# Inline assembly適合case
<!-- ond-spec-id: conformance/cases/inline-assembly -->

全caseはOnd Language v1、Kagura v1 targetの単一projectとして実行する。\
既定manifest、source復元、phase比較は[共通case契約](./README.md#入力と期待結果の読み方)に従う。\
Core caseは完全な`main.ond`をCore phaseへ与え、KaguraRun caseはCore検査後の通常buildを[整数実行vectorの配置、起動、終了命令、register初期値](./kagura-integers.md#kagura上の整数実行vector)で実行する。\
KaguraRunのOUTPUTは`0x20000100`から64個のu32 wordとし、指定されないwordはSENTINEL `0xdeadbeef`を保持する。\
KaguraRunの観測対象はKagura CPUの停止状態、OUTPUT、指定device traceとし、命令上限はcase入力の打切り条件とする。\
各caseの期待値は[Ond inline assembly](../../language/v1/inline-assembly.md)および[Kagura v1 inline assembly](../../targets/kagura-v1/inline-assembly.md)から導く。

## Runtimeの値渡し
<!-- ond-spec-id: conformance/cases/inline-assembly-values -->

### ASM-01.input-snapshot-order
<!-- ond-spec-id: conformance/cases/inline-assembly-input-snapshot-order -->

要件は`ASM-01`、`ASM-02`である。次のsourceをKaguraRunで実行する。

```ond
package main
var State:u32
func first() -> u32 { State = State * 10 + 1; return 3 }
func second() -> u32 { State = State * 10 + 2; return 4 }
func main() {
    var result:u32
    asm (r1 <- first(), r2 <- second(), r3 -> result,) {
        ADD r3, r1, r2, 0
    }
    *(536871168 as u32 as *u32) = State * 100 + result
}
```

期待はOUTPUT[0]=1207、正常終了、SP初期値と`r7..r11`の初期値保持である。\
値1207は`first`、`second`の順序でState=12となり、両snapshotの和が7になることを示す。

### ASM-02.right-identifier-r1
<!-- ond-spec-id: conformance/cases/inline-assembly-right-identifier -->

要件は`ASM-01`、`ASM-02`である。`r1`を通常のOnd local identifierとして使う次のsourceをKaguraRunで実行する。

```ond
package main
func main() {
    var r1:u32 = 9
    var result:u32
    asm (r2 <- r1, r3 -> result) { ADD r3, r2, r0, 1 }
    *(536871168 as u32 as *u32) = result
}
```

期待はOUTPUT[0]=10、正常終了、SP初期値と`r7..r11`の初期値保持である。

### ASM-03.inout-narrow-bool-f32
<!-- ond-spec-id: conformance/cases/inline-assembly-inout-narrow-bool-f32 -->

要件は`ASM-02`である。次のsourceをKaguraRunで実行する。

```ond
package main
func signed() -> i8 {
    var value:i8 = -2
    asm (r1 <-> value) { ADD r1, r1, r0, 0 }
    return value
}
func truth() -> bool {
    var value:bool
    asm (r1 -> value) { ADD r1, r0, r0, 2 }
    return value
}
func floating() -> f32 {
    var value:f32 = 1.5
    var result:f32
    asm (r1 <- value, r2 -> result) { ADD r2, r1, r0, 0 }
    return result
}
func main() {
    *(536871168 as u32 as *u32) = signed() as u32
    var truthBits:u32
    if truth() { truthBits = 1 }
    *(536871172 as u32 as *u32) = truthBits
    var floatBits:u32
    asm (r1 <- floating(), r2 -> floatBits) { ADD r2, r1, r0, 0 }
    *(536871176 as u32 as *u32) = floatBits
}
```

期待はOUTPUT[0]=0xfffffffe、OUTPUT[1]=1、OUTPUT[2]=0x3fc00000、正常終了、SP初期値と`r7..r11`の初期値保持である。\
f32 return comparisonはu32 `0x3fc00000` bit patternとする。

### ASM-04.binding-register-and-callee-save
<!-- ond-spec-id: conformance/cases/inline-assembly-binding-register-callee-save -->

要件は`ASM-02`、`ASM-04`である。次のsourceをKaguraRunで実行する。

```ond
package main
func main() {
    var value:u32 = 8
    asm (r1 <- value) { ADD r1, r1, r0, 2 }
    asm { ADD r7, r0, r0, 123 }
    *(536871168 as u32 as *u32) = value
}
```

期待はOUTPUT[0]=8、正常終了、SP初期値と`r7..r11`の初期値保持である。\
first asmはinput registerを変更してもsource変数のbinding値を更新せず、second asmの`r7`変更はABIのcallee-save状態として関数外へ残らない。

### ASM-05.memory-alias-and-order
<!-- ond-spec-id: conformance/cases/inline-assembly-memory-alias-order -->

要件は`ASM-04`である。次のsourceをKaguraRunで実行する。

```ond
package main
func main() {
    var address:u32 = 536870952
    var pointer = address as *u32
    *pointer = 5
    var result:u32
    asm (r1 <- 11, r2 <- address, r3 -> result) {
        STW r1, r2, r0, 0
        LDW r3, r2, r0, 0
    }
    *(536871168 as u32 as *u32) = result + *pointer
}
```

期待はOUTPUT[0]=22、正常終了、SP初期値と`r7..r11`の初期値保持である。\
source memory accessは同一addressをaliasし、store後のloadとOndからの後続readが値11を観測する。

### ASM-06.mmio-write-order
<!-- ond-spec-id: conformance/cases/inline-assembly-mmio-write-order -->

要件は`ASM-04`である。前caseの共通設定を用い、追加で`0xffff8050`の4-byte mappingをTraceDeviceに置換する。\
TraceDeviceは各成功したstoreについて`(address, width, value)`を順に記録し、loadにはzeroを返す。\
次のsourceをKaguraRunで実行する。

```ond
package main
func main() {
    var address:u32 = 4294934608
    asm (r1 <- 1, r2 <- address, r3 <- 2) {
        STW r1, r2, r0, 0
        STW r3, r2, r0, 0
    }
}
```

期待はTraceDevice記録が`[(0xffff8050, 4, 1), (0xffff8050, 4, 2)]`の順、正常終了、SP初期値と`r7..r11`の初期値保持である。\
asm output bindingがない場合も、両storeを観測する。

### ASM-07.noreturn-transfer-skips-defer
<!-- ond-spec-id: conformance/cases/inline-assembly-noreturn-defer -->

要件は`ASM-04`である。次のsourceをKaguraRunで実行し、共通case契約の命令上限2,000,000内で観測する。

```ond
package main
func boot() {
    *(536871168 as u32 as *u32) = 2
    for {}
}
func main() {
    defer { *(536871172 as u32 as *u32) = 1 }
    asm noreturn (r1 <- boot, r2 <- 536871176, r14 <- 8388608) {
        STW r14, r2, r0, 0
        JZ r0, r0, r1, 0
    }
}
```

期待は正常終了通知がなくOUTPUT[0]=2、OUTPUT[1]=SENTINEL、OUTPUT[2]=0x00800000であり、instruction limit到達時に停止する。\
外部移行後に`main`のdeferが実行された場合はOUTPUT[1]=1となり不適合である。\
期待するOUTPUT[1]はSENTINEL `0xdeadbeef`のままとする。

### ASM-08.fault-preserves-completed-store
<!-- ond-spec-id: conformance/cases/inline-assembly-fault-preserves-store -->

要件は`ASM-02`、`ASM-04`である。次のsourceをKaguraRunで実行する。

```ond
package main
var Global:u32 = 9
func main() {
    var faultAddress:u32 = 305419896
    var outputAddress:u32 = 536871168
    asm (r1 <- faultAddress, r2 -> Global, r3 <- 77, r4 <- outputAddress) {
        STW r3, r4, r0, 0
        LDB r2, r1, r0, 0
    }
}
```

期待は`BUS_FAULT`、fault address `0x12345678`、width 1、operation `load`、OUTPUT[0]=77である。\
停止時にlinked image内の公開global `Global`のstorage valueは9のままとし、normal exitは発生しない。\
`STW`完了後のwriteは保持し、faultした`LDB`と未完了output bindingの書戻しは効果を持たない。

## 静的拒否
<!-- ond-spec-id: conformance/cases/inline-assembly-rejections -->

各caseは示したsourceをCore phaseへ与え、`reject`を期待する。\
他のcaseと独立した入力として扱う。\
拒否位置はsource内の該当tokenを示し、診断の主位置はそのtokenを含むUTF-8 byte範囲とする。

| Case ID | 要件 | 完全な`main.ond` | 拒否位置 | 拒否の根拠 |
| --- | --- | --- | --- | --- |
| `ASM-R01.duplicate-register` | `ASM-02` | <code>package main<br>func main(){var x:u32=1;var y:u32;asm(r1&lt;-x,r1-&gt;y){ADD r1,r1,r0,0}}</code> | 2番目の`r1` | register bindingの重複 |
| `ASM-R02.duplicate-output` | `ASM-02` | <code>package main<br>func main(){var x:u32;asm(r1-&gt;x,r2-&gt;x){ADD r1,r0,r0,0;ADD r2,r0,r0,0}}</code> | 2番目のoutput binding `r2-&gt;x`の`x` | output変数の重複 |
| `ASM-R03.invalid-output-type` | `ASM-02` | <code>package main<br>func main(){var x:[1]u32;asm(r1-&gt;x){ADD r1,r0,r0,0}}</code> | `->x`の`x` | aggregateはbinding scalar型ではない |
| `ASM-R04.uninitialized-scratch` | `ASM-03` | <code>package main<br>func main(){var x:u32;asm(r1&lt;-x){ADD r1,r2,r0,0}}</code> | `ADD`の第2 operand `r2` | `r2`をreadする全経路に定義がない |
| `ASM-R05.output-branch-path` | `ASM-03` | <code>package main<br>func main(){var x:u32;var condition:u32=0;asm(r2&lt;-condition,r1-&gt;x){JZ r0,r2,done;ADD r1,r0,r0,1;done:}}</code> | `r1-&gt;x`の`r1` | taken edgeでoutput registerが未定義 |
| `ASM-R06.normal-stack-register` | `ASM-03` | <code>package main<br>func main(){asm{ADD r1,r14,r0,0}}</code> | `ADD`の第2 operand `r14` | 通常asmで`r14`を使用 |
| `ASM-R07.normal-external-jump` | `ASM-03` | <code>package main<br>func main(){var target:u32=0;asm(r1&lt;-target){JZ r0,r0,r1,0}}</code> | `JZ`命令 | 通常asmでexternal indirect jump形式を使用 |
| `ASM-R08.noreturn-fallthrough` | `ASM-04` | <code>package main<br>func main(){asm noreturn{ADD r1,r0,r0,1}}</code> | `asm noreturn` block終端 | block末尾への正常到達経路 |
| `ASM-R09.invalid-shift-immediate` | `ASM-03` | <code>package main<br>func main(){asm{SHIFT r1,r0,r0,32}}</code> | `SHIFT`のimm `32` | SHIFT reserved bit pattern |
| `ASM-R10.invalid-compare-immediate` | `ASM-03` | <code>package main<br>func main(){asm{CMP r1,r0,r0,3}}</code> | `CMP`のimm `3` | CMP reserved comparison encoding |
| `ASM-R11.noreturn-output-binding` | `ASM-04` | <code>package main<br>func main(){var x:u32;asm noreturn(r1-&gt;x){ADD r1,r0,r0,1;JZ r0,r0,r1,0}}</code> | `asm noreturn`のoutput binding `r1-&gt;x` | noreturnはoutput bindingを持たない |
