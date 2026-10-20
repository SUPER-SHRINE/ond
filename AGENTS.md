# Ondリポジトリの作業指針

## 検証

- リポジトリ全体に関わる変更を完了とする前に、`pwsh -NoProfile -File tools/quality.ps1 -Profile Full`を実行する。
- 実装中は対象を絞った最小のテストを優先し、最後に全体の品質検査を実行する。
- 通常の修正で`tools/clippy-baseline.json`を更新しない。baselineの変更にはユーザーの明示的な承認と、追加される警告すべてのレビューが必要。

## 変更の境界

- Ond言語仕様、ABI、MIRの意味論、Object／LinkedImage形式、公開APIの変更は、ユーザーの承認を必要とする設計判断として扱う。
- Ondのタスクから隣接するKaguraリポジトリを変更しない。Kagura側の変更が必要な場合は、課題を`blocked`として記録する。
- ユーザーの承認なく依存関係を追加・更新しない。
- ローカル品質ループではネットワークを利用しない。
- ユーザーによる無関係な変更を保持し、それらに対して破壊的なGit操作を実行しない。
- ソースファイルが概ね1,000行を超えた場合は、責務による分割が妥当か確認する。行数を満たすことだけを目的に分割しない。

## 言語

- リポジトリ内の説明文書、課題のタイトル・本文・判断履歴・完了記録、品質ループの報告は日本語を基本とする。
- コマンド、識別子、ファイルパス、外部ツールの原文エラー、機械処理用のキーと列挙値は、正確性と互換性のため英語のままでよい。必要に応じて日本語で意味を補足する。
- 品質ループが作るコミットメッセージは日本語で簡潔に記述する。

## ドキュメント管理と仕様レビュー

- 規範定義元は`specs/ond/`
- 適用文書は`specs/`、`docs/manual/`、`docs/decisions/`、`docs/specification-revision/`、crate内説明など、Ond規格を説明する文書
- 「非規範」等の表示は適用除外の理由にしない
- 除外文書は`docs/releases/`内のrelease notes、root `CHANGELOG.md`、root `README.md`
- 規則は単一箇所で定義し、他の場所から定義元を参照
- 規格として必要な操作手順・入出力例は契約として定義
- 利用案内、編集・移行作業の記録、設計判断理由、実装状態、将来予定、内部構造や学習用の解説を規範契約へ混在させない
- 規範に必要な契約は定義元へ統合し、新たな非規範保存先を設けて回避しない

### レビュー観点

| 観点 | 確認内容 |
| --- | --- |
| 独立実装可能性 | 入力、成立条件、型、評価順、状態変化、結果、失敗、観測点、適用版が定まり、Rust sourceなしで意味を確定できる。規格は外部から観測できる結果を定め、内部algorithmをsourceから仕様へ写すだけでは指摘解消としない |
| 適合判定 | 文書内の共通前提への参照を含め、caseの入力、設定、targetと版、初期状態、操作、期待結果または許容結果、比較方法、規範IDを特定できる。情報の重複再掲は要求せず、必須情報をRust fixtureやcallbackへ委ねない |
| 単一定義と整合 | 規則の定義元が一つであり、本文、文法、表、schema、要件、case間に矛盾や独自の追加条件がない |
| 肯定定義 | 構文、型、演算、動作、適合条件とその導出規則で定義する。対象外・未サポートの列挙や他言語との差による集合定義をしない。異常入力の条件、error/fault、結果、副作用、停止を必要に応じて定義する |
| 規範純度 | 現在・旧来・移行済みの状態、実績値、将来予定、実装都合、学習用の解説や判断理由を規範契約へ混在させない |
| 版、裁量、委譲 | 対象version/profile、外部規範の名称・版・参照箇所、実装裁量の範囲、維持義務、外部観測の保証を明記する。未記載事項を実装裁量として補わない |

