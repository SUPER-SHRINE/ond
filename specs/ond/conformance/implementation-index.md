# 実装検証と歴史ID
<!-- ond-spec-id: conformance/implementation-index -->

MIR型、phase、lowering、helper、validatorと探索方式の管理元はRust/Ond sourceである。\
以下の既存IDは同じ主題の実装追跡を維持し、外部から観測される規則は各行の規範定義元を参照する。\
MIR-01はcompiler固有のIR妥当性検証に対応する。\
外部の言語適合は[要件とcase](./requirements.md)の入力・観測結果で照合する。

## backend入口とtarget境界
<!-- ond-spec-id: targets/kagura-v1/mir-backend -->

[実装source](../../../crates/compiler/src/mir_backend/mod.rs)で管理する。\
抽出した外部規則は[定義元](../targets/kagura-v1/README.md)に従う。

## 検証済み入力とsource診断
<!-- ond-spec-id: targets/kagura-v1/mir-backend-io-and-responsibilities -->

[実装source](../../../crates/compiler/src/mir_backend/mod.rs)で管理する。\
抽出した外部規則は[定義元](../language/v1/declarations.md#診断と成功結果)に従う。

## 型配置の実装
<!-- ond-spec-id: targets/kagura-v1/mir-data-layout -->

[実装source](../../../crates/compiler/src/mir_backend/layout.rs)で管理する。\
抽出した外部規則は[定義元](../targets/kagura-v1/machine.md#5-data-layout)に従う。

## 引数・戻り値とframeの実装
<!-- ond-spec-id: targets/kagura-v1/mir-abi-lowering -->

[実装source](../../../crates/compiler/src/mir_backend/function.rs)で管理する。\
抽出した外部規則は[定義元](../targets/kagura-v1/abi.md)に従う。

## 命令選択と演算実装
<!-- ond-spec-id: targets/kagura-v1/mir-instruction-lowering -->

[実装source](../../../crates/compiler/src/mir_backend/ops.rs)で管理する。\
抽出した外部規則は[定義元](../language/v1/expressions.md)に従う。

## global配置とstartupの実装
<!-- ond-spec-id: targets/kagura-v1/mir-intrinsic-global-startup -->

[実装source](../../../crates/compiler/src/program/startup.rs)で管理する。\
抽出した外部規則は[定義元](../language/v1/execution.md#初期化とエントリ)に従う。

## backend内部構成
<!-- ond-spec-id: targets/kagura-v1/mir-backend-architecture -->

[実装source](../../../crates/compiler/src/mir_backend/mod.rs)で管理する。\
抽出した外部規則は[定義元](../targets/kagura-v1/README.md)に従う。

## Object生成実装
<!-- ond-spec-id: targets/kagura-v1/mir-backend-object-contract -->

[実装source](../../../crates/compiler/src/mir_backend/globals.rs)で管理する。\
抽出した外部規則は[定義元](../artifacts.md#object)に従う。

## runtime helper供給と依存閉包
<!-- ond-spec-id: targets/kagura-v1/runtime-helper-supply -->

[実装source](../../../crates/compiler/src/mir_backend/runtime.rs)で管理する。\
抽出した外部規則は[定義元](../targets/kagura-v1/abi.md)に従う。

## numeric helperとallocator方式
<!-- ond-spec-id: targets/kagura-v1/numeric-and-allocator-lowering -->

[実装source](../../../crates/compiler/src/mir_backend/runtime/heap.ond)で管理する。\
抽出した外部規則は[定義元](../targets/kagura-v1/machine.md#31-heap-allocator)に従う。

## 関数内branch拡張
<!-- ond-spec-id: targets/kagura-v1/function-local-branches -->

[実装source](../../../crates/compiler/src/mir_backend/emit.rs)で管理する。\
抽出した外部規則は[定義元](../targets/kagura-v1/abi.md#1-registers)に従う。

## relocation実装回帰
<!-- ond-spec-id: targets/kagura-v1/relocation-verification -->

[実装source](../../../crates/compiler/src/linker/tests.rs)で管理する。\
抽出した外部規則は[定義元](../linking.md)に従う。

## compiler APIの接続
<!-- ond-spec-id: targets/kagura-v1/standard-path-responsibilities -->

[実装source](../../../crates/compiler/src/lib.rs)で管理する。\
抽出した外部規則は[定義元](../build/commands.md)に従う。

## 内部IRとcacheの管理元
<!-- ond-spec-id: conformance/internal-source-owners -->

| 主題 | Source管理元 | 抽出した外部規則 |
| --- | --- | --- |
| AST/HIR/MIRの型とphase | [IR](../../../crates/compiler-core/src/ir.rs)、[MIR](../../../crates/compiler-core/src/mir.rs)、[pipeline](../../../crates/compiler-core/src/pipeline/mod.rs) | [language](../language/v1/README.md) |
| MIR validator | [validator](../../../crates/compiler-core/src/mir/validate.rs) | [診断・成功結果](../language/v1/declarations.md#診断と成功結果) |
| process内cache | [cache](../../../crates/compiler-core/src/pipeline/hir/cache.rs) | [再利用と失効](../build/cache.md) |
| allocator first-fit/header/分割・結合・cursor | [heap.ond](../../../crates/compiler/src/mir_backend/runtime/heap.ond)、[heap回帰](../../../crates/compiler/src/mir_backend/heap_tests.rs) | [heap外部契約](../targets/kagura-v1/machine.md#31-heap-allocator) |
| Object/link patch実装 | [linker](../../../crates/compiler/src/linker/mod.rs)、[validator](../../../crates/compiler/src/linker/validate.rs)、[thunks](../../../crates/compiler/src/linker/thunks.rs) | [artifact](../artifacts.md)、[linking](../linking.md) |

旧`mir.md`と`ond-ir.md`のglobal runtime initializer/source init hook記述は、基準commitの言語規則およびNOTE-INIT-01と食い違っていた。\
移行後の初期化規則は[言語の定義元](../language/v1/execution.md#初期化とエントリ)へ統一し、globalはzeroまたはcompile-time constant、`init`は明示callする通常関数として記す。

## 既存回帰と要件
<!-- ond-spec-id: conformance/existing-regression-requirements -->

登録caseがない要件も、既存のsource fixtureと固定期待値へ以下の表で対応づける。\
test名リンクの位置から完全入力とassertionを参照できる。\
各行は記した検査範囲を主張し、任意の入力・実装方式すべての網羅を表すものではない。\
新しい期待値をcompiler実行結果から採用せず、既存の仕様由来期待と規範を照合した。

| 要件ID | 既存testと完全fixture | 検査入力・仕様由来期待と範囲 |
| --- | --- | --- |
| `SYN-01` | [unicode_identifiers_use_letter_and_decimal_digit_categories](../../../crates/compiler-core/src/ond/lexical_tests.rs#L22), [unicode_does_not_change_ascii_only_export_policy](../../../crates/compiler-core/src/ond/lexical_tests.rs#L41) | Letter/Nd識別子の受理、結合文字等のerror、ASCII大文字だけの公開判定 |
| `SYN-02` | [eof_comments_bom_and_source_offsets](../../../crates/compiler-core/src/ond/lexical_tests.rs#L255) | 先頭BOM/CRLF/EOF comment受理、直接NUL/途中BOM/未閉鎖commentのerror |
| `SYN-03` | [integer_spellings_and_radices](../../../crates/compiler-core/src/ond/lexical_tests.rs#L59), [invalid_numeric_spelling_is_a_lexical_error](../../../crates/compiler-core/src/ond/lexical_tests.rs#L150) | 2/8/10/16進とunderscore、0_600=384、不正数字のerror |
| `SYN-04` | [float_forms_and_exponent_boundaries](../../../crates/compiler-core/src/ond/lexical_tests.rs#L112), [hexadecimal_float_rounds_once_with_guard_and_sticky_bits](../../../crates/compiler-core/src/ond/lexical_tests.rs#L317) | decimal/hex float、0x1.000001p0のbits=0x3f800000、stickyと範囲error |
| `SYN-05` | [strings_are_validated_even_without_hir_lowering](../../../crates/compiler-core/src/ond/lexical_tests.rs#L180) | escape範囲、raw CR除去、quoted CR保持、LFのerror |
| `SYN-06` | [eof_comments_bom_and_source_offsets](../../../crates/compiler-core/src/ond/lexical_tests.rs#L255) | EOF/comment改行に伴うsemicolon挿入 |
| `SYN-08` | [delimiters_empty_statements_and_trailing_call_comma_are_supported](../../../crates/compiler-core/src/syntax_discard_tests.rs#L212) | grouped宣言、空文、末尾commaの受理、値5 |
| `SYN-09` | [call_statements_are_allowed_in_control_flow_clauses](../../../crates/compiler-core/src/syntax_discard_tests.rs#L235), [non_call_expression_statements_and_missing_separators_are_rejected](../../../crates/compiler-core/src/syntax_discard_tests.rs#L254) | call headerを受理し値5、他の式文と区切り欠落をerror |
| `SYN-10` | [non_call_expression_statements_and_missing_separators_are_rejected](../../../crates/compiler-core/src/syntax_discard_tests.rs#L254) | 記載した入力はgrammar成立条件違反としてerror。あらゆる不正構文の全件網羅を主張する行ではない |
| `PKG-04` | [constants_resolve_across_packages_and_lexical_scopes](../../../crates/compiler-core/src/semantic_tests.rs#L8), [reports_cycles_and_private_imports](../../../crates/compiler-core/src/semantic_tests.rs#L83) | 公開type/constant/global/functionの解決、private名と循環のerror |
| `DECL-03` | [discards_preserve_calls_and_multi_value_positions](../../../crates/compiler-core/src/syntax_discard_tests.rs#L19), [blank_parameters_keep_argument_slots_but_no_binding](../../../crates/compiler-core/src/syntax_discard_tests.rs#L51), [discard_never_suppresses_evaluation_faults](../../../crates/compiler-core/src/syntax_discard_tests.rs#L86) | blank破棄も評価し[3,12121234]、blank parameter位置を保ち42、破棄式のzero divisionはfault |
| `DECL-04` | [short_declarations_reuse_only_current_scope_variables](../../../crates/compiler-core/src/syntax_discard_tests.rs#L65), [invalid_blank_uses_and_short_declarations_are_rejected](../../../crates/compiler-core/src/syntax_discard_tests.rs#L114) | 現scope再利用で[5,6]、新規名欠落/重複/blank参照/個数違いはerror |
| `TYPE-01` | [anonymous_structs_match_across_files_and_function_signatures](../../../crates/compiler-core/src/type_identity_tests.rs#L53), [public_fields_match_across_packages_and_private_fields_do_not](../../../crates/compiler-core/src/type_identity_tests.rs#L82) | 構造型同一性、公開field名と非公開field所有packageの区別 |
| `TYPE-02` | [defined_structs_stay_distinct_but_matching_underlying_casts_work](../../../crates/compiler-core/src/type_identity_tests.rs#L67) | defined structの別identity、同underlyingのcast値7、暗黙assignmentはerror |
| `TYPE-03` | [nested_arrays_pointers_and_function_parameter_names_are_structural](../../../crates/compiler-core/src/type_identity_tests.rs#L62), [field_order_names_and_nominal_element_types_remain_significant](../../../crates/compiler-core/src/type_identity_tests.rs#L116) | array長・pointer先・parameter/result型列、field順/nameとnominal要素型を照合 |
| `TYPE-04` | [recursive_pointer_elements_are_checked_after_resolution](../../../crates/compiler-core/src/array_layout_tests.rs#L125) | pointer再帰受理、値として循環するCycle [1]Cycleをerror |
| `NIL-02` | [faults_do_not_execute_following_effects_or_overwrite_destination](../../../crates/compiler/src/mir_backend/conformance/evaluation.rs#L86) | NIL-02.call: nil function callはBusFault、後続write前に停止し5word SENTINEL保持 |
| `AGG-01` | [length_is_maximum_index_plus_one_not_last_index](../../../crates/compiler-core/src/inferred_array_tests.rs#L27), [runtime_elements_keep_source_order_and_holes_are_zero](../../../crates/compiler-core/src/inferred_array_tests.rs#L45), [invalid_inference_is_a_source_error](../../../crates/compiler-core/src/inferred_array_tests.rs#L54) | 最大index+1の推論長、穴zeroと評価順123、空/重複/範囲外はerror |
| `AGG-02` | [memory_errors_have_source_locations](../../../crates/compiler-core/src/memory_tests.rs#L180) | 重複field、位置struct literal、未知field、&literal等をsource付きerror |
| `AGG-03` | [nested_aggregates_copy_by_value_and_pointers_alias_storage](../../../crates/compiler-core/src/memory_tests.rs#L10), [aggregate_arguments_returns_and_parallel_assignment_preserve_snapshots](../../../crates/compiler-core/src/memory_tests.rs#L53) | nested copy [1,7,9,7,0,0]、argument/return/parallel snapshot [99,2,1,2] |
| `BOUND-01` | [arrays_including_string_literals_are_bounds_checked_but_raw_pointers_are_not](../../../crates/compiler-core/src/memory_tests.rs#L140) | 有効index1は0、index2/-1はbounds fault。raw guard形状のcallbackは内部検査として区分 |
| `PTR-01` | [nested_aggregates_copy_by_value_and_pointers_alias_storage](../../../crates/compiler-core/src/memory_tests.rs#L10) | local/field/indexのaddress取得とalias writeを値[1,7,9,7,0,0]で照合 |
| `STR-02` | [string_literals_are_mutable_byte_array_values_and_copy_independently](../../../crates/compiler-core/src/memory_tests.rs#L72), [string_literal_lengths_and_pointer_types_require_exact_matches](../../../crates/compiler-core/src/memory_tests.rs#L121) | 文字列byte配列copy/mutation [5,0xe9,42,0x9f,0]、長さ/要素型/pointer型不一致error |
| `LEN-01` | [array_len_preserves_one_operand_side_effect_and_runtime_faults](../../../crates/compiler-core/src/constant_value_tests.rs#L374) | len operandを1回評価、index0は[2,1]、index1はbounds fault |
| `MEM-01` | [failed_allocation_and_overflow_preserve_existing_memory](../../../crates/compiler/src/mir_backend/heap_tests.rs#L36), [allocation_zeroing_reuse_and_explicit_initialization_execute](../../../crates/compiler/src/mir_backend/heap_tests.rs#L5) | 失敗/overflowは既存42を保ってnilで43、alignment余り0、new zeroとzero-size成功。後者test内のaddress再利用比較は内部回帰 |
| `MEM-02` | [layout_and_abi_have_independent_fixed_expectations](../../../crates/compiler/src/mir_backend/memory_tests.rs#L365) | stack/aggregateのlayoutとABIに対する固定値照合。escape pointerの有効期間は利用側の前提 |
| `MEM-03` | [bounds_fault_precedes_destination_write](../../../crates/compiler/src/mir_backend/memory_tests.rs#L153) | bounds失敗時にdestinationを保持して停止。追加stack依存はABIのfault規則と組合せ回帰で照合 |
| `MEM-04` | [arrays_including_string_literals_are_bounds_checked_but_raw_pointers_are_not](../../../crates/compiler-core/src/memory_tests.rs#L140) | raw indexのguard形状は実装検査。有効pointerと生存期間が前提であり、不正free/破損pointerの成功動作を期待する検査はここでは定義しない |
| `FLOW-01` | [executes_cfg_scopes_loops_and_short_circuit](../../../crates/compiler-core/src/control_flow_tests.rs#L100) | 3形forと最内break/continueでloops=22、scope値1、branch入力0/1/2→7/8/9 |
| `CALL-01` | [multiple_results_are_evaluated_once_and_forwarded](../../../crates/compiler-core/src/call_tests.rs#L133), [statement_calls_discard_zero_one_or_many_results](../../../crates/compiler-core/src/call_tests.rs#L246) | pair/forwardの複数値展開は[11,12]、call回数を保持しstatementで0/1/複数resultを破棄 |
| `CALL-02` | [calls_support_imports_forward_references_and_mutual_recursion](../../../crates/compiler-core/src/call_tests.rs#L182), [indirect_calls_evaluate_callee_then_arguments_and_keep_short_circuit](../../../crates/compiler-core/src/call_tests.rs#L206) | import/forwardで8、even(4/3)→true/false、間接callは[7,9]とcallee→引数trace |
| `INIT-01` | [serialized_packages_link_identically_to_full_codegen_without_runtime_initializers](../../../crates/cli/src/tests.rs#L604) | cold/warm/fullのimageとinitializer metadata一致。zero/constant globalを使う現fixtureの範囲を検査し、任意initializer列全順序の網羅を主張しない |
| `INIT-02` | [globals_are_zeroed_or_initialized_from_compile_time_values](../../../crates/compiler-core/src/global_tests.rs#L14), [runtime_global_initializers_are_rejected](../../../crates/compiler-core/src/global_tests.rs#L43) | global zero/constantで[0,9,4,5,3]、runtime call/global read/new/multi-return initializerはerror |
| `OUT-01` | [global_objects_use_checked_natural_bss_layout_and_abs32_relocations](../../../crates/compiler/src/mir_backend/global_tests.rs#L70), [string_literals_and_aggregate_constants_have_stable_rodata_addresses](../../../crates/compiler/src/mir_backend/global_tests.rs#L103) | Bss自然alignment/Abs32とrodata addressをartifactで照合。実行環境固有の動作は実行環境側の契約 |
| `OUT-02` | [compile_project_does_not_predeclare_asset](../../../crates/compiler/src/pipeline/tests.rs#L216) | 未宣言asset functionはsource error。非source fileの同梱は配布ツール、device構成は実行環境側の契約 |
| `MIR-01` | [validator_rejects_use_before_definition](../../../crates/compiler-core/src/completion_tests.rs#L220), [validator_rejects_non_dominating_branch_value](../../../crates/compiler-core/src/completion_tests.rs#L235), [validator_rejects_wrong_operation_and_direct_target](../../../crates/compiler-core/src/completion_tests.rs#L267), [validator_requires_division_and_bounds_guards](../../../crates/compiler-core/src/completion_tests.rs#L297) | 破損IRのID/型/支配/guardを拒否する実装内部検査 |
