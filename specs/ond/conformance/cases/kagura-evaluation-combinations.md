# Kagura上の評価・faultと組合せ検査
<!-- ond-spec-id: conformance/cases/kagura-evaluation-combinations -->

共通manifestとmemory領域は[整数実行vector](./kagura-integers.md)に従う。\
本書の評価caseと共有evaluationは命令上限200,000とし、他の初期状態と比較方法は共通契約に従う。\
各caseは、[評価順](../../language/v1/expressions.md#式と評価順)、[代入snapshot](../../language/v1/statements.md#静的成立条件)、[bounds](../../language/v1/types.md#配列とstruct)、[nil call](../../language/v1/declarations.md#関数)から期待を導く。\
入力欄は`main.ond`の全sourceである。\
fault caseは後続effectを実行せず、表の出力を保持して停止する。\
`NIL-02.call`のBusFaultは、この実行環境でaddress 0がunmappedであることから導くcase固有の期待である。

| Case ID | 要件 | Source条件 | INPUT | 期待OUTPUT | 期待停止 |
| --- | --- | --- | --- | --- | --- |
| <code>DECL-03.discard</code> | <code>DECL-03, EVAL-01</code> | <code>package main<br>var trace:u32<br>func mark(n:u32)-&gt;u32{trace=trace*10+n;return n}<br>func pair(_:u32)-&gt;(u32,u32,u32){return mark(3),mark(4),mark(5)}<br>func main(){_=mark(1);var _,b,_=pair(mark(2));_=mark(6);*(536871168 as u32 as *u32) = trace;*(536871172 as u32 as *u32) = b}</code> | <code>なし</code> | <code>[123456, 4]</code> | <code>return</code> |
| <code>EVAL-01.callee</code> | <code>DECL-03, EVAL-01</code> | <code>package main<br>var trace:u32<br>func mark(n:u32)-&gt;u32{trace=trace*10+n;return n}<br>func f(a:u32,b:u32)-&gt;u32{return a+b}<br>func target()-&gt;func(u32,u32)-&gt;u32{_=mark(1);return f}<br>func main(){var n=target()(mark(2),mark(3));*(536871168 as u32 as *u32) = trace;*(536871172 as u32 as *u32) = n}</code> | <code>なし</code> | <code>[123, 5]</code> | <code>return</code> |
| <code>NIL-02.call</code> | <code>NIL-02, DECL-03, BOUND-01</code> | <code>package main<br>func main(){var f:func();f();*(536871184 as u32 as *u32) = 99}</code> | <code>[0]</code> | <code>SENTINELを5word保持</code> | <code>BusFault</code> |
| <code>DECL-03.discard-fault</code> | <code>NIL-02, DECL-03, BOUND-01</code> | <code>package main<br>func main(){var address:u32=536870912;var input = address as *u32;var x=*input;_=(1 as u32)/x;*(536871184 as u32 as *u32) = 99}</code> | <code>[0]</code> | <code>SENTINELを5word保持</code> | <code>InvalidInstruction</code> |
| <code>BOUND-01.negative</code> | <code>NIL-02, DECL-03, BOUND-01</code> | <code>package main<br>func main(){var p=536871172 as *[2]u32;var address:u32=536870912;var input = address as *u32;var i=(*input) as i32;(*p)[i]=9;*(536871184 as u32 as *u32) = 99}</code> | <code>[0xffffffff]</code> | <code>SENTINELを5word保持</code> | <code>InvalidInstruction</code> |
| <code>BOUND-01.length</code> | <code>NIL-02, DECL-03, BOUND-01</code> | <code>package main<br>func main(){var p=536871172 as *[2]u32;var address:u32=536870912;var input = address as *u32;var i=*input;(*p)[i]=9;*(536871184 as u32 as *u32) = 99}</code> | <code>[2]</code> | <code>SENTINELを5word保持</code> | <code>InvalidInstruction</code> |
| <code>BOUND-01.empty</code> | <code>NIL-02, DECL-03, BOUND-01</code> | <code>package main<br>func main(){var p=536871172 as *[0]u32;var address:u32=536870912;var input = address as *u32;var i=*input;(*p)[i]=9;*(536871184 as u32 as *u32) = 99}</code> | <code>[0]</code> | <code>SENTINELを5word保持</code> | <code>InvalidInstruction</code> |
| <code>BOUND-01.last</code> | <code>BOUND-01</code> | <code>package main<br>func main(){var inputAddress:u32=536870912;var input=inputAddress as *u32;var p=536871168 as *[2]u32;var i=*input;(*p)[i]=9}</code> | <code>[1]</code> | <code>[SENTINEL, 9]</code> | <code>return</code> |

## 内部MIRを操作する組合せ回帰
<!-- ond-spec-id: conformance/cases/kagura-internal-combinations -->

外部の引数・戻り値・保存register、stack容量faultは[ABI](../../targets/kagura-v1/abi.md)と[メモリ管理](../../language/v1/execution.md#メモリ管理)を定義元とする。

内部組合せ検査IDのscopeと対応規範は[Sourceとの管理境界](../../../../AGENTS.md#sourceとの管理境界)に記す。

## 共有caseとsuite数
<!-- ond-spec-id: conformance/cases/kagura-shared-inventory -->

Kagura suiteの25浮動小数点bit vectorと25評価fixtureは、[numeric-float-vectors](./numeric-float-vectors.md)と[static-evaluation](./static-evaluation.md)の同一case ID・期待を共有する。\
Kaguraでの共有評価25件は、[static-evaluation](./static-evaluation.md)の各sourceの`func main() {}`を次の入口へ置換して実行する。\
返値がN個の場合、`var v0,...,vN-1 = test()`として一度評価し、`*(OUTPUT + 4*i as u32 as *u32) = vi as u32`をi昇順で行う`main`を構成する。\
OUTPUTへのcastは加算結果全体へ適用し、返値は各caseの期待列の下位32bitと比較する。\
f32 bit vectorのsource・操作は[numeric-float-vectors](./numeric-float-vectors.md)を参照する。