適合テストの期待値は規範本文の規則から導く。\
テストは規格から導出した確認を行い、実測結果で規格の意味を決めない。\
case固有の入力・期待値と一般規則の再掲を区別し、各caseを定義元へ対応づける。\
内部AST/HIR/MIR構造への依存と、外部形式・ABIの規範を区別する。

### レビュー手順と完了条件

- 文書とcaseを先に読み、そこで決められない点を記録
- Rust sourceは差分・影響の確認に使用
- 指摘にはfile位置またはspec ID、問題箇所、基準違反、別実装・判定への影響、修正方向、意味判断の要否を記録
- 文章上の好みではなく、曖昧さ、冗長、整合、規範混在など具体的な基準違反を指摘
- 意味を保てる修正と設計判断が必要な項目を分け、後者はユーザーへ戻す
- 既存変更境界とspec/requirement/case IDの追跡を維持
- 重複削除・主題移動時はIDの扱いを確認し、同じIDで意味をすり替えない
- 完了条件は全対象章の確認、定義欠落・矛盾・二重定義・規範混在・実装依存の指摘解消、未決定の設計判断0件、リンク・ID・規範とcaseの整合確認
- case通過だけで規格全体への適合を主張しない

## 自動品質ループ

- Ondの長期品質ループの実行、再開、復旧、状態確認を求められた場合は、`.agents/skills/ond-quality-loop/SKILL.md`を使用する。
- 1回のループで扱う課題は最大1件とする。
- 自動修正は、再現可能な失敗または回帰テストがある、高確信・低リスクの課題に限定する。
- 自動でmergeやpushを行わない。成功した修正は、専用の`codex/quality-loop` worktreeブランチに、1課題1コミットで保存してユーザーのレビューを待つ。
- 運用状態は`.ond-quality/`に保存する
- 規格に必要な契約は`specs/ond/`の定義元へ統合し、別の説明・履歴文書へ複製しない

## 仕様文書の管理方針
<!-- ond-spec-id: decisions/specification-migration -->

規範契約の定義元は`specs/ond/`とし、編集と管理の規則は本書に置く。

### 編集原則
<!-- ond-spec-id: decisions/specification-editorial-principles -->

- 機能集合は明示した構文、型、演算、動作、適合条件とその導出で定める
- 規範節は適用範囲、入力形式、成立条件、意味と観測結果、失敗条件、委譲を必要に応じて記す
- 外部規範は名称・版・参照箇所を固定し、target固有の契約は対象profileを明記する

### 論点IDと定義元
<!-- ond-spec-id: decisions/specification-approved-questions -->

