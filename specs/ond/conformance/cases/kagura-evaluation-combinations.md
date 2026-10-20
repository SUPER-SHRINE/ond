# Kagura上の評価・faultと組合せ検査
<!-- ond-spec-id: conformance/cases/kagura-evaluation-combinations -->

共通manifestとmemory領域は[整数実行vector](./kagura-integers.md)に従う。\
[evaluation fixture](../../../../crates/compiler/src/mir_backend/conformance/evaluation.rs)の独自8件は、[評価順](../../language/v1/expressions.md#式と評価順)、[代入snapshot](../../language/v1/statements.md#静的成立条件)、[bounds](../../language/v1/types.md#配列とstruct)、[nil call](../../language/v1/declarations.md#関数)から期待を導く。\
完全sourceはfixtureにあり、表の条件はその入力・観測点を示す。\
fault caseは後続effectを実行せず、出力を保持して停止する。

| Case ID | 要件 | Source条件 | INPUT | 期待OUTPUT | 期待停止 |
| --- | --- | --- | --- | --- | --- |
| <code>DECL-03.discard</code> | <code>DECL-03, EVAL-01</code> | <code>mark(1)、pair(mark(2))で3/4/5を評価し中間値4を保持、その後mark(6)</code> | <code>なし</code> | <code>[123456, 4]</code> | <code>return</code> |
| <code>EVAL-01.callee</code> | <code>DECL-03, EVAL-01</code> | <code>target()でmark(1)、引数mark(2)/mark(3)、fが2+3を返す</code> | <code>なし</code> | <code>[123, 5]</code> | <code>return</code> |
| <code>NIL-02.call</code> | <code>NIL-02, DECL-03, BOUND-01</code> | <code>var f:func(); f()、後続のOUTPUT+16書込み</code> | <code>[0]</code> | <code>SENTINELを5word保持</code> | <code>BusFault</code> |
| <code>DECL-03.discard-fault</code> | <code>NIL-02, DECL-03, BOUND-01</code> | <code>INPUTをx:u32として読み、_=(1 as u32)/x</code> | <code>[0]</code> | <code>SENTINELを5word保持</code> | <code>InvalidInstruction</code> |
| <code>BOUND-01.negative</code> | <code>NIL-02, DECL-03, BOUND-01</code> | <code>OUTPUT+4の*[2]u32へsigned indexで9を書込む</code> | <code>[0xffffffff]</code> | <code>SENTINELを5word保持</code> | <code>InvalidInstruction</code> |
| <code>BOUND-01.length</code> | <code>NIL-02, DECL-03, BOUND-01</code> | <code>OUTPUT+4の*[2]u32へu32 indexで9を書込む</code> | <code>[2]</code> | <code>SENTINELを5word保持</code> | <code>InvalidInstruction</code> |
| <code>BOUND-01.empty</code> | <code>NIL-02, DECL-03, BOUND-01</code> | <code>OUTPUT+4の*[0]u32へu32 indexで9を書込む</code> | <code>[0]</code> | <code>SENTINELを5word保持</code> | <code>InvalidInstruction</code> |
| <code>BOUND-01.last</code> | <code>BOUND-01</code> | <code>OUTPUTの*[2]u32へu32 indexで9を書込む</code> | <code>[1]</code> | <code>[SENTINEL, 9]</code> | <code>return</code> |

## 内部MIRを操作する組合せ回帰
<!-- ond-spec-id: conformance/cases/kagura-internal-combinations -->

[combinations fixture](../../../../crates/compiler/src/mir_backend/conformance/combinations.rs)の3件は、localを増やして巨大frameを作る実装回帰である。\
外部観測はABIの引数・戻り値・保存register、stack容量faultとlink slot到達性であり、内部MIR操作を共通実装義務とは扱わない。\
要件tagのCALL/OUTはこの既存suiteのfamily tagであり、要件IDのCALL-01/02とOUT-01/02は[索引](../requirements.md)を参照する。

| Case ID | 既存tag | 入力・検査条件 | 期待結果 |
| --- | --- | --- | --- |
| `ABI.far-large-aggregate-helper` | CALL, MEM-02, OUT | 7引数、aggregate return、遠距離call、large frame、再帰、除算/float helperを組合せ | 正常return、[28,8,9,31,15,7]、SP/callee-save保持 |
| `FRAME.exact-fit` | MEM-02 | stack size=main frame+walk frame×3、walk(2) | 正常return、9 |
| `FRAME.next-call-fault` | MEM-03 | 同stack size、walk(3) | InvalidInstruction、SENTINEL、SPとguard word保持 |

## 共有caseとsuite数
<!-- ond-spec-id: conformance/cases/kagura-shared-inventory -->

Kagura suiteの25浮動小数点bit vectorと25評価fixtureは、[numeric-float-vectors](./numeric-float-vectors.md)と[static-evaluation](./static-evaluation.md)の同一case ID・期待を共有する。\
Kaguraでの実行はこの50件、整数336件、cast93件、独自evaluation8件、内部組合せ3件の合計490観測caseである。\
runner自己検査はfixture実行器の回帰であり、この件数に含める対象は上記の規範入力と内部組合せである。
