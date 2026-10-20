# Kagura回帰の外部観測
<!-- ond-spec-id: conformance/cases/kagura-regression-observations -->

適用版、manifest、section load、register初期値、正常終了命令と判定手順は[整数実行の共通契約](./kagura-integers.md)に従う。\
本書のLinkPlanは`read_only=null`、memory base=`0x1000`、memory size=`0x10000`、stack size=`0x2000`とし、SP初期値=`0x11000`、stack bottom=`0xf000`へ読み替える。\
OUTPUTは`0xffff8050`から16byteの読み書き可能なzero初期化領域、EXITは`0xffff8040`から4byteで初期値SENTINELとする。\
正常時はEXIT=0、SPとcallee-save registerの初期値保持、OUTPUTの表のu32値を比較する。\
CPU命令上限は200,000とし、各入力はfresh machineで実行する。

## Heapの保証
<!-- ond-spec-id: conformance/kagura-regression/heap -->

要件は`MEM-01`、規範定義元は[メモリ管理](../../language/v1/execution.md#メモリ管理)と[heap容量](../../targets/kagura-v1/machine.md#31-heap-allocator)とする。\
次のtemplateのDECLARATIONSを表の入力へ置換して`main.ond`を構成する。

```ond
package main
DECLARATIONS
func main() { *(4294934608 as u32 as *u32) = (calc()) as u32 }
```

| 回帰識別名・入力 | DECLARATIONS | OUTPUT |
| --- | --- | --- |
| <code>allocation_zeroing_reuse_and_explicit_initialization_execute/1</code> | <code>type S struct{a:u8;b:u16;c:[2]u32;}<br>var G:*S<br>func calc()-&gt;u32{G=new(S);if G==nil{return 999};var value=G.a as u32+G.b as u32+G.c[1];G.c[0]=77;value=value+G.c[0];free(G);return value}</code> | 77 |
| <code>allocation_zeroing_reuse_and_explicit_initialization_execute/3</code> | <code>type Z struct{}<br>func calc()-&gt;u32{var a=new(Z);var b=new([0]u16);var c=alloc[u8](0);free(c);var ok:u32;if a!=nil &amp;&amp; b!=nil &amp;&amp; c==nil{ok=1};free(a);free(b);return ok}</code> | 1 |
| <code>failed_allocation_and_overflow_preserve_existing_memory/1</code> | <code>func calc()-&gt;u32{var a=alloc[u8](8);a[0]=42;var b=alloc[u8](4294967295);var c=alloc[u8](65536);var d=new([65536]u8);var result=a[0] as u32;if b==nil &amp;&amp; c==nil &amp;&amp; d==nil{result=result+1};free(a);return result}</code> | 43 |

`failed_allocation_and_overflow_preserve_existing_memory/2`のu8 allocationに対する4-byte address剰余比較、`allocation_zeroing_reuse_and_explicit_initialization_execute/2`の解放address再利用比較、および`first_fit_splitting_coalescing_and_tail_reclamation_execute`はallocator方式の実装回帰として追跡する。

## Bounds失敗と後続write
<!-- ond-spec-id: conformance/kagura-regression/bounds-before-write -->

要件は`MEM-03`であり、次の`main.ond`を実行する。\
期待は20,000命令以内のInvalidInstruction fault、EXITのSENTINEL保持、OUTPUT=0の保持である。

```ond
package main
func set(i:u32){var a=[2]u8{1,2};a[i]=99;*(4294934608 as u32 as *u32) = 123}
func main(){set(2)}
```

## 静的byte storageの読取り
<!-- ond-spec-id: conformance/kagura-regression/static-storage -->

要件は`OUT-01`であり、規範定義元は[文字列と定数storage](../../language/v1/expressions.md#文字列リテラル)、[宣言](../../language/v1/declarations.md#宣言)、[data layout](../../targets/kagura-v1/machine.md#5-data-layout)とする。\
各行は`main.ond`の全sourceであり、OUTPUTをu32として比較する。

| main.ond | OUTPUT |
| --- | --- |
| <code>package main<br>func text()-&gt;*u8{return &amp;&quot;abc&quot;[0]}<br>func calc()-&gt;u32{var p=text();return p[0] as u32*10000+p[1] as u32*100+p[2] as u32}<br>func main(){*(4294934608 as u32 as *u32)=(calc()) as u32}<br></code> | 979899 |
| <code>package main<br>const Data=[3]u16{10,20,30}<br>func data()-&gt;*u16{return &amp;Data[0]}<br>func calc()-&gt;u32{var p=data();return Data[1] as u32+p[2] as u32}<br>func main(){*(4294934608 as u32 as *u32)=(calc()) as u32}<br></code> | 50 |

追加のCore入力は次の`main.ond`とし、Messageの論理値が`[3]u8`、bytes=`61 62 63`であることを照合する。\
Kagura Objectではこの定数の読み取り専用storageを、対応する定義symbolのsectionとoffsetで特定し、offsetからのbytes=`61 62 63 00`（最後はhidden sentinel）、symbolの論理size=3を照合する。

```ond
package main
const Message="abc"
func pointer()->*u8{return &Message[0]}
func main(){}
```

## Layoutの実装追跡と外部case
<!-- ond-spec-id: conformance/kagura-regression/layout-tracking -->

`layout_and_abi_have_independent_fixed_expectations`は内部layout関数への入力と結果を検査する実装回帰識別名である。\
外部のlayout値は[BI-04](./layout-operators.md)、ABIは[引数](../../targets/kagura-v1/abi.md#4-arguments)と[戻り値](../../targets/kagura-v1/abi.md#5-return-values)、local storageの寿命と配置のcaseは[MEM-02](./memory-placement.md)を参照する。

`global_objects_use_checked_natural_bss_layout_and_abs32_relocations`のsection選択、正確offset、relocation命令選択とinitializer構造は実装回帰の追跡対象とする。\
外部のglobal値と初期化は[global入力](./regression-observations.md#globals_are_zeroed_or_initialized_from_compile_time_values)、外部Object条件は[成果物](../../artifacts.md#object)を定義元とする。
