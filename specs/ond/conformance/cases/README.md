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
| numeric-integers | 405 | [numeric-integers](./numeric-integers.md) |
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
`PKG-01.missing_manifest`だけはmanifestを置かず、それ以外の例外は各familyに記す。\
依存project、cacheは入力欄に明示された場合だけ用いる。\
file見出しのない単一file入力は`main.ond`として扱う。\
file見出しがある場合は、見出しに続くsourceを各fileの内容として復元する。\
表の入力欄はHTML entityを元の文字へ復号し、`<br>`をLF、`&#13;`をCRとしてsource byte列を復元する。\
file見出しとfile間の区切りはsourceへ含めず、各code本文の末尾LFは保持する。\
`main.ond:<br>`等のfile見出しに続く部分が当該fileの内容であり、`<br><br>次のfile名:<br>`はfile区切りとして読む。

Phase列は観測操作を次のように指定する。

| Phase | 操作と成功期待 |
| --- | --- |
| Parse | projectを読み込み、token化と構文解析を行う。受理は構文の成立を表し、構文解釈期待は表に記した結合・型形式を照合する |
| Core | project全体の名前・型・定数・静的条件を検査する。成功時は記載された宣言の型と値を照合する |
| MemoryVm | Coreの検査後、freshなglobal zero/constant初期化状態で記載された関数を指定argument列により1回呼び、返値列、fault、実行済みeffectを照合する |

`accept`単独は当該phaseの成功を表し、表の値欄に型・値が記載されている場合はそれらも比較する。\
`reject`は当該phaseで入力の不成立を診断して失敗することを表し、[診断と成功結果](../../language/v1/declarations.md#診断と成功結果)の原因位置と独立したエラーの収集を比較する。\
原因位置にfileを記した行は、そのfileの不成立な名前・式・宣言等に対応するUTF-8 byte範囲を持つことを照合する。\
`project`はmanifest欠落・entry欠落・import graphなどproject入力全体の不成立に対応する。\
主位置と関連位置を区別するgeneric診断は[型parameterの診断規則](../../language/v1/types.md#型parameter)に従う。\
比較対象は成功・失敗、原因との対応、独立した原因の収集、および各章が明示した診断の集約と関連位置とする。

argument列と返値列の整数bitsはu64で表し、signed値は符号拡張したtwo's-complement表現、f32値は[Ond binary32](../../language/v1/expressions.md#floating-point-arithmetic)のu32 bitsとして比較する。\
argumentの整数はparameter宣言型の幅と符号で解釈し、`18446744073709551615`は`i32`parameterでは`-1`を表す。\
boolのargumentと返値は0/1、f32はu32 bit patternとし、bytesは要素ごとのbyte列、typed nilは記載型のnil値として照合する。\
比較はphaseと各caseの型・値・診断・state・停止結果を対象とする。

## 外部allocatorと実装回帰
<!-- ond-spec-id: conformance/case-implementation-boundary -->

allocatorの外部観測は[言語のheap契約](../../language/v1/execution.md#メモリ管理)と[Kaguraの配置境界](../../targets/kagura-v1/machine.md#31-heap-allocator)を適用する。\
実装検査IDのscopeと対応規範は[Sourceとの管理境界](../../../../AGENTS.md#sourceとの管理境界)に記す。

## 追加の回帰入力
<!-- ond-spec-id: conformance/cases/additional-observations -->

- [成果物・build・link](./artifact-contracts.md)
- [ONDX wire・配置・CLI・test生成](./executable-binaries.md)
- [公開interfaceと型付きwire値](./interface-values.md)
- [Package build_idの完全入力](./package-hash.md)
- [診断の原因とgeneric関連位置](./diagnostic-causes.md)
- [debugとfault](./debug-fault-contracts.md)
- [operator解決と代入](./operator-contracts.md)
- [字句](./lexical-observations.md)
- [言語回帰の外部観測](./regression-observations.md)
- [Kagura回帰の外部観測](./kagura-regression-observations.md)
- [Stackとstartup storageの外部観測](./memory-placement.md)
- [cacheとlink](./build-observations.md)
- [Roleの設定・選択・可視性・補完・cache](./roles.md)

## Kagura backendの実行入力
<!-- ond-spec-id: conformance/kagura-case-inventory -->

- [整数336件](./kagura-integers.md)
- [数値変換93件](./kagura-casts.md)
- [評価/fault8件、共有50件](./kagura-evaluation-combinations.md)
- [Inline assembly適合case](./inline-assembly.md)
