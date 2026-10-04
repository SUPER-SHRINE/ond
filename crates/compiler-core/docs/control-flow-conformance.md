# 制御フロー・評価順・名前scopeの適合ケース

2026-09-20追補: evaluationの25ケースを[共有fixture](../src/test_vectors/evaluation.rs)へ抽出した。coreの既存MemoryVm検証はadapter経由で維持し、同じsource・期待trace/値をKaguraでも実行する。仕様・判定は変更していない。[backend側の記録](../../../specs/ond/targets/kagura-v1/conformance-checklist.md)を参照。

対象: EVAL-02、FLOW-02/03、NAME-01/02。既存テストを移設せず、[共通形式](./conformance-cases.md)へ85件を追加した。
空blockのparser不具合修正後、保留していた2件を通常実行へ戻し、対照ケース24件を追加した。合計111ケースで、ignoredはない。

| module | 通常ケース数 | 検証内容 |
| --- | ---: | --- |
| [evaluation.rs](../src/conformance/evaluation.rs) | 25 | sourceで記録した副作用traceと最終値をMIRで実行・照合 |
| [flow.rs](../src/conformance/flow.rs) | 25 | 条件/block/break/continueの診断と、return/終端loopの受理・拒否 |
| [name_duplicates.rs](../src/conformance/name_duplicates.rs) | 16 | packageのvar/const/func/type全4×4組み合わせを別file間で拒否 |
| [names.rs](../src/conformance/names.rs) | 19 | local/parameter重複、scope開始点・終了点、import shadowの負例 |
| [control_headers.rs](../src/conformance/control_headers.rs) | 26 | bare identifier条件＋空block、composite literalとの対照、branch/postの実行結果 |

## 実行結果の照合

新しいMemoryVm層はcompile→MIR validator→既存memory_test_vmの順で実行する。
Ond側の`value`・`index`・`ptr`・`flag`がglobal `Trace`へ番号を追記する。
戻り値の先頭をtrace、後続を最終的な変数・配列要素の値とし、その全体を独立した期待配列と照合する。
traceだけ合って書き込みが間違う場合、または最終値だけ合って順序が間違う場合の両方を検出する。
runnerの負例でもtraceと結果を別々に変更し、誤った期待値が通らないことを確認する。

- 一般operand・引数・複数return値を左から右へ評価する。
- 複数LHSのindex/pointerを先に評価し、その後RHSを評価してから左から右へ書き込む。
- 異なるLHS、同じarray要素へのalias、同じpointerへのaliasを対照する。
- index変数・pointer変数自体を同時に代入しても、先に決定した書き込み先を使う。
- RHSでindexや格納先を変更する場合、RHSをすべて読む前に書き込むと結果が変わるswapを検証する。
- aggregate全体とそのfield/要素の代入順を入れ替え、後の書き込みが勝つことを確認する。
- pointer base→index、複数戻り値call、短絡branch、continue→post→condition、breakでpostを飛ばすことを確認する。
- 入れ子loopのcontinue/breakが最内loopに属すること、return後に副作用が発生しないことを確認する。
- var/const/短い宣言の同時初期値は外側のbindingを参照し、内側scope終了後に元のbindingへ戻る。
- if initは両branchから見え、for initはcondition/body/postから見える。import名をlocal structでshadowし、scope終了後にpackage参照が復元する。

実行器の対応範囲に限定し、小さい整数・bool・array・struct・pointer・direct callのみ使用する。
traceは最大6桁で、i32の範囲内。数値のwrap・float精度・fault処理・Kagura実行を確認したとは扱わない。
実行のstep budgetは既存実行器のものを使う。無限loopの受理ケースはCore層だけで検証し、実行しない。

## 診断と終端判定

負例は診断件数・文言・file・byte spanまで共通runnerで照合する。
parameter重複のprimary spanはparameter全体、それ以外のbinding重複は名前を指す現在の診断に合わせる。

FLOW-03は現在の構造的な判定を明示する。

| sourceの形 | 戻り値を持つ関数での扱い |
| --- | --- |
| 両branchでreturn、またはreturn/終端loop | 受理 |
| 片branchにreturnなし、入れ子の片側に抜け道あり | missing return |
| `for {}` / `for ; ; {}`（外へ出るbreakなし） | 終端として受理 |
| 内側loopだけにbreakがある | 外側の無条件loopは終端 |
| 外側loopへ属するbreakがある | 後続returnが必要 |
| return/continue/内側の無限loopより後のbreak | 終端判定に影響しない |
| `if false { break }`内のbreak | 構造的にbreakありとして扱い、後続returnが必要 |
| 条件付きloop（`for true {}`を含む） | 終端とはみなさず、後続returnが必要 |

条件の定数伝播で終端性を推論する機能を追加したわけではない。

## 空blockとcomposite literalの区別（修正済）

```ond
func test(flag: bool) { if flag {} }
func test(flag: bool) { for flag {} }
```

以前は`flag {}`をnamed composite literalと誤認していた。現在は制御headerの文脈を管理し、これらを空blockとして受理する。
scope終了後の負例も、括弧による回避を外した`if Fresh := true; Fresh {}`で検証する。

文脈処理は[control_syntax.rs](../src/ond/control_syntax.rs)に分離した。
initの代入値、括弧内、call引数、index、composite要素では通常のliteral解析へ戻す。
`S{}.Flag`等のpostfix、後続operator、別のbodyが続く場合はcomposite literalを保持する。
initializer・call/index・入れ子要素・for post・multiline empty block・文脈復帰を対照テストで確認した。
修正に対するユーザー承認を受けて実装し、P1の残件を解消した。

```text
cargo test -p ond-compiler-core conformance
cargo test --workspace --quiet
cargo test -p ond-compiler-core conformance::control_headers
```

空block修正時は全409テスト成功（compiler-core: 302件）、ignoredは0件。
最新の再監査結果と残件は[チェックリスト](./conformance-checklist.md)を参照。
