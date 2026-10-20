# 仕様書本移行の照合記録

基準commitは`06e87c39d99f2389af163d0394d9808f2c3078bd`、移行判断は[決裁一覧](./open-questions.md)のSPEC-Q01〜Q09に従う。\
旧言語・build・backend文書の外部規則を概念章へ集約し、crate内部詳細文書は削除した。\
Rust/Ond sourceと既存回帰を内部設計の管理元とする。

## 旧新ID対応

基準commitの既存ond-spec-id全110件を、一意な同一IDとして保持した。\
旧pathは履歴照合用の文字列であり、移行後の定義元をリンク先に示す。\
MIR内部の主題は同じ歴史IDを[実装検証索引](../../specs/ond/conformance/implementation-index.md)へ移し、各項から抽出した外部規則とsourceを追跡できる。

| 既存ID | 基準commitのpath | 移行後の定義元 |
| --- | --- | --- |
| `build/artifacts-and-cache` | `specs/ond/package-build.md` | [成果物とキャッシュ](../../specs/ond/build/cache.md#成果物とキャッシュ) |
| `build/commands` | `specs/ond/package-build.md` | [コマンド](../../specs/ond/build/commands.md#コマンド) |
| `build/compile-command` | `specs/ond/package-build.md` | [ond compile](../../specs/ond/build/commands.md#ond-compile) |
| `build/implemented-units-and-constraints` | `specs/ond/package-build.md` | [コンパイル単位と成果物](../../specs/ond/build/packages.md#コンパイル単位と成果物) |
| `build/io-and-failure` | `specs/ond/package-build.md` | [入出力と失敗時](../../specs/ond/build/commands.md#入出力と失敗時) |
| `build/link-command` | `specs/ond/package-build.md` | [ond link](../../specs/ond/build/commands.md#ond-link) |
| `build/output` | `specs/ond/v1/language.md` | [12. Build Output](../../specs/ond/language/v1/execution.md#build-output) |
| `build/package-artifact-contract` | `specs/ond/package-build.md` | [Package成果物・独立compiler契約](../../specs/ond/build/packages.md#package成果物独立compiler契約) |
| `build/public-metadata-revision-3` | `specs/ond/package-build.md` | [公開interface（旧revision 3の主題）](../../specs/ond/artifacts.md#公開interface旧revision-3の主題) |
| `build/responsibilities` | `specs/ond/package-build.md` | [責務](../../specs/ond/build/packages.md#責務) |
| `build/semantic-cache` | `specs/ond/semantic-cache.md` | [意味解析cache](../../specs/ond/build/cache.md#意味解析cache) |
| `build/semantic-cache-invalidation` | `specs/ond/semantic-cache.md` | [同一性と失効](../../specs/ond/build/cache.md#同一性と失効) |
| `build/semantic-cache-lifetime` | `specs/ond/semantic-cache.md` | [保持期間](../../specs/ond/build/cache.md#保持期間) |
| `build/semantic-cache-visibility` | `specs/ond/semantic-cache.md` | [参照環境](../../specs/ond/build/cache.md#参照環境) |
| `build/test-build` | `specs/ond/test-build.md` | [テストbuild](../../specs/ond/build/test-build.md#テストbuild) |
| `build/test-build-acceptance` | `specs/ond/test-build.md` | [受入確認](../../specs/ond/build/test-build.md#受入確認) |
| `build/test-entrypoints` | `specs/ond/test-build.md` | [Buildと起動入口](../../specs/ond/build/test-build.md#buildと起動入口) |
| `build/test-execution-contract` | `specs/ond/test-build.md` | [起動情報と実行責務](../../specs/ond/build/test-build.md#起動情報と実行責務) |
| `build/test-source-context` | `specs/ond/test-build.md` | [Sourceと文脈](../../specs/ond/build/test-build.md#sourceと文脈) |
| `build/verification` | `specs/ond/package-build.md` | [検証](../../specs/ond/build/cache.md#検証) |
| `builtins/alloc` | `specs/ond/v1/builtins.md` | [2. `alloc`](../../specs/ond/language/v1/builtins.md#2-alloc) |
| `builtins/compile-errors` | `specs/ond/v1/builtins.md` | [10. よくある compile error](../../specs/ond/language/v1/builtins.md#10-よくある-compile-error) |
| `builtins/free` | `specs/ond/v1/builtins.md` | [3. `free`](../../specs/ond/language/v1/builtins.md#3-free) |
| `builtins/functions` | `specs/ond/v1/builtins.md` | [組み込み関数](../../specs/ond/language/v1/builtins.md#組み込み関数) |
| `builtins/index` | `specs/ond/v1/builtins.md` | [1. 一覧](../../specs/ond/language/v1/builtins.md#1-一覧) |
| `builtins/len` | `specs/ond/v1/builtins.md` | [4. `len`](../../specs/ond/language/v1/builtins.md#4-len) |
| `builtins/len-arrays` | `specs/ond/v1/builtins.md` | [4.1 配列に対する `len`](../../specs/ond/language/v1/builtins.md#41-配列に対する-len) |
| `builtins/name-resolution` | `specs/ond/v1/builtins.md` | [9. 名前解決上の注意](../../specs/ond/language/v1/builtins.md#9-名前解決上の注意) |
| `builtins/new` | `specs/ond/v1/builtins.md` | [5. `new`](../../specs/ond/language/v1/builtins.md#5-new) |
| `builtins/related-specifications` | `specs/ond/v1/builtins.md` | [11. 関連仕様](../../specs/ond/language/v1/builtins.md#11-関連仕様) |
| `builtins/syntax` | `specs/ond/v1/builtins.md` | [組み込み構文](../../specs/ond/language/v1/builtins.md#組み込み構文) |
| `builtins/this-file` | `specs/ond/v1/builtins.md` | [6. `thisFile`](../../specs/ond/language/v1/builtins.md#6-thisfile) |
| `builtins/this-line` | `specs/ond/v1/builtins.md` | [7. `thisLine`](../../specs/ond/language/v1/builtins.md#7-thisline) |
| `builtins/trap` | `specs/ond/v1/builtins.md` | [8. `trap`](../../specs/ond/language/v1/builtins.md#8-trap) |
| `builtins/v1` | `specs/ond/v1/builtins.md` | [Built-ins v1](../../specs/ond/language/v1/builtins.md#built-ins-v1) |
| `debug-metadata/artifacts` | `specs/ond/debug-metadata.md` | [1. 成果物](../../specs/ond/debug-metadata/v1.md#1-成果物) |
| `debug-metadata/function-and-unwind` | `specs/ond/debug-metadata.md` | [3. functionとunwind](../../specs/ond/debug-metadata/v1.md#3-functionとunwind) |
| `debug-metadata/source-and-fault-range` | `specs/ond/debug-metadata.md` | [2. sourceとfault range](../../specs/ond/debug-metadata/v1.md#2-sourceとfault-range) |
| `debug-metadata/v1` | `specs/ond/debug-metadata.md` | [Ond Debug Metadata v1](../../specs/ond/debug-metadata/v1.md#ond-debug-metadata-v1) |
| `debug-metadata/vm-responsibilities` | `specs/ond/debug-metadata.md` | [4. VM実装の責務](../../specs/ond/debug-metadata/v1.md#4-vm実装の責務) |
| `expressions/floating-point-arithmetic` | `specs/ond/v1/language.md` | [Floating-Point Arithmetic](../../specs/ond/language/v1/expressions.md#floating-point-arithmetic) |
| `expressions/integer-arithmetic` | `specs/ond/v1/language.md` | [Integer Arithmetic](../../specs/ond/language/v1/expressions.md#integer-arithmetic) |
| `expressions/integer-constant-arithmetic` | `specs/ond/v1/language.md` | [Integer Constant Arithmetic](../../specs/ond/language/v1/expressions.md#integer-constant-arithmetic) |
| `expressions/overview` | `specs/ond/v1/language.md` | [式と評価順](../../specs/ond/language/v1/expressions.md#式と評価順) |
| `functions/declarations-and-calls` | `specs/ond/v1/language.md` | [関数](../../specs/ond/language/v1/declarations.md#関数) |
| `functions/user-defined-operators` | `specs/ond/v1/language.md` | [User-defined operator](../../specs/ond/language/v1/expressions.md#user-defined-operator) |
| `language/declarations` | `specs/ond/v1/language.md` | [宣言](../../specs/ond/language/v1/declarations.md#宣言) |
| `language/semantic-id-policy` | `specs/ond/v1/README.md` | [Semantic ID](../../specs/ond/language/v1/README.md#semantic-id) |
| `language/source-and-packages` | `specs/ond/v1/language.md` | [Sourceとproject](../../specs/ond/language/v1/names-and-scopes.md#sourceとproject) |
| `language/v1` | `specs/ond/v1/language.md` | [適用範囲](../../specs/ond/language/v1/README.md#適用範囲) |
| `language/v1-index` | `specs/ond/v1/README.md` | [Ond Language v1](../../specs/ond/language/v1/README.md#ond-language-v1) |
| `literals/strings` | `specs/ond/v1/language.md` | [文字列リテラル](../../specs/ond/language/v1/expressions.md#文字列リテラル) |
| `runtime/initialization-and-entry` | `specs/ond/v1/language.md` | [11. 初期化とエントリ](../../specs/ond/language/v1/execution.md#初期化とエントリ) |
| `runtime/memory-management` | `specs/ond/v1/language.md` | [10. メモリ管理](../../specs/ond/language/v1/execution.md#メモリ管理) |
| `statements/control-flow` | `specs/ond/v1/language.md` | [文と制御構文](../../specs/ond/language/v1/statements.md#文と制御構文) |
| `statements/defer` | `specs/ond/v1/language.md` | [`defer`](../../specs/ond/language/v1/statements.md#defer) |
| `syntax/grammar` | `specs/ond/v1/syntax.md` | [文法の定義元](../../specs/ond/language/v1/lexical.md#文法の定義元) |
| `syntax/grammar-notes` | `specs/ond/v1/syntax.md` | [構文と静的条件の対応](../../specs/ond/language/v1/declarations.md#構文と静的条件の対応) |
| `syntax/operator-precedence` | `specs/ond/v1/syntax.md` | [演算子の優先順位](../../specs/ond/language/v1/expressions.md#演算子の優先順位) |
| `syntax/packages-and-imports` | `specs/ond/v1/syntax.md` | [Packageとimport](../../specs/ond/language/v1/names-and-scopes.md#packageとimport) |
| `syntax/scope-and-shadowing` | `specs/ond/v1/syntax.md` | [Scopeとshadowing](../../specs/ond/language/v1/names-and-scopes.md#scopeとshadowing) |
| `syntax/semicolons-and-line-breaking` | `specs/ond/v1/syntax.md` | [Semicolonと改行](../../specs/ond/language/v1/lexical.md#semicolonと改行) |
| `syntax/source-form` | `specs/ond/v1/syntax.md` | [Source形式](../../specs/ond/language/v1/lexical.md#source形式) |
| `syntax/v1` | `specs/ond/v1/syntax.md` | [字句とsource形式](../../specs/ond/language/v1/lexical.md#字句とsource形式) |
| `targets/kagura-v1/abi` | `specs/ond/targets/kagura-v1/abi.md` | [Ond ABI for Kagura v1](../../specs/ond/targets/kagura-v1/abi.md#ond-abi-for-kagura-v1) |
| `targets/kagura-v1/abi-arguments` | `specs/ond/targets/kagura-v1/abi.md` | [4. Arguments](../../specs/ond/targets/kagura-v1/abi.md#4-arguments) |
| `targets/kagura-v1/abi-bounds-error` | `specs/ond/targets/kagura-v1/abi.md` | [8. Bounds Error](../../specs/ond/targets/kagura-v1/abi.md#8-bounds-error) |
| `targets/kagura-v1/abi-call-boundary` | `specs/ond/targets/kagura-v1/abi.md` | [2. Call Boundary](../../specs/ond/targets/kagura-v1/abi.md#2-call-boundary) |
| `targets/kagura-v1/abi-function-pointer` | `specs/ond/targets/kagura-v1/abi.md` | [9. Function Pointer](../../specs/ond/targets/kagura-v1/abi.md#9-function-pointer) |
| `targets/kagura-v1/abi-registers` | `specs/ond/targets/kagura-v1/abi.md` | [1. Registers](../../specs/ond/targets/kagura-v1/abi.md#1-registers) |
| `targets/kagura-v1/abi-return-values` | `specs/ond/targets/kagura-v1/abi.md` | [5. Return Values](../../specs/ond/targets/kagura-v1/abi.md#5-return-values) |
| `targets/kagura-v1/abi-stack-frame` | `specs/ond/targets/kagura-v1/abi.md` | [6. Stack Frame](../../specs/ond/targets/kagura-v1/abi.md#6-stack-frame) |
| `targets/kagura-v1/abi-stack-overflow` | `specs/ond/targets/kagura-v1/abi.md` | [7. Stack Overflow](../../specs/ond/targets/kagura-v1/abi.md#7-stack-overflow) |
| `targets/kagura-v1/abi-value-classification` | `specs/ond/targets/kagura-v1/abi.md` | [3. Value Classification](../../specs/ond/targets/kagura-v1/abi.md#3-value-classification) |
| `targets/kagura-v1/address-space` | `specs/ond/targets/kagura-v1/machine.md` | [2. Address Space](../../specs/ond/targets/kagura-v1/machine.md#2-address-space) |
| `targets/kagura-v1/call-jump-slot-relocation` | `specs/ond/targets/kagura-v1/mir-backend.md` | [CallSlot/JumpSlot relocation](../../specs/ond/linking.md#callslotjumpslot-relocation) |
| `targets/kagura-v1/data-layout` | `specs/ond/targets/kagura-v1/machine.md` | [5. Data Layout](../../specs/ond/targets/kagura-v1/machine.md#5-data-layout) |
| `targets/kagura-v1/function-local-branches` | `specs/ond/targets/kagura-v1/mir-backend.md` | [関数内branch拡張](../../specs/ond/conformance/implementation-index.md#関数内branch拡張) |
| `targets/kagura-v1/heap-allocator` | `specs/ond/targets/kagura-v1/machine.md` | [3.1 Heap allocator](../../specs/ond/targets/kagura-v1/machine.md#31-heap-allocator) |
| `targets/kagura-v1/index` | `specs/ond/targets/kagura-v1/README.md` | [Ond for Kagura v1](../../specs/ond/targets/kagura-v1/README.md#ond-for-kagura-v1) |
| `targets/kagura-v1/large-stack-frame` | `specs/ond/targets/kagura-v1/mir-backend.md` | [10. Stack frame容量](../../specs/ond/targets/kagura-v1/abi.md#10-stack-frame容量) |
| `targets/kagura-v1/long-branch-relocation-slots` | `specs/ond/targets/kagura-v1/mir-backend.md` | [固定長slot規範](../../specs/ond/linking.md#固定長slot規範) |
| `targets/kagura-v1/machine-profile` | `specs/ond/targets/kagura-v1/machine.md` | [1. Machine Profile](../../specs/ond/targets/kagura-v1/machine.md#1-machine-profile) |
| `targets/kagura-v1/mir-abi-lowering` | `specs/ond/targets/kagura-v1/mir-backend.md` | [引数・戻り値とframeの実装](../../specs/ond/conformance/implementation-index.md#引数戻り値とframeの実装) |
| `targets/kagura-v1/mir-backend` | `specs/ond/targets/kagura-v1/mir-backend.md` | [backend入口とtarget境界](../../specs/ond/conformance/implementation-index.md#backend入口とtarget境界) |
| `targets/kagura-v1/mir-backend-architecture` | `specs/ond/targets/kagura-v1/mir-backend.md` | [backend内部構成](../../specs/ond/conformance/implementation-index.md#backend内部構成) |
| `targets/kagura-v1/mir-backend-io-and-responsibilities` | `specs/ond/targets/kagura-v1/mir-backend.md` | [検証済み入力とsource診断](../../specs/ond/conformance/implementation-index.md#検証済み入力とsource診断) |
| `targets/kagura-v1/mir-backend-object-contract` | `specs/ond/targets/kagura-v1/mir-backend.md` | [Object生成実装](../../specs/ond/conformance/implementation-index.md#object生成実装) |
| `targets/kagura-v1/mir-boundary-decisions` | `specs/ond/targets/kagura-v1/mir-backend.md` | [ゼロサイズとaccessの境界](../../specs/ond/targets/kagura-v1/machine.md#ゼロサイズとaccessの境界) |
| `targets/kagura-v1/mir-data-layout` | `specs/ond/targets/kagura-v1/mir-backend.md` | [型配置の実装](../../specs/ond/conformance/implementation-index.md#型配置の実装) |
| `targets/kagura-v1/mir-instruction-lowering` | `specs/ond/targets/kagura-v1/mir-backend.md` | [命令選択と演算実装](../../specs/ond/conformance/implementation-index.md#命令選択と演算実装) |
| `targets/kagura-v1/mir-intrinsic-global-startup` | `specs/ond/targets/kagura-v1/mir-backend.md` | [global配置とstartupの実装](../../specs/ond/conformance/implementation-index.md#global配置とstartupの実装) |
| `targets/kagura-v1/mir-long-distance-control-flow` | `specs/ond/targets/kagura-v1/mir-backend.md` | [長距離callと分岐](../../specs/ond/linking.md#長距離callと分岐) |
| `targets/kagura-v1/mir-ordered-aggregate` | `specs/ond/targets/kagura-v1/mir-backend.md` | [3.3 Ordered aggregate access](../../specs/ond/targets/kagura-v1/machine.md#33-ordered-aggregate-access) |
| `targets/kagura-v1/mir-zero-sized-types` | `specs/ond/targets/kagura-v1/mir-backend.md` | [3.2 ゼロサイズとaggregate access](../../specs/ond/targets/kagura-v1/machine.md#32-ゼロサイズとaggregate-access) |
| `targets/kagura-v1/numeric-and-allocator-lowering` | `specs/ond/targets/kagura-v1/mir-backend.md` | [numeric helperとallocator方式](../../specs/ond/conformance/implementation-index.md#numeric-helperとallocator方式) |
| `targets/kagura-v1/project-configuration` | `specs/ond/targets/kagura-v1/machine.md` | [7. Project Configuration](../../specs/ond/targets/kagura-v1/machine.md#7-project-configuration) |
| `targets/kagura-v1/ram-layout` | `specs/ond/targets/kagura-v1/machine.md` | [3. RAM Layout](../../specs/ond/targets/kagura-v1/machine.md#3-ram-layout) |
| `targets/kagura-v1/raw-pointer-access` | `specs/ond/targets/kagura-v1/machine.md` | [6. Raw Pointer Access](../../specs/ond/targets/kagura-v1/machine.md#6-raw-pointer-access) |
| `targets/kagura-v1/relocation-verification` | `specs/ond/targets/kagura-v1/mir-backend.md` | [relocation実装回帰](../../specs/ond/conformance/implementation-index.md#relocation実装回帰) |
| `targets/kagura-v1/runtime-contract` | `specs/ond/targets/kagura-v1/machine.md` | [Ond for Kagura Target Runtime Contract](../../specs/ond/targets/kagura-v1/machine.md#ond-for-kagura-target-runtime-contract) |
| `targets/kagura-v1/runtime-helper-supply` | `specs/ond/targets/kagura-v1/mir-backend.md` | [runtime helper供給と依存閉包](../../specs/ond/conformance/implementation-index.md#runtime-helper供給と依存閉包) |
| `targets/kagura-v1/standard-path-responsibilities` | `specs/ond/targets/kagura-v1/mir-backend.md` | [compiler APIの接続](../../specs/ond/conformance/implementation-index.md#compiler-apiの接続) |
| `targets/kagura-v1/startup` | `specs/ond/targets/kagura-v1/machine.md` | [4. Startup](../../specs/ond/targets/kagura-v1/machine.md#4-startup) |
| `type-system/arrays-and-structs` | `specs/ond/v1/language.md` | [配列とstruct](../../specs/ond/language/v1/types.md#配列とstruct) |
| `type-system/methods` | `specs/ond/v1/language.md` | [Method](../../specs/ond/language/v1/types.md#method) |
| `type-system/nil-and-pointers` | `specs/ond/v1/language.md` | [`nil`とpointer](../../specs/ond/language/v1/types.md#nilとpointer) |
| `type-system/type-identity` | `specs/ond/v1/language.md` | [型同一性](../../specs/ond/language/v1/types.md#型同一性) |
| `type-system/type-parameters` | `specs/ond/v1/language.md` | [型parameter](../../specs/ond/language/v1/types.md#型parameter) |
| `type-system/types` | `specs/ond/v1/language.md` | [型](../../specs/ond/language/v1/types.md#型) |

## 承認済み保証範囲と内部管理

SPEC-Q04のallocator方式変更は承認済みの保証範囲変更である。\
成功領域のsize/alignment/存続allocationとの分離、zero初期化、failureとfree結果をmachine章へ残し、first-fit/header/分割/結合/cursorはheap.ondと実装回帰で管理する。\
SPEC-Q05のMIR内部型/phase/helper/命令選択/大offset scratch/要素展開はsource管理とし、外部の評価順・fault・zero-size/aggregate access・ABI・24-byte CallSlot/JumpSlot契約を各規範へ抽出した。\
NOTE-INIT-01はglobalのzero/constantと通常関数initを定義元に揃え、旧MIR資料の古い記述を現行言語規則へ統一した。

## 適合データの照合

既存の適合要件ID全70件を維持した。\
case宣言から入力とExpectedデータを抽出し、numeric751、型境界1,315、静的fixture181の計2,247件を[family別表](../../specs/ond/conformance/cases/README.md)へ記録した。\
抽出は入力データと期待宣言の読出しだけを行い、compilerの評価結果を期待値へ採用する処理は実行していない。\
一時的なcore宣言抽出testはofflineで3件成功し、その後に全て復元した。\
case ID・要件ID列・完全source・Expectedの表内容を元宣言と往復照合し、2,247件すべての一致とID一意性を確認した。\
既存のAST/MIR callbackとfirst-fit回帰は実装検証として区分する。\
共通manifestおよび欠落manifestの例外も入力条件に含める。

規範本文の範囲・型・短絡・初期化・layout・ABI・成果物・link検査の意味は基準資料と承認済み判断へ照合した。\
仕様ID検査、local link検査、全体品質検査の実行結果は作業完了時の報告に記す。

Kagura backendの独自440 IDとcore登録2,247 IDの交差0件も集合照合し、共有50観測caseを識別した。\
整数336件の宣言抽出testはofflineで1件成功し、抽出後に復元した。

旧EBNFの82 production全件を概念章へ保持し、production名の重複0件を確認した。\
既存generic named function callの型argumentを明示するTypeApplicationを1件追加し、任意Typeの明示適用と通常indexの静的な選択条件を記した。

## 検証完了

`pwsh -NoProfile -File tools/quality.ps1 -Profile Full`は2026-10-20に成功した。\
実行記録は`.ond-quality/runs/20261020-114624-quality/summary.json`であり、release-manifest-fixtures、spec-semantic-ids、cargo-fmt、cargo-test、cargo-clippyの全5工程がexit 0となった。\
clippyの既知警告81件を維持し、baselineの変更は行っていない。

最終文書は180個の一意なond-spec-idを持ち、基準110 IDと既存適合要件70 IDを維持する。\
local Markdownの参照先pathとheading anchorに不整合0件、旧EBNF 82 productionの欠落と重複0件を確認した。\
一時的な抽出testと生成script/dataはすべて除去し、無変更の11ファイルをHEADのraw bytesへ復元した。\
最終Rust差分は適合runnerが参照するrequirementsのpath更新とその整形である。