| 論点ID | 同じ主題の規範定義元 |
| --- | --- |
| SPEC-Q01 | [整数演算のoperand・result型](specs/ond/language/v1/expressions.md#integer-arithmetic) |
| SPEC-Q02 | [整数定数の正確値と範囲検査](specs/ond/language/v1/expressions.md#integer-constant-arithmetic) |
| SPEC-Q03 | [Unicode分類と字句](specs/ond/language/v1/lexical.md#source形式) |
| SPEC-Q04 | [heapの外部保証](specs/ond/language/v1/execution.md#メモリ管理)と[Kagura配置](specs/ond/targets/kagura-v1/machine.md#31-heap-allocator) |
| SPEC-Q05 | [言語](specs/ond/language/v1/README.md)、[machine](specs/ond/targets/kagura-v1/machine.md)、[ABI](specs/ond/targets/kagura-v1/abi.md)、[成果物](specs/ond/artifacts.md)、[linking](specs/ond/linking.md)の利用境界 |
| SPEC-Q06 | [Object・LinkedImage](specs/ond/artifacts.md)と[linking](specs/ond/linking.md) |
| SPEC-Q07 | [版と互換条件](specs/ond/artifacts.md#版識別と互換条件) |
| SPEC-Q08 | [cacheの再利用と失効](specs/ond/build/cache.md) |
| SPEC-Q09 | [適合要件](specs/ond/conformance/requirements.md)と[入力・期待値](specs/ond/conformance/cases/README.md) |
| NOTE-INIT-01 | [global初期化とentry](specs/ond/language/v1/execution.md#初期化とエントリ) |

### Sourceとの管理境界
<!-- ond-spec-id: decisions/specification-source-boundaries -->

- 外部言語、target、ABI、成果物、linking、CLI、cache、debug、test buildの契約は`specs/ond/`へ置く
- Rust型、内部AST/HIR/MIR、phase、helper、validator構造、命令選択・探索方式はsourceで管理する
- 実装検査IDは本書の追跡表でscopeと対応規範を記録し、別実装の外部適合条件を定義する根拠にしない

| 実装追跡ID・識別名 | scope | 対応する外部規範 |
| --- | --- | --- |
| `MIR-01`; `validator_rejects_use_before_definition`; `validator_rejects_non_dominating_branch_value`; `validator_rejects_wrong_operation_and_direct_target`; `validator_requires_division_and_bounds_guards` | MIR validatorのID、型、演算、支配、guard、global、initializer検証 | [Language v1](specs/ond/language/v1/README.md)、[診断](specs/ond/language/v1/declarations.md#診断と成功結果)、[cache](specs/ond/build/cache.md)、[成果物](specs/ond/artifacts.md)、[linking](specs/ond/linking.md) |
| `DECL-01.zero.0`–`DECL-01.zero.13`; zero-storage callback | zero storage内部形状とcallback | [宣言](specs/ond/language/v1/declarations.md#宣言) |
| `NUM-02.imported-constant.core` | imported constantのfolded MIR形状 | [整数定数演算](specs/ond/language/v1/expressions.md#integer-constant-arithmetic)、[package scope](specs/ond/language/v1/names-and-scopes.md#scopeとshadowing) |
| `FP-02.numeric-dynamic-mir-contracts` | dynamic numeric MIRのopcode、型、guard形状 | [整数演算](specs/ond/language/v1/expressions.md#integer-arithmetic)、[f32演算](specs/ond/language/v1/expressions.md#floating-point-arithmetic)、[型変換](specs/ond/language/v1/types.md#型同一性) |
| `ABI.far-large-aggregate-helper`; `FRAME.exact-fit`; `FRAME.next-call-fault` | 内部frame構築を伴うKagura評価組合せ | [Kagura ABI](specs/ond/targets/kagura-v1/abi.md)、[heapとstack容量](specs/ond/language/v1/execution.md#メモリ管理) |
| `first_fit_splitting_coalescing_and_tail_reclamation_execute`; `failed_allocation_and_overflow_preserve_existing_memory/2`; `allocation_zeroing_reuse_and_explicit_initialization_execute/2` | allocator方式、alignmentと解放address再利用の実装検査 | [heap契約](specs/ond/language/v1/execution.md#メモリ管理)、[Kagura heap配置](specs/ond/targets/kagura-v1/machine.md#31-heap-allocator) |
| `layout_and_abi_have_independent_fixed_expectations`; `global_objects_use_checked_natural_bss_layout_and_abs32_relocations` | 内部layout関数、section/offset、relocation選択、initializer構造 | [Kagura data layout](specs/ond/targets/kagura-v1/machine.md#5-data-layout)、[ABI](specs/ond/targets/kagura-v1/abi.md)、[Object](specs/ond/artifacts.md#object) |

## Semantic ID
<!-- ond-spec-id: language/semantic-id-policy -->

`specs/ond` 配下の各見出しには、探索処理から安定して参照できる `ond-spec-id` を HTML コメントとして直後に置く。\
見出し名やファイル配置を変更しても既存 ID は維持し、別の意味へ再利用しない。\
新しい見出しには一意な小文字 kebab-case のパスを割り当てる。\
`tools/test-spec-ids.ps1` と共通品質チェックが、欠落、形式違反、重複を検出する。
