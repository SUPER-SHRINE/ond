# 適合case索引
<!-- ond-spec-id: conformance/cases-index -->

各family文書はcase ID、完全なsource入力、要件ID、仕様から導く受理・拒否・値期待を対応づける。

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
| numeric-runtime（内部追跡） | 1 | [numeric-runtime](./numeric-runtime.md) |
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
| layout-operators | 4 | [layout-operators](./layout-operators.md) |

## 入力と期待結果の読み方
<!-- ond-spec-id: conformance/case-contract -->

適用版はOnd Language v1とKagura v1のdata layoutである。\
各caseは他caseから独立したprojectとして扱い、sourceとmanifestは入力欄をそのままUTF-8で作成する。\
既定manifestは`ond.toml`に`[project]`と`name = "test"`をLFで記す。\
`PKG-01.missing_manifest`だけはmanifestを置かず、それ以外の例外はfamily本文に記す。\
追加library、依存project、cacheは明示された入力がある場合だけ用いる。

表の入力欄はHTML entityを元の文字へ復号し、`<br>`をLF、`&#13;`をCRとしてsource byte列を復元する。\
file見出しとfile間の区切りはsourceへ含めず、各code本文の末尾LFは保持する。\
`main.ond:<br>`等のfile見出しに続く部分が当該fileの内容であり、fileを分ける`<br><br>次のfile名:<br>`は区切りとして読む。

Phase列は観測操作を次のように指定する。

| Phase | 操作と成功期待 |
| --- | --- |
| Parse | projectを読み込み、token化と構文解析を行う。受理は構文の成立のみを表し、名前・型・entryの成立や実行成功を含まない。構文解釈期待は結合・型形式として照合する |
| Core | project全体の名前・型・定数・静的条件を検査する。成功時は記載された宣言の型と値を照合する |
| MemoryVm | Coreの検査後、freshなglobal zero/constant初期化状態で記載された関数を指定argument列により1回呼ぶ。返値列、fault、実行済みeffectを照合する。実装固有のIRやVM命令を要求しない |

`accept`は当該phaseの成功と診断0件を表す。\
`reject; diagnostics=N`はError/Warningのseverityと個数を含め、各診断が同じ行の位置とmessage比較を満たすことを1対1で照合する。\
診断順序は比較せず、`exact`は全文一致、`contains`は部分文字列一致とする。\
診断文と件数はcase固有の観測であり、一般の言語error規則へ追加する条件ではない。\
`file:[start,end)`はfile先頭からのUTF-8 byte範囲、`file:EOF`はEOFのzero幅位置、`project`はproject全体に対する診断位置を表す。

argument列と返値列の整数bitsはu64で表し、signed値は符号拡張したtwo's-complement表現、f32値は[Ond binary32](../../language/v1/expressions.md#floating-point-arithmetic)のu32 bitsとして比較する。\
argumentの整数はparameter宣言型の幅と符号で解釈し、`18446744073709551615`は`i32`parameterでは`-1`を表す。\
boolのargumentと返値は0/1、f32はu32 bit patternとし、bytesは要素ごとのbyte列、typed nilは記載型のnil値として照合する。\
比較はphaseと各caseの型・値・診断・state・停止結果を対象とする。

## 外部allocatorと実装回帰
<!-- ond-spec-id: conformance/case-implementation-boundary -->

外部allocatorの適合範囲は[言語のheap契約](../../language/v1/execution.md#メモリ管理)と[Kaguraの配置境界](../../targets/kagura-v1/machine.md#31-heap-allocator)を適用する。\
`first_fit_splitting_coalescing_and_tail_reclamation_execute`、`FP-02.numeric-dynamic-mir-contracts`、zero-storage callback、`MIR-01`、内部frame組合せは実装追跡対象であり、外部適合caseの通過数へ加えない。\
これらのIDと外部契約の対応は[追跡索引](../implementation-index.md)と[内部numeric追跡](./numeric-runtime.md)を参照する。

## 追加の回帰入力
<!-- ond-spec-id: conformance/cases/additional-observations -->

- [成果物・build・link](./artifact-contracts.md)
- [debugとfault](./debug-fault-contracts.md)
- [operator解決と代入](./operator-contracts.md)
- [字句](./lexical-observations.md)
- [言語回帰の外部観測](./regression-observations.md)
- [Kagura回帰の外部観測](./kagura-regression-observations.md)
- [cacheとlink](./build-observations.md)

## Kagura backendの実行入力
<!-- ond-spec-id: conformance/kagura-case-inventory -->

- [整数336件](./kagura-integers.md)
- [数値変換93件](./kagura-casts.md)
- [評価/fault8件、内部組合せ3件、共有50件](./kagura-evaluation-combinations.md)
