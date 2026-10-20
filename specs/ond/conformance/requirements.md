# 適合要件と規範定義元の索引
<!-- ond-spec-id: conformance/requirements-index -->

規範定義元とcase対応を各表に記し、入力条件と仕様由来期待値は[case索引](./cases/README.md)を参照する。

各適合caseは、ここに示す要件IDと対応する仕様を参照する。\
要件の詳細な定義は、各節に示す仕様を正とする。

## 字句・構文
<!-- ond-spec-id: conformance/requirements-lexical-syntax -->

詳細: [Syntax v1](../language/v1/lexical.md)、[Language v1](../language/v1/README.md)

| ID | 要件の要約 | 規範定義元 | 対応するcase入力・期待 |
| --- | --- | --- | --- |
| SYN-01 | Unicode Letter/Nd識別子、ASCII限定公開判定、BOM/NUL | [SYN-01の定義元](../language/v1/lexical.md#source形式) | [unicode_identifiers_use_letter_and_decimal_digit_categoriesの入力・期待](./implementation-index.md#既存回帰と要件) |
| SYN-02 | UTF-8ファイル、LF/CRLF、空白、コメント | [SYN-02の定義元](../language/v1/lexical.md#source形式) | [eof_comments_bom_and_source_offsetsの入力・期待](./implementation-index.md#既存回帰と要件) |
| SYN-03 | 2/8/10/16進整数とunderscore制約 | [SYN-03の定義元](../language/v1/lexical.md#source形式) | [integer_spellings_and_radicesの入力・期待](./implementation-index.md#既存回帰と要件) |
| SYN-04 | decimal/hex float字句、直接binary32丸め | [SYN-04の定義元](../language/v1/lexical.md#source形式) | [float_forms_and_exponent_boundariesの入力・期待](./implementation-index.md#既存回帰と要件) |
| SYN-05 | quoted/raw string、escape範囲、CR・LF | [SYN-05の定義元](../language/v1/lexical.md#source形式) | [strings_are_validated_even_without_hir_loweringの入力・期待](./implementation-index.md#既存回帰と要件) |
| SYN-06 | semicolon挿入・EOF・閉じ括弧前省略 | [SYN-06の定義元](../language/v1/lexical.md#semicolonと改行) | [eof_comments_bom_and_source_offsetsの入力・期待](./implementation-index.md#既存回帰と要件) |
| SYN-07 | 演算子優先順位7段階、同順位左結合、postfix as | [SYN-07の定義元](../language/v1/expressions.md#演算子の優先順位) | [static-precedence](./cases/static-precedence.md) |
| SYN-08 | grouped const/var/type、空文、末尾comma | [SYN-08の定義元](../language/v1/declarations.md#構文) | [delimiters_empty_statements_and_trailing_call_comma_are_supportedの入力・期待](./implementation-index.md#既存回帰と要件) |
| SYN-09 | call以外の式文、if/for clauseの文種制限 | [SYN-09の定義元](../language/v1/statements.md#静的成立条件) | [call_statements_are_allowed_in_control_flow_clausesの入力・期待](./implementation-index.md#既存回帰と要件) |
| SYN-10 | 明示された構文と静的条件に基づく受理とerror | [SYN-10の定義元](../language/v1/README.md#適用範囲) | [non_call_expression_statements_and_missing_separators_are_rejectedの入力・期待](./implementation-index.md#既存回帰と要件) |
| SYN-11 | 関数型の名前省略・記載・混在、宣言は名前必須 | [SYN-11の定義元](../language/v1/declarations.md#関数) | [static-seed](./cases/static-seed.md) |
| SYN-12 | EBNFと本文の整合 | [SYN-12の定義元](../language/v1/lexical.md#文法の定義元) | [static-constant-contexts](./cases/static-constant-contexts.md)、[static-control-headers](./cases/static-control-headers.md)、[static-seed](./cases/static-seed.md) |

## package・名前解決・宣言
<!-- ond-spec-id: conformance/requirements-names-declarations -->

詳細: [Syntax v1](../language/v1/lexical.md)、[Language v1](../language/v1/README.md)、[パッケージbuild仕様](../build/packages.md)

| ID | 要件の要約 | 規範定義元 | 対応するcase入力・期待 |
| --- | --- | --- | --- |
| PKG-01 | 全projectでmanifestの有効なnameが必須、実行compileではmain.ondとfunc mainが必須、directoryとpackage名一致 | [PKG-01の定義元](../language/v1/names-and-scopes.md#packageとimport) | [static-scope](./cases/static-scope.md)、[static-seed](./cases/static-seed.md) |
| PKG-02 | 相対import、root import禁止、存在しないimport、循環 | [PKG-02の定義元](../language/v1/names-and-scopes.md#packageとimport) | [static-scope](./cases/static-scope.md) |
| PKG-03 | import名のfile scope・同path重複・fileごとのalias | [PKG-03の定義元](../language/v1/names-and-scopes.md#scopeとshadowing) | [static-scope](./cases/static-scope.md)、[static-seed](./cases/static-seed.md) |
| PKG-04 | 公開/非公開のtype・const・global・function | [PKG-04の定義元](../language/v1/names-and-scopes.md#packageとimport) | [constants_resolve_across_packages_and_lexical_scopesの入力・期待](./implementation-index.md#既存回帰と要件) |
| NAME-01 | package/local/parameterの重複宣言 | [NAME-01の定義元](../language/v1/names-and-scopes.md#scopeとshadowing) | [static-name-duplicates](./cases/static-name-duplicates.md)、[static-names](./cases/static-names.md) |
| NAME-02 | scope開始点、block/if/for scope、shadow | [NAME-02の定義元](../language/v1/names-and-scopes.md#scopeとshadowing) | [static-evaluation](./cases/static-evaluation.md)、[static-names](./cases/static-names.md) |
| NAME-03 | 組み込み型名・組み込み名のbinding禁止 | [NAME-03の定義元](../language/v1/names-and-scopes.md#scopeとshadowing) | [static-seed](./cases/static-seed.md) |
| DECL-01 | 初期値からの型推論、型付きzero、初期値なし型必須 | [DECL-01の定義元](../language/v1/declarations.md#宣言) | [boundaries-declarations](./cases/boundaries-declarations.md) |
| DECL-02 | const適格性、aggregate/typed nil、cycle | [DECL-02の定義元](../language/v1/declarations.md#宣言) | [boundaries-declarations](./cases/boundaries-declarations.md) |
| DECL-03 | blank identifierの破棄、binding/storageなし | [DECL-03の定義元](../language/v1/names-and-scopes.md#scopeとshadowing) | [discards_preserve_calls_and_multi_value_positionsの入力・期待](./implementation-index.md#既存回帰と要件) |
| DECL-04 | :=の新規名必須・現blockの変数だけ再利用 | [DECL-04の定義元](../language/v1/names-and-scopes.md#scopeとshadowing) | [short_declarations_reuse_only_current_scope_variablesの入力・期待](./implementation-index.md#既存回帰と要件) |

## 型・変換・nil
<!-- ond-spec-id: conformance/requirements-types-conversions -->

詳細: [Language v1](../language/v1/README.md)

| ID | 要件の要約 | 規範定義元 | 対応するcase入力・期待 |
| --- | --- | --- | --- |
| TYPE-01 | 構造型同一性、位置独立、非公開fieldのpackage区別 | [TYPE-01の定義元](../language/v1/types.md#型同一性) | [anonymous_structs_match_across_files_and_function_signaturesの入力・期待](./implementation-index.md#既存回帰と要件) |
| TYPE-02 | defined型の区別、同じunderlyingへの明示変換 | [TYPE-02の定義元](../language/v1/types.md#型同一性) | [defined_structs_stay_distinct_but_matching_underlying_casts_workの入力・期待](./implementation-index.md#既存回帰と要件) |
| TYPE-03 | 配列長・参照先・関数parameter/result列の一致 | [TYPE-03の定義元](../language/v1/types.md#型同一性) | [nested_arrays_pointers_and_function_parameter_names_are_structuralの入力・期待](./implementation-index.md#既存回帰と要件) |
| TYPE-04 | 直接再帰拒否、pointer経由再帰受理 | [TYPE-04の定義元](../language/v1/types.md#型) | [recursive_pointer_elements_are_checked_after_resolutionの入力・期待](./implementation-index.md#既存回帰と要件) |
| CAST-01 | 型不一致の暗黙変換拒否 | [CAST-01の定義元](../language/v1/types.md#型同一性) | [boundaries-conversions](./cases/boundaries-conversions.md) |
| CAST-02 | 定数整数変換・実行時wrap変換 | [CAST-02の定義元](../language/v1/types.md#型同一性) | [numeric-casts](./cases/numeric-casts.md) |
| CAST-03 | int↔f32、ties-even/trunc、NaN/Inf/範囲外 | [CAST-03の定義元](../language/v1/types.md#型同一性) | [numeric-casts](./cases/numeric-casts.md) |
| NIL-01 | pointer/function型nil、既定型なし、0とは別 | [NIL-01の定義元](../language/v1/types.md#nilとpointer) | [boundaries-conversions](./cases/boundaries-conversions.md)、[static-seed](./cases/static-seed.md) |
| NIL-02 | nil function pointer call | [NIL-02の定義元](../language/v1/declarations.md#関数) | [faults_do_not_execute_following_effects_or_overwrite_destinationの入力・期待](./implementation-index.md#既存回帰と要件) |

## 数値・演算・評価順序
<!-- ond-spec-id: conformance/requirements-numeric-evaluation -->

詳細: [Syntax v1](../language/v1/lexical.md)、[Language v1](../language/v1/README.md)

| ID | 要件の要約 | 規範定義元 | 対応するcase入力・期待 |
| --- | --- | --- | --- |
| NUM-01 | literal既定型/型文脈と整数6型の範囲 | [NUM-01の定義元](../language/v1/types.md#型) | [numeric-integers](./cases/numeric-integers.md) |
| NUM-02 | 定数算術の正確値、bitwiseのN-bit射影、値要求境界のrange check | [NUM-02の定義元](../language/v1/expressions.md#integer-constant-arithmetic) | [numeric-integers](./cases/numeric-integers.md)、[static-seed](./cases/static-seed.md) |
| NUM-03 | 定数shiftのu32 count、k mod N、左shift非wrap | [NUM-03の定義元](../language/v1/expressions.md#integer-constant-arithmetic) | [numeric-integers](./cases/numeric-integers.md) |
| NUM-04 | 定数/動的div・rem、符号、0、min/-1 | [NUM-04の定義元](../language/v1/expressions.md#integer-arithmetic) | [numeric-integers](./cases/numeric-integers.md) |
| NUM-05 | runtime wrap・bitwise・signed/unsigned比較 | [NUM-05の定義元](../language/v1/expressions.md#integer-arithmetic) | [numeric-integers](./cases/numeric-integers.md) |
| FP-01 | constant binary32各演算、signed zero/Inf/NaN | [FP-01の定義元](../language/v1/expressions.md#floating-point-arithmetic) | [numeric-float-vectors](./cases/numeric-float-vectors.md)、[numeric-floats](./cases/numeric-floats.md)、[static-seed](./cases/static-seed.md) |
| FP-02 | constant/runtimeのf32 bit一致 | [FP-02の定義元](../language/v1/expressions.md#floating-point-arithmetic) | [numeric-float-vectors](./cases/numeric-float-vectors.md) |
| OP-01 | operatorごとのoperand型・result型と型不一致error | [OP-01の定義元](../language/v1/expressions.md#式と評価順) | [boundaries-conversions](./cases/boundaries-conversions.md)、[boundaries-operators](./cases/boundaries-operators.md)、[layout-operators](./cases/layout-operators.md) 、[operator-contracts](./cases/operator-contracts.md) |
| EVAL-01 | &&/\|\|短絡、callee→引数の左から右 | [EVAL-01の定義元](../language/v1/expressions.md#式と評価順) | [static-constant-contexts](./cases/static-constant-contexts.md) |
| EVAL-02 | 一般operand、代入LHSのindex/pointer→RHS→書込順 | [EVAL-02の定義元](../language/v1/statements.md#静的成立条件) | [static-evaluation](./cases/static-evaluation.md) 、[operator-contracts](./cases/operator-contracts.md) |

## 配列・struct・文字列・メモリ
<!-- ond-spec-id: conformance/requirements-aggregate-memory -->

詳細: [Language v1](../language/v1/README.md)、[Built-ins](../language/v1/builtins.md)、[Kagura machine](../targets/kagura-v1/machine.md)

| ID | 要件の要約 | 規範定義元 | 対応するcase入力・期待 |
| --- | --- | --- | --- |
| AGG-01 | 固定長・定数長・推論長、key/穴/重複 | [AGG-01の定義元](../language/v1/types.md#配列とstruct) | [length_is_maximum_index_plus_one_not_last_indexの入力・期待](./implementation-index.md#既存回帰と要件) |
| AGG-02 | field名付きliteral、nested型明記、非addressable | [AGG-02の定義元](../language/v1/types.md#配列とstruct) | [memory_errors_have_source_locationsの入力・期待](./implementation-index.md#既存回帰と要件) |
| AGG-03 | aggregate値コピーとsource順・1回評価 | [AGG-03の定義元](../language/v1/types.md#配列とstruct) | [nested_aggregates_copy_by_value_and_pointers_alias_storageの入力・期待](./implementation-index.md#既存回帰と要件) |
| BOUND-01 | arrayの定数境界拒否、動的bounds check | [BOUND-01の定義元](../language/v1/types.md#配列とstruct) | [arrays_including_string_literals_are_bounds_checked_but_raw_pointers_are_notの入力・期待](./implementation-index.md#既存回帰と要件) |
| PTR-01 | &local/field/index、deref、raw index u32/unchecked | [PTR-01の定義元](../language/v1/types.md#nilとpointer) | [nested_aggregates_copy_by_value_and_pointers_alias_storageの入力・期待](./implementation-index.md#既存回帰と要件) |
| STR-01 | UTF-8・emoji・任意byte escapeとbyte len、raw string | [STR-01の定義元](../language/v1/expressions.md#文字列リテラル) | [static-seed](./cases/static-seed.md) |
| STR-02 | `[N]u8`型、完全長一致、copy/mutation/addressability、pointer非変換 | [STR-02の定義元](../language/v1/expressions.md#文字列リテラル) | [string_literals_are_mutable_byte_array_values_and_copy_independentlyの入力・期待](./implementation-index.md#既存回帰と要件) |
| LEN-01 | builtin配列lenは定数u32を返しoperandを1回評価 | [LEN-01の定義元](../language/v1/builtins.md#41-配列に対する-len) | [array_len_preserves_one_operand_side_effect_and_runtime_faultsの入力・期待](./implementation-index.md#既存回帰と要件) |
| BI-01 | alloc/free/newの型・arity・戻り値 | [BI-01の定義元](../language/v1/builtins.md#1-一覧) | [boundaries-builtins](./cases/boundaries-builtins.md) |
| BI-02 | lenの型・arity、builtin名のbinding禁止、`str`は通常identifier | [BI-02の定義元](../language/v1/builtins.md#4-len) | [boundaries-builtins](./cases/boundaries-builtins.md)、[layout-operators](./cases/layout-operators.md) 、[operator-contracts](./cases/operator-contracts.md) |
| BI-03 | thisFile/thisLineの型・arity・論理位置 | [BI-03の定義元](../language/v1/builtins.md#6-thisfile) | [boundaries-builtins](./cases/boundaries-builtins.md) |
| BI-04 | sizeof/alignofの型operandとlayout値 | [BI-04の定義元](../language/v1/builtins.md#組み込み構文) | [layout-operators](./cases/layout-operators.md) |
| MEM-01 | new zero/alignment、alloc failure/0、free nil等 | [MEM-01の定義元](../language/v1/execution.md#メモリ管理) | [failed_allocation_and_overflow_preserve_existing_memoryの入力・期待](./implementation-index.md#既存回帰と要件) |
| MEM-02 | stack/startup配置、escapeしても暗黙heap化なし | [MEM-02の定義元](../language/v1/execution.md#メモリ管理) | [memory-placement](./cases/memory-placement.md) |
| MEM-03 | bounds check失敗のnon-return/fault契約 | [MEM-03の定義元](../language/v1/execution.md#メモリ管理) | [bounds_fault_precedes_destination_writeの入力・期待](./implementation-index.md#既存回帰と要件) |
| MEM-04 | raw pointerの有効領域・寿命・free入力を利用側が管理 | [MEM-04の定義元](../language/v1/execution.md#メモリ管理) | [arrays_including_string_literals_are_bounds_checked_but_raw_pointers_are_notの入力・期待](./implementation-index.md#既存回帰と要件) |

## 制御フロー・関数・起動
<!-- ond-spec-id: conformance/requirements-flow-entry -->

詳細: [Syntax v1](../language/v1/lexical.md)、[Language v1](../language/v1/README.md)、[Kagura ABI](../targets/kagura-v1/abi.md)

| ID | 要件の要約 | 規範定義元 | 対応するcase入力・期待 |
| --- | --- | --- | --- |
| FLOW-01 | if/else-if、for3形式、最内break/continue | [FLOW-01の定義元](../language/v1/statements.md#文と制御構文) | [executes_cfg_scopes_loops_and_short_circuitの入力・期待](./implementation-index.md#既存回帰と要件) |
| FLOW-02 | bool条件、必須block、loop外break/continue拒否 | [FLOW-02の定義元](../language/v1/statements.md#静的成立条件) | [static-control-headers](./cases/static-control-headers.md)、[static-evaluation](./cases/static-evaluation.md)、[static-flow](./cases/static-flow.md) |
| FLOW-03 | 全経路return・終端loopの判定 | [FLOW-03の定義元](../language/v1/statements.md#静的成立条件) | [static-evaluation](./cases/static-evaluation.md)、[static-flow](./cases/static-flow.md) |
| CALL-01 | 0/1/複数戻り値、展開位置・個数・型 | [CALL-01の定義元](../language/v1/declarations.md#関数) | [multiple_results_are_evaluated_once_and_forwardedの入力・期待](./implementation-index.md#既存回帰と要件) |
| CALL-02 | direct/indirect、forward/import/mutual recursion | [CALL-02の定義元](../language/v1/declarations.md#関数) | [calls_support_imports_forward_references_and_mutual_recursionの入力・期待](./implementation-index.md#既存回帰と要件) |
| INIT-01 | file辞書順/import DFS/宣言順/各package1回 | [INIT-01の定義元](../language/v1/execution.md#初期化とエントリ) | [serialized_packages_link_identically_to_full_codegen_without_runtime_initializersの入力・期待](./implementation-index.md#既存回帰と要件) |
| INIT-02 | 全global事前zero、内部init、推論/参照順独立 | [INIT-02の定義元](../language/v1/execution.md#初期化とエントリ) | [globals_are_zeroed_or_initialized_from_compile_time_valuesの入力・期待](./implementation-index.md#既存回帰と要件) |
| INIT-03 | mainちょうど1つ/引数戻り値なし、initは通常の関数名として許可 | [INIT-03の定義元](../language/v1/execution.md#初期化とエントリ) | [static-scope](./cases/static-scope.md) |
| OUT-01 | caller指定のmain終了経路、ABI/rodata/layout/static link | [OUT-01の定義元](../language/v1/execution.md#build-output) | [global_objects_use_checked_natural_bss_layout_and_abs32_relocationsの入力・期待](./implementation-index.md#既存回帰と要件) |
| OUT-02 | build toolのasset管理とsource入力の境界 | [OUT-02の定義元](../language/v1/execution.md#build-output) | [compile_project_does_not_predeclare_assetの入力・期待](./implementation-index.md#既存回帰と要件) |

## 診断と実装検証の追跡
<!-- ond-spec-id: conformance/requirements-diagnostics-ir -->

詳細: [Language v1](../language/v1/README.md)、[Kagura target契約](../targets/kagura-v1/README.md)、[実装追跡](./implementation-index.md)

| ID | 要件の要約 | 規範定義元 | 対応するcase入力・期待 |
| --- | --- | --- | --- |
| DIAG-01 | source errorを位置付き診断で返し、全検査成功時に成功結果を得る | [DIAG-01の定義元](../language/v1/declarations.md#診断と成功結果) | [static-diagnostics](./cases/static-diagnostics.md)、[diagnostic-causes](./cases/diagnostic-causes.md) |
| DIAG-02 | Unicode/複数fileの診断位置 | [DIAG-02の定義元](../language/v1/declarations.md#診断と成功結果) | [static-diagnostics](./cases/static-diagnostics.md)、[static-seed](./cases/static-seed.md)、[diagnostic-causes](./cases/diagnostic-causes.md) |

## 成果物・build・target
<!-- ond-spec-id: conformance/requirements-artifact-build-target -->

| ID | 要件の要約 | 規範定義元 | 対応するcase入力・期待 |
| --- | --- | --- | --- |
| ART-01 | JSON wire schemaとmanifest参照の整合 | [成果物](../artifacts.md) | [artifact-contracts](./cases/artifact-contracts.md)、[interface-values](./cases/interface-values.md) |
| DBG-01 | link前後debug schema、区間、unwindと不適合時の扱い | [debug](../debug-metadata/v1.md) | [debug-fault-contracts](./cases/debug-fault-contracts.md) |
| HASH-01 | canonical JSONと公開SHA-256識別子 | [canonical/hash](../build/cache.md#canonical-json) | [artifact-contracts](./cases/artifact-contracts.md)、[package-hash](./cases/package-hash.md)、[interface-values](./cases/interface-values.md) |
| DEP-01 | Git edgeとlock v2の固定・照合 | [lock](../build/packages.md#git依存lock-v2) | [artifact-contracts](./cases/artifact-contracts.md) |
| LINK-01 | 配置、symbol集合、Call16/Jump16の形とpatch | [linking](../linking.md) | [artifact-contracts](./cases/artifact-contracts.md) |
| FAULT-01 | KaguraのCPU faultとOndの意味分類 | [fault対応](../targets/kagura-v1/machine.md#faultと観測結果) | [debug-fault-contracts](./cases/debug-fault-contracts.md) |
