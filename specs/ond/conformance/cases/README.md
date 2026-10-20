# 適合case索引
<!-- ond-spec-id: conformance/cases-index -->

各family文書は既存のcase ID、完全なsource入力、要件ID、仕様から導く受理・拒否・値期待を対応づける。\
入力とExpectedの宣言データを移行し、compilerの評価結果を期待値へ写す方法は用いていない。

## Case family
<!-- ond-spec-id: conformance/case-family-inventory -->

| Family | 登録case数 | 入力・期待値表 |
| --- | ---: | --- |
| boundaries-builtins | 67 | [boundaries-builtins](./boundaries-builtins.md) |
| boundaries-conversions | 866 | [boundaries-conversions](./boundaries-conversions.md) |
| boundaries-declarations | 60 | [boundaries-declarations](./boundaries-declarations.md) |
| boundaries-operators | 322 | [boundaries-operators](./boundaries-operators.md) |
| numeric-casts | 264 | [numeric-casts](./numeric-casts.md) |
| numeric-float-vectors | 25 | [numeric-float-vectors](./numeric-float-vectors.md) |
| numeric-floats | 69 | [numeric-floats](./numeric-floats.md) |
| numeric-integers | 392 | [numeric-integers](./numeric-integers.md) |
| numeric-runtime | 1 | [numeric-runtime](./numeric-runtime.md) |
| static-constant-contexts | 25 | [static-constant-contexts](./static-constant-contexts.md) |
| static-control-headers | 26 | [static-control-headers](./static-control-headers.md) |
| static-diagnostics | 6 | [static-diagnostics](./static-diagnostics.md) |
| static-evaluation | 25 | [static-evaluation](./static-evaluation.md) |
| static-flow | 25 | [static-flow](./static-flow.md) |
| static-name-duplicates | 16 | [static-name-duplicates](./static-name-duplicates.md) |
| static-names | 19 | [static-names](./static-names.md) |
| static-precedence | 2 | [static-precedence](./static-precedence.md) |
| static-scope | 32 | [static-scope](./static-scope.md) |
| static-seed | 5 | [static-seed](./static-seed.md) |

numericは751件（integer392、cast264、float69、float bits25、dynamic1）、型境界は1,315件（conversion866、operator322、builtin67、declaration60）、静的fixtureは181件である。\
静的fixtureにMemoryVm用evaluation25件を含む。\
compiler-core登録分の一意なcase IDは合計2,247件であり、各family表のID・要件列・source・Expectedを抽出元宣言と往復照合した。\
元registryはそのまま保持し、移行の最終Rust差分は規範索引への参照pathである。

## 入力と期待結果の読み方
<!-- ond-spec-id: conformance/case-contract -->

Parseはtokenとgrammar、Coreは名前・型・定数、MemoryVmは限定した実行fixtureを照合する。\
各caseの完全入力にはfile名とsourceを示す。\
manifestとsource file名は各表の共通前提および例外に示す。\
layer、診断severity・個数・位置は各表から参照するregistry宣言で特定する。\
規範本文の意味から導く期待値と、compiler内部AST/HIR/MIRの形を検査するcallbackを区分する。\
整数期待値のbitsはu64、signed値は符号拡張したtwo's-complement表現、f32値はIEEE binary32のu32 bitsである。

## 外部allocatorと実装回帰
<!-- ond-spec-id: conformance/case-implementation-boundary -->

`first_fit_splitting_coalescing_and_tail_reclamation_execute`は[実装heap回帰](../../../../crates/compiler/src/mir_backend/heap_tests.rs)の識別子として保持する。\
first-fit、block分割・結合、末尾回収の実装方式を検査し、外部allocatorの適合範囲は[heap契約](../../targets/kagura-v1/machine.md#31-heap-allocator)に定める成功領域、alignment、存続allocationの分離、初期化、failure/free結果とする。\
AST/MIR callbackが主題のcaseは各表でimplementationと表示する。\
`FP-02.numeric-dynamic-mir-contracts`とzero-storage callbackは内部opcode・guard・storageを検査し、実行結果の適合は各MemoryVmまたはtarget回帰の入力・期待と区別する。

移行照合記録: sorted `(case ID, requirement ID列, source, Expected)` JSON tuple列のSHA-256は`65138e994d683fc1dcc79b7860a7a2139752c710cd3f19d6c9fabfbdacdb963e`である。

## Kagura backendの実行入力
<!-- ond-spec-id: conformance/kagura-case-inventory -->

- [整数336件](./kagura-integers.md)
- [数値変換93件](./kagura-casts.md)
- [評価/fault8件、内部組合せ3件、共有50件](./kagura-evaluation-combinations.md)

backend独自440件のIDは内部で一意であり、core登録2,247件との交差は0件であることを集合照合した。\
共有の浮動小数点25件と評価25件を含め、backend suiteは490観測caseとなる。\
core登録とbackend独自を合わせた一意IDは2,687件である。\
件数はこの二つのsuiteを対象とし、他の回帰testを含むrepository全検査のcase数とは区別する。
