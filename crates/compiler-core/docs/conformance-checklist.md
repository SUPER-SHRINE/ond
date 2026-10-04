# compiler-core 仕様適合テスト・チェックリスト

2026-09-20 backend接続追補: 本文はcore監査時点の記録。336整数vectors・25float bits・evaluation 25ケースのKagura実行を追加し、共有fixtureへ抽出した。coreの312 test関数を維持している。backend待ち項目の最新状況は[backend適合表](../../../specs/ond/targets/kagura-v1/conformance-checklist.md)を参照し、過去の不足記述を現在の未実装判定に使わない。2026-10-03にfile scope importと診断回復・generic具体化診断の記述を現行実装へ同期した。

## 1. 対象と判定方法

最終更新: 2026-10-03。

対象仕様:

- [Language v1](../../../specs/ond/v1/language.md)（以下 L）
- [Syntax v1](../../../specs/ond/v1/syntax.md)（以下 S）
- [Built-in Functions v1](../../../specs/ond/v1/builtins.md)（以下 B）

対象工程はsource → AST → 型付きHIR → MIR → validator。
この文書は既存テストの検証内容を仕様に対応付けた棚卸しであり、仕様適合完了の宣言ではない。
不足は「実装がない」ではなく「その条件を確認するテストが不足」を意味する。
初回監査では実装・テストコードは変更していない。以後の仕様決定は§3、検証追加は§4・各追補文書に記録する。
今回のP2は証拠と残件の再整理のみで、実装・テストは変更していない。

判定:

- **検証済**: 当該行の狭い確認対象について、明示的なassertionがある。組み合わせ全体の網羅を意味しない。
- **部分**: 確認対象の一部しかassertしていない。右端に未検証条件を明示する。MIR形状は命令生成の証拠だが、実行結果の証拠にはしない。
- **不足**: 今回調査したcoreテストに直接の検証を確認できない。実装の成否は未判定。
- **別工程**: target/backend/runtimeの実行結果で確認すべき契約。coreで確認できる型・MIR部分は別行に記載。

証拠の強さ:

- A: parse/compile成功とvalidator通過。値・実行結果の正しさまでは保証しない。
- D: compile失敗、診断文・位置のassertion（単なるis_errとの違いを明記）。
- H: AST/HIR/MIRの具体的な型・値・命令・順序のassertion。
- E: テスト用MIR interpreterによる結果・traceのassertion。
- R: 既存AST backend経由のKagura実行。MIR backendの証拠には数えない。

### 現在の実行ベースライン

文字列仕様変更後に`cargo test --workspace`を再実行し、**全419テスト成功、ignored 0**。
compiler-coreは311件、language-serverは48件。これはRust test関数数であり、仕様の達成率ではない。
数値751ケース、型境界1,366ケース、制御/名前111ケース、診断6ケースはそれぞれテーブル内の件数で、Rust test関数数に加算しない。
数値の独立oracle 336 vectorsは生成MIRの実行検証件数に含めない。

### 初回〜P0の実行履歴

cargo test --workspace --quiet を初回監査で実行し、全210テスト成功。
うちcompiler-coreは103件、language-serverは48件。
件数はRustのtest関数数であり、表駆動テスト内の入力数や仕様項目数とは一致しない。

P0 scope/grammar追補後: `cargo test --workspace --quiet` 全278テスト成功。
compiler-coreは171件。共通形式の新規ケース50件に加え、EOF/loader照合とstartup validatorの回帰テストを追加。

Q-03短絡castの決定反映後: 同コマンドで全289テスト成功、compiler-coreは182件。
共通形式9ケースと通常の値文脈の回帰テスト2件を追加し、scope/grammarのP0を完了。

初回監査時には旧runtime適合ケースと旧AST backendの実行テストも存在したが、
現在のworkspaceからは撤去済みであり、現行実装の適合証拠には含めない。

## 2. チェックリスト

表中の参照は末尾のテスト索引を参照。右端には残件または検証範囲の限界を記す。
検証済の行にも追加候補はあり、全言語仕様への完全適合を意味しない。複数工程にまたがる行ではcore証拠とbackend残件を併記する。

### 字句・構文（S §§1–3,5–7、L §§1,8）

| ID | 確認対象 | 状態 / 既存証拠 | 不足・追加対象 |
| --- | --- | --- | --- |
| SYN-01 | Unicode Letter/Nd識別子、ASCII限定公開判定、BOM/NUL | 検証済 A/D/H: [LEX] unicode_identifiers_use_letter_and_decimal_digit_categories、unicode_does_not_change_ascii_only_export_policy、eof_comments_bom_and_source_offsets | Unicode全カテゴリの網羅ではない |
| SYN-02 | UTF-8ファイル、LF/CRLF、空白、コメント | 部分 A/D/H: [PARSE] コメント/byte span、[LEX] BOM/EOF、[診断表](./diagnostic-conformance.md) CRLF/Unicodeのlexer・parser・HIR位置 | 不正UTF-8のloader診断、同一sourceのLF/CRLF対照、block comment非入れ子の明示ケース。CRLFの位置検証は追加済 |
| SYN-03 | 2/8/10/16進整数とunderscore制約 | 検証済 A/D/H: [LEX] integer_spellings_and_radices、invalid_numeric_spelling_is_a_lexical_error | 型範囲の検証はNUM-01へ分離 |
| SYN-04 | decimal/hex float字句、直接binary32丸め | 検証済 A/D/H: [LEX] float_forms_and_exponent_boundaries、hexadecimal_float_rounds_once_with_guard_and_sticky_bits、hexadecimal_conversion_matches_exact_power_of_two_reference | 演算結果の検証はFP-01以降 |
| SYN-05 | quoted/raw string、escape範囲、CR・LF | 検証済 A/D/H: [LEX] strings_are_validated_even_without_hir_lowering、[MEM] decodes_raw_unicode_octal_and_invalid_string_escapes | 全escapeを列挙する追補は可能 |
| SYN-06 | semicolon挿入・EOF・閉じ括弧前省略 | 部分 A/D/E: [PARSE] inserts_semicolons_on_newlines_in_block、treats_multiline_block_comment_as_newline_for_semicolon_insertion、[LEX] EOF、[DISC] delimiters_empty_statements_and_trailing_call_comma_are_supported | 挿入対象token全種類×改行/EOF、else直前改行、複数行引数・literal |
| SYN-07 | 演算子優先順位7段階、同順位左結合、postfix as | 検証済 H: [precedence](../src/conformance/precedence.rs) SYN-07.precedence-associativity.ast / precedence-values.core | ASTの結合形と型の整合する式のHIR定数値。全operator×全数値型の行列はNUM/CASTで扱う |
| SYN-08 | grouped const/var/type、空文、末尾comma | 部分 A/E: [DISC] delimiters_empty_statements_and_trailing_call_comma_are_supported、[FT] ast_preserves_optional_names_and_type_spans | 各listの空/1件/複数件と不正separatorの対照 |
| SYN-09 | call以外の式文、if/for clauseの文種制限 | 検証済 A/D/E: [DISC] call_statements_are_allowed_in_control_flow_clauses、non_call_expression_statements_and_missing_separators_are_rejected | 他の未定義構文全般は次行 |
| SYN-10 | 未記載構文を受理しない | 部分 D: [LEX] imaginary literal、[DISC] 一部不正構文 | grouped/dot import、rune、slice、tuple、closure、local func/type、変種operator等の負例一覧 |
| SYN-11 | 関数型の名前省略・記載・混在、宣言は名前必須 | 検証済 A/D/H: [FT] 全4件 | 型名解決・callbackの実行値の組み合わせはCALL-02 |
| SYN-12 | EBNFと本文の整合 | 部分 A/D/H/E: §3のQ-01〜04、[constant contexts](../src/conformance/constant_contexts.rs) SYN-12.q03-*、[数値表](./numeric-conformance.md) | 既知のQ-01〜04は反映済。文法全productionの受理/拒否対照はSYN-06/08/10等に残る。数値行列の成功を文法全体の整合証明にしない |

### パッケージ・名前解決・宣言（L §§1–3,11、S §§3–4）

| ID | 確認対象 | 状態 / 既存証拠 | 不足・追加対象 |
| --- | --- | --- | --- |
| PKG-01 | manifest/main.ond必須、directoryとpackage名一致 | 検証済 A/D: [scope](../src/conformance/scope.rs) PKG-01.* とseed missing-main-file.reject | 空project/空sourceは拒否。package句だけで宣言のないpackageは受理（仕様は宣言数を要求しない） |
| PKG-02 | 相対import、root import禁止、存在しないimport、循環 | 検証済 A/D: [scope](../src/conformance/scope.rs) PKG-02.*、[LEX] import_paths_use_the_same_string_decoding | 入れ子path、root/未知/不正path、自己/三段循環の診断文・file/spanを照合 |
| PKG-03 | import名のfile scope・同path重複・fileごとのalias | 検証済 A/D/H: [scope](../src/conformance/scope.rs) PKG-03.*、[GLOBAL] 初期化順 | 各fileが必要なimportを宣言し、辞書順が前/後のfileから型/const/global/functionを参照する。別fileの同aliasが異なるpathを指せることも確認 |
| PKG-04 | 公開/非公開のtype・const・global・function | 部分 A/D/E: [SEM]/[GLOBAL]/[LEX] 公開判定、[P1制御表](./control-flow-conformance.md) import名のlocal shadowと復元 | 残りは全symbol種×公開/非公開×未importの対照。qualifier shadowの代表例は検証済 |
| NAME-01 | package/local/parameterの重複宣言 | 検証済 D/E: [P1制御表](./control-flow-conformance.md) package4×4、local var/const相互、parameterと直下local・同名parameterの拒否 | 別fileの重複を診断位置まで照合。内側blockでのshadowは実行結果で検証 |
| NAME-02 | scope開始点、block/if/for scope、shadow | 検証済 D/E: [P1制御表](./control-flow-conformance.md) 同時宣言・scope外参照・import shadow | var/const/短い宣言の初期値は外側bindingを参照。if条件の括弧なし空blockの不具合も修正し、回避用括弧を外した負例で照合 |
| NAME-03 | 組み込み型名・組み込み名のbinding禁止 | 検証済 D: [SPEC] 8型×宣言位置、[CALL]/[DONE] `len`のlocal bindingと組み込み名のtop-level関数の拒否 | struct field名・関数型内の説明用parameter名はbindingを作らないため対象外 |
| DECL-01 | 初期値からの型推論、型付きzero、初期値なし型必須 | 検証済 D/H: [型境界表](./type-boundary-conformance.md) 14型のzero/推論/後続異型代入、local/global型必須 | zeroのMIR値・格納先を照合。実機メモリ表現はbackend工程 |
| DECL-02 | const適格性、aggregate/typed nil、cycle | 検証済 D/H/E: [型境界表](./type-boundary-conformance.md) typed nil/defined aggregate/非定数拒否、[CONST] 入れ子、[SEM] cycle | 通常call/非nil関数/global/load/allocation/address/非定数aggregateを拒否。短絡/lenはQ-03を併用 |
| DECL-03 | blank identifierの破棄、binding/storageなし | 検証済 A/D/H/E: [DISC] discards_preserve_calls_and_multi_value_positions、discarded_globals_execute_in_order_without_storage、blank_parameters_keep_argument_slots_but_no_binding、invalid_blank_uses_and_short_declarations_are_rejected | 破棄でもfaultするケースを含む |
| DECL-04 | :=の新規名必須・現blockの変数だけ再利用 | 検証済 A/D/E: [DISC] short_declarations_reuse_only_current_scope_variables、invalid_blank_uses_and_short_declarations_are_rejected | 名前解決一般の網羅とは別 |

### 型・変換・nil（L §§3–4,9）

| ID | 確認対象 | 状態 / 既存証拠 | 不足・追加対象 |
| --- | --- | --- | --- |
| TYPE-01 | 構造型同一性、位置独立、非公開fieldのpackage区別 | 検証済 A/D/H/E: [IDENT] field_equality_and_hash_ignore_origin_but_not_owner、anonymous_structs_match_across_files_and_function_signatures、public_fields_match_across_packages_and_private_fields_do_not | 網羅的な生成型テストは後続候補 |
| TYPE-02 | defined型の区別、同じunderlyingへの明示変換 | 検証済 A/D/E: [IDENT] defined_structs_stay_distinct_but_matching_underlying_casts_work、field_order_names_and_nominal_element_types_remain_significant | scalar/aggregate別の境界はCASTへ |
| TYPE-03 | 配列長・参照先・関数parameter/result列の一致 | 部分 A/D: [IDENT] nested_arrays_pointers_and_function_parameter_names_are_structural、field_order_names_and_nominal_element_types_remain_significant、[FT] named_unnamed_and_mixed_signatures_have_identical_types | 戻り値型/個数/順序違い、再帰型を含む同一性の対照 |
| TYPE-04 | 直接再帰拒否、pointer経由再帰受理 | 部分 A/H/D: [BASE] represents_pointer_recursive_structs_with_type_ids、[DONE] validator_rejects_recursive_value_layout_but_allows_pointer_recursion | source側の相互再帰、array経由再帰、関数型経由、defined型chain |
| CAST-01 | 型不一致の暗黙変換拒否 | 検証済 D/H: [型境界表](./type-boundary-conformance.md) 14×14型対×代入/引数/return＋明示cast | scalar/2 pointer型/function/array/struct/defined整数の有限行列。literalの型文脈とは区別 |
| CAST-02 | 定数整数変換・実行時wrap変換 | 部分（runtime残）D/H: [数値表](./numeric-conformance.md) 定数36方向171ケース、動的36方向のMIR、runtime oracle用変換vectors | coreの列挙済み定数結果・cast型/属性は照合済。backend経由のwrap結果は未検証 |
| CAST-03 | int↔f32、ties-even/trunc、NaN/Inf/範囲外 | 部分（runtime残）D/H: [数値表](./numeric-conformance.md) 93定数ケース、動的12方向のcast型/Checked属性 | coreの列挙済み境界・fraction・NaN/±Inf・2^24前後を照合。runtime結果/faultはbackend工程 |
| NIL-01 | pointer/function型nil、既定型なし、0とは別 | 検証済 D/H: [型境界表](./type-boundary-conformance.md) nil87件、typed nil定数 | 代入/引数/return、型なしnil、0との区別、比較の両順序を照合 |
| NIL-02 | nil function pointer call | 別工程（CPU fault）R: 旧ケース59。core側は部分 A/H: [CALL] 間接call、[型境界表](./type-boundary-conformance.md) function nil定数/zero | core残: nil calleeを呼ぶIndirect callの保持・signatureを明示assert。nil値の検証はcall/faultの検証ではない |

### 数値・演算・評価順序（L §6、S §5）

| ID | 確認対象 | 状態 / 既存証拠 | 不足・追加対象 |
| --- | --- | --- | --- |
| NUM-01 | literal既定型/型文脈と整数6型の範囲 | 検証済 D/H: [数値表](./numeric-conformance.md) 74ケース、[VALUE] 値文脈の代表例 | min/maxと直外側、符号、radix、既定i32を照合。literal/定数式×6型×local/global/引数/returnの全対照は残る。P1の型付きparameter行列（CAST-01）で代用しない |
| NUM-02 | arbitrary precision定数、最後にrange check | 検証済 D/H/E: [CONST] exact_integer_intermediates_are_not_limited_to_i128、[VALUE] exact_values_flow_through_globals_assignments_arguments_and_returns、rejects_overflow_at_all_value_boundaries | 同一式の各境界への適用は代表例 |
| NUM-03 | 定数shiftのu32 count、k mod N、左shift非wrap | 検証済 D/H: [数値表](./numeric-conformance.md) 108定数ケースと動的MIR型/opcode | 6型×0/N−1/N/N+1/u32::MAX、負数right shift、count型/範囲拒否。runtime値はbackend工程 |
| NUM-04 | 定数/動的div・rem、符号、0、min/-1 | 部分（runtime残）D/H: [数値表](./numeric-conformance.md) 72定数ケース、全6型の動的guard/operand/opcode | coreの列挙済み定数4象限・min/max・0・min/-1を照合。runtime fault/結果はvectorsを用いたbackend検証待ち |
| NUM-05 | runtime wrap・bitwise・signed/unsigned比較 | 部分（runtime残）D/H: [数値表](./numeric-conformance.md) 138定数ケース、全operator×6型の動的MIR契約、独立oracleの期待値表 | wrap結果をMIR命令の存在だけで検証済とはしない。runtime実行は別工程 |
| FP-01 | constant binary32各演算、signed zero/Inf/NaN | 検証済 H: [数値表](./numeric-conformance.md) 69ケース＋入力bits付き25ケース | 全算術/単項operator、各種丸め境界、subnormal/underflow/overflow、NaN比較6種、NaN符号正規化。全bit pattern網羅ではない |
| FP-02 | constant/runtimeのf32 bit一致 | 部分（runtime残）H: [数値表](./numeric-conformance.md) 入力/期待bits25件の定数結果、動的全operatorのMIR契約 | 共有入力bitsを用いたruntimeとのbit一致はMIR backend移行後。MIR形状/定数結果を実機一致とみなさない |
| OP-01 | 許可operator×operand型、結果型、禁止型 | 検証済 D/H: [型境界表](./type-boundary-conformance.md) 14型×binary19/unary4、[数値表](./numeric-conformance.md) opcode/guard | pointer算術等の拒否と結果型を照合。アドレス/参照外しは[MEM]を併用 |
| EVAL-01 | &&/||短絡、callee→引数の左から右 | 検証済 E: [CFG] shortAnd/shortOr/nested、[CALL] indirect_calls_evaluate_callee_then_arguments_and_keep_short_circuit | compile-time適格性・未評価式の診断規則はQ-03 |
| EVAL-02 | 一般operand、代入LHSのindex/pointer→RHS→書込順 | 検証済 E: [P1制御表](./control-flow-conformance.md) 副作用trace＋書込結果 | 異なるLHS/alias、index・pointerのsnapshot、RHSの変更、aggregate全体/要素、複数return値を対照。backend実行は別工程 |

### 配列・struct・文字列・メモリ（L §§4–5,7,10、B）

| ID | 確認対象 | 状態 / 既存証拠 | 不足・追加対象 |
| --- | --- | --- | --- |
| AGG-01 | 固定長・定数長・推論長、key/穴/重複 | 検証済 A/D/H/E: [INFER] 全4件、[MEM] memory_errors_have_source_locations、[CONST] nested_aggregate_constants_and_implicit_zeros | source長の0/負数/非整数/非定数の全対照は追補 |
| AGG-02 | field名付きliteral、nested型明記、非addressable | 部分 A/D/H/E: [MEM] nested copy/positional field拒否、[型境界表](./type-boundary-conformance.md) 名前付きarray/struct定数・zero | nested型省略拒否、匿名struct literal拒否、複合型内の全要素型zero、暗黙heap allocation命令の不存在assertが残る。DECL-01の変数zeroとは区別 |
| AGG-03 | aggregate値コピーとsource順・1回評価 | 検証済 E/H: [MEM] コピー2件、[GLOBAL] aggregate_and_multiple_global_initializers_preserve_value_and_pointer_semantics、[INFER] runtime_elements_keep_source_order_and_holes_are_zero | 大型/多段入れ子のstressは未実施 |
| BOUND-01 | arrayの定数境界拒否、動的bounds check | 検証済 D/H/E: [MEM] arrays_including_string_literals_are_bounds_checked_but_raw_pointers_are_not、memory_errors_have_source_locations、[DONE] guard検査 | index6型×N=0/1/maxの行列は追加候補 |
| PTR-01 | &local/field/index、deref、raw index u32/unchecked | 部分 D/H/E: [MEM] alias/Ordered Load・Store、raw/no Check・型拒否、[P1制御表](./control-flow-conformance.md) pointer base→index・LHS snapshot | 評価順の代表例は解消。address_takenの複数経路、*struct暗黙deref、全placeのOrdered Store対照が残る |
| STR-01 | UTF-8・emoji・任意byte escapeとbyte len、raw string | 検証済 D/H/E: [MEM] string_literal_array_types_flow_through_globals_arguments_and_returns、[LEX] string検証 | import pathは値と別にvalid UTF-8を要求 |
| STR-02 | `[N]u8`型、完全長一致、copy/mutation/addressability、pointer非変換 | 検証済 D/H/E: [MEM] string_literals_are_mutable_byte_array_values_and_copy_independently、string_literal_lengths_and_pointer_types_require_exact_matches | 新MIR backend/実機でのaggregate ABIは別工程。文字列専用MIRは撤去済み |
| LEN-01 | len(array)は定数u32を返しoperandを1回評価 | 検証済 A/D/H/E: [CONST]/[VALUE] 副作用・faultを含むoperandの1回評価と型検査、[型境界表](./type-boundary-conformance.md) defined array | defined型とoperand評価は個別には追補済。両者を混ぜた組み合わせは追加候補 |
| BI-01 | alloc/free/load32/store32/newの型・arity・戻り値 | 検証済 D/H: [型境界表](./type-boundary-conformance.md) 引数位置別14型・arity・void、[DONE] intrinsic形状 | new([4]u8)→*[4]u8受理、*u8拒否。MMIO/allocator実行はbackend工程 |
| BI-02 | lenの型・arity、builtin名の予約、str廃止 | 検証済 D/H/E: [型境界表](./type-boundary-conformance.md) arity/14型/defined型、[CALL]/[DONE] `len`のbinding拒否、[SPEC] strは通常識別子 | 組み込み名全体の予約規則はNAME-03 |
| BI-03 | thisFile/thisLineの型・arity・論理位置 | 検証済 D/H/E: [型境界表](./type-boundary-conformance.md) arity・戻り値、backend実機テストで論理path・1始まり行・path共有を確認 | object間のpath共有は対象外 |
| MEM-01 | new zero/alignment、alloc failure/0、free nil等 | 別工程: coreではIntrinsicとTypeIdを確認。Rは旧ケース62–65/78–80 | 新MIR backend経由でallocator/zero/alignmentを再検証 |
| MEM-02 | stack/startup配置、escapeしても暗黙heap化なし | 部分 H: [MEM] address_taken、[GLOBAL] symbolic global | storage分類とIntrinsic不存在をcoreで確認。実アドレス/lifetimeは別工程 |
| MEM-03 | bounds check失敗のnon-return/fault契約 | coreはH/D: [MEM] Check::Boundsの生成・validator。実行は別工程K: [backend適合表](../../../specs/ond/targets/kagura-v1/conformance-checklist.md) KB-11 | targetでfault、後続副作用の抑制、debug operationを確認 |
| MEM-04 | lifetime/OOB raw/double freeなど検査しない領域 | 別工程/仕様上保証なし | 成功値を期待する適合ケースを作らない。coreが余分な保証を暗黙追加しない範囲を確認 |

### 制御フロー・関数・起動（L §§8–12、S §§4,7）

| ID | 確認対象 | 状態 / 既存証拠 | 不足・追加対象 |
| --- | --- | --- | --- |
| FLOW-01 | if/else-if、for3形式、最内break/continue | 検証済 E: [CFG]/[DISC] 制御文、[P1制御表](./control-flow-conformance.md) continue→post→条件、最内loopへのbreak/continue | continue後のpost順は追補済。深い入れ子のstressは未実施 |
| FLOW-02 | bool条件、必須block、loop外break/continue拒否 | 検証済 A/D/E: [P1制御表](./control-flow-conformance.md) 8負例＋loop/branch実行、control header26ケース | `if flag {}` / `for flag {}`の誤拒否を修正。composite literal対照・branch/post実行を追加し、ignoredを解除 |
| FLOW-03 | 全経路return・終端loopの判定 | 検証済 A/D/E: [P1制御表](./control-flow-conformance.md) 17受理/拒否ケースとreturn後の副作用停止 | branch欠落、break帰属、return/continue後のbreak、if(false)内break、条件付きloopを構造的判定の対照ケースとして明示 |
| CALL-01 | 0/1/複数戻り値、展開位置・個数・型 | 検証済 A/D/H/E: [CALL] multiple_results_are_evaluated_once_and_forwarded、statement_calls_discard_zero_one_or_many_results、call_errors_are_source_diagnostics_not_pending | tuple非存在はSYN-10の負例へ |
| CALL-02 | direct/indirect、forward/import/mutual recursion | 検証済 A/H/E: [CALL] calls_support_imports_forward_references_and_mutual_recursion、indirect_calls_evaluate_callee_then_arguments_and_keep_short_circuit、[FT] callback_arguments_returns_and_indirect_calls_reach_valid_mir | 名前省略型とaggregate/multi-returnを混ぜた実行値の追補 |
| INIT-01 | file辞書順/import DFS/宣言順/各package1回 | 検証済 H/E: [GLOBAL] dfs_initialization_preserves_import_file_and_declaration_order_once | 多段diamond・空package・無初期化packageの追加対照 |
| INIT-02 | 全global事前zero、内部init、推論/参照順独立 | 検証済 H/E/D: [GLOBAL] forward_types_do_not_reorder_initializers_and_all_storage_starts_zero、zero_only_declarations_do_not_reset_prior_initializer_writes、global_type_errors_and_inference_cycles_are_diagnosed | 全型のzero網羅はDECL-01 |
| INIT-03 | mainちょうど1つ/引数戻り値なし、init禁止 | 検証済 A/D: [scope](../src/conformance/scope.rs) INIT-03.* | 欠落/重複/型名衝突/引数・戻り値の各組み合わせ、別packageのmain、別packageを含むinit禁止 |
| OUT-01 | SDKによるmain終了処理、ABI/rodata/layout/static link | 別工程 R: runtime適合project・LinkedImage直接実行テスト | target適合表と機種SDKで検証する |
| OUT-02 | assetは言語機能外、非source fileの暗黙同梱なし | 別工程: [OLD-C] compile_project_does_not_predeclare_asset、cartridge::tests::writes_cartridge_header_without_implicitly_packaging_project_files | coreのasset予約名不存在・packaging境界の維持 |

### 診断とMIR検証（横断要件）

| ID | 確認対象 | 状態 / 既存証拠 | 不足・追加対象 |
| --- | --- | --- | --- |
| DIAG-01 | source errorでFrontendError、部分的成功を返さない | 検証済 D: [診断表](./diagnostic-conformance.md) 既存拒否テスト・複数独立エラー・global初期値・同一本体の文ごとの回復・generic定義/具体化診断 | 原因/severity/件数/file-spanとgeneric具体化のrelated labelを照合。依存する派生診断は抑制。backend faultは別工程 |
| DIAG-02 | Unicode/複数fileの診断位置 | 検証済 D: [診断表](./diagnostic-conformance.md) lexer/parser/HIR・import先・CRLF/4-byte文字 | 正確なUTF-8 byte範囲と同名識別子の出現位置を照合。internal MIR errorをsource errorとして許可しない |
| MIR-01 | ID/type/op/dominance/guard/global/initの不正IR拒否 | 検証済 H/D: [DONE] validator系、[MEM] corrupted place、[GLOBAL] corrupted globals、[IDENT] private owner | 言語受理の正しさとは独立。新命令追加時にmutation testを追加 |

## 3. Q-01〜Q-04の決定と反映

ユーザー決定を反映（2026-09-19）。[specification_decision_tests](../src/specification_decision_tests.rs)に5件追加。

- **Q-01 決定済**: if条件の括弧は不要だが使用可能。通常の式のグループ化。仕様文言修正と実行結果テストを追加。
- **Q-02 決定済**: 0戻り値はarrow省略だけ。関数宣言・関数型とも → () をparserで拒否。1件の括弧付き戻り値と末尾commaは維持。
- **Q-03 基本規則を明文化**: 通常callは未評価の短絡側でもconstにできない。一方、false && (1/0 == 0) は除算しない。定数文字列のlenと配列長lenを許可するが、len(array)のoperandは通常どおり1回評価する。未評価部分も名前解決・型検査をする。追補: [constant_contexts.rs](../src/conformance/constant_contexts.rs)で通常/未評価call、算術短絡、配列call/要素/明示castのlen評価、未評価部分の名前/型エラー、評価されるcast範囲エラーを照合。短絡側の明示castは値と範囲検査を省略し、`false && ((250 + 6) as u8 == 0)` と対応する `true ||` を受理する。評価される側の範囲エラーは維持する。通常の値文脈は[constant_value_tests.rs](../src/constant_value_tests.rs)で代入/引数/return/global、短絡の未評価call/castのMIR除去、len operandの1回評価、評価抑止状態が後続へ漏れないことを検証。
- **Q-04 更新済・実装済**: 組み込み型名と`alloc`、`free`、`len`、`trap`、`sizeof`、`alignof`、`load32`、`store32`、`thisFile`、`thisLine`、`new`はshadow不可。type/var/const/:=/関数/関数宣言parameterのbinding名を禁止する。field名と関数型の説明用parameter名はbindingを作らないので対象外。廃止した`str`は通常識別子として利用できる。

旧NAME-03の「事前定義型shadow許可」はこの決定で撤回した。既存実装をshadow可能に変えるのではなく、宣言を拒否する方針へ仕様を変更した。

## 4. 次に追加する順序

以下は当初計画の実施状況。完了は各作業の成果物についての判定であり、§2全行の完了ではない。

- [x] **P0: 適合ケースの共通形式** — [共通形式とrunner](./conformance-cases.md)を追加。spec ID、source files、期待受理/拒否、型/値/診断、Parse/Core層を明示。代表5ケースを登録。既存テストは移設せず維持。
- [x] **P0: scope/grammarの穴を先に固定** — SYN-07、PKG-01～03、INIT-03の共通ケース追加済。Q-01/02/04は§3の既存テストを利用。Q-03は25ケースを登録し、短絡側castの非評価も決定・実装・テストへ反映済。
- [x] **P0: 数値の仕様表をテストデータ化** — [数値表](./numeric-conformance.md)に751共通ケース、336 runtime整数oracle vectorsを追加。6整数型・境界・全operator、180関数の動的MIR契約を明示。runtime結果のbackend検証は引き続き別工程。
- [x] **P1: 制御フローと評価順の組み合わせ** — [制御表](./control-flow-conformance.md)の85ケースとMemoryVm層を追加。副作用traceと書込結果を照合。空blockのparser不具合を修正し、control header26ケースを通常実行（計111ケース、ignoredなし）。
- [x] **P1: 型/値境界の負例行列** — [型境界表](./type-boundary-conformance.md)に1,366ケースを維持。CAST-01、NIL-01、OP-01、BI-01/02/03、DECL-01/02の受理/拒否・診断位置・型/zero値を検証。`str`型・constructor廃止を反映。
- [x] **P1: 診断の期待値強化** — [診断表](./diagnostic-conformance.md)。compiler-coreのis_errだけのテストを原因照合に置換。既存compile拒否は件数/severity/file-spanも検証し、Unicode・複数fileの6ケースと誤期待値の負例を追加。後続で同一本体の複数エラー、genericの定義時名前検査、具体化chainと重複抑制を追補。
- [x] **P2: coreの適合状況を更新** — 文字列仕様変更後に証拠を再照合。全419テストを再実行し、古い不足記述・実行件数を更新。core内の残件とbackend待ちを下記に分離。完全適合の宣言ではない。
- [ ] **別工程: MIR backend適合** — Rで検証している旧ケースを、新しいbackend経路で実行し直す。

### 次のcore内追補（未実施）

既存証拠で縮めた後の残件を、着手可能な単位にまとめる。今回追加実装したものではない。

| 順序 | ID | 完了判断に必要な追加検証 |
| --- | --- | --- |
| 1 | SYN-02/06/08/10/12 | 不正UTF-8 loader、LF/CRLF同一入力、非入れ子comment、semicolon対象token、list境界、未対応構文の拒否表 |
| 2 | PKG-04、NAME-03 | symbol種×公開/非公開×未import、残るbuiltin×binding scope、newのbinding拒否 |
| 3 | TYPE-03/04、NIL-02 | 関数戻り値型/個数/順序の違い、相互/array/関数型経由の再帰、nil calleeのIndirect call保持・signature |
| 4 | NUM-01/02 | literal/定数式×6整数型×local/global/引数/returnの文脈対照。型付き変数のCAST-01とは別表 |
| 5 | AGG-01/02、PTR-01、STR-02、MEM-02 | 長さの異常値・nested literal制約、全要素型zero、place属性、byte配列literalのaggregate ABI、暗黙heap化なし |

検証済行のstress・追加組み合わせ（CALL-02/INIT-01/BOUND-01等）は各行に残す。
これらの不足だけから実装バグや未実装と断定しない。

### backend/runtimeへ引き継ぐ未完了契約

- CAST-02/03、NUM-03/04/05、FP-02: 幅・符号・wrap・shift・cast・fault・binary32 bitsを生成MIRの実行結果で照合する。336整数vectors/25float vectorsを再利用するが、oracleの自己検証とは別に実行する。
- NIL-02、MEM-01/03: null間接callのCPU fault、allocatorのzero/alignment/failure/free、bounds errorのnon-return/faultをtarget経路で確認する。
- MEM-02、STR-01、OUT-01/02: stack/global/.rodata/ABI・寿命・終了コード・link/cartridge/asset packagingを担当工程で検証する。coreはsymbolicなstorage・型・命令まで。
- MEM-04: 仕様上保証しない動作に成功値の期待を追加しない。別工程へ送ったことを「後で安全性を保証する予定」と読み替えない。

旧AST backendのRテストや簡易MIR VMのEテストだけでは、この引き継ぎ項目を完了にしない。

### 数値テストのoracleに関する注意

[既存memory_test_vm](../src/memory_test_vm.rs)は一部演算だけを扱い、
整数Add/Multiplyはhost u64演算、Castは値を複製している。
[control_flow_tests](../src/control_flow_tests.rs)のinterpreterもSignedDivisionOverflowを実検査せず、
小さな非負値を主に扱う。これらをそのまま6型のwrap/変換/float適合oracleにしてはいけない。

数値のcoreテストでは、まずconstantの期待bitsと動的MIRのopcode・operand/result型・必要guardを独立にassertする。
MIR実行値を確認するなら、言語仕様どおりの幅/符号/floatを扱うテスト実行器または別oracleを用意し、
compilerと同じ定数評価関数を期待値生成に再利用しない。物理的なfault、allocator、ABIはtargetで確認する。

## 5. テスト索引

各リンク先は監査したsource。表中の関数名は当該suite内のtest関数を指す。

- [LEX]: 字句11件。token種/原文/byte span、数値と文字列の変換、拒否例。
- [PARSE]: parser7件。基本signature、semicolon、コメント、package clause。
- [BASE]: core基本統合6件。型付きHIR・MIRと型table。
- [SEM]: 意味検査4件。定数/名前/型エラーとsource診断。
- [CFG]: CFG統合1件。複数関数・入力で短絡/loop/scope/branchを実行。
- [CALL]: call6件。複数戻り値、call trace、間接call、拒否。
- [MEM]: memory10件。aggregate/pointer/byte-array string literal、boundsと不正place。
- [GLOBAL]: global9件。zero/init順序、推論、global IR検証。
- [DISC]: blank/syntax10件。破棄・:=・文種制限。
- [DONE]: completion11件。builtinとMIR mutation validator。
- [CONST]: 定数7件。任意精度・shift・aggregate・f32代表値。
- [VALUE]: 値境界5件。必須評価と非評価規則。
- [INFER]: 推論長4件。長さ・key・型適合・拒否。
- [IDENT]: 型同一性7件。構造/nominal/packageとowner validator。
- [FT]: 関数型4件。optional name AST/型同一性/callback/拒否。
- MIR内部unit testはmir/validate*.rs・mir/display.rs等にもある。上記suiteの件数は全311件の内訳一覧ではない。
- [共通registry](../src/conformance/cases.rs): scope/precedence/constant contexts、制御/名前、診断の固定ケースと、生成ケースのID重複検査。
- [数値表](./numeric-conformance.md)、[型境界表](./type-boundary-conformance.md): 生成行列・inventory・MIR契約検査。
- [制御表](./control-flow-conformance.md)、[診断表](./diagnostic-conformance.md): P1の実行/診断契約と検証範囲。
- [仕様決定](../src/specification_decision_tests.rs): Q-01〜04の決定反映。runner自身の負例は[conformance/tests.rs](../src/conformance/tests.rs)。
- [OLD-C]: 既存compiler統合テスト。coreを通るsource拒否ケースは参考にできるが、新MIR実行の代替にはしない。

[LEX]: ../src/ond/lexical_tests.rs
[PARSE]: ../src/ond/tests.rs
[BASE]: ../src/tests.rs
[SEM]: ../src/semantic_tests.rs
[CFG]: ../src/control_flow_tests.rs
[CALL]: ../src/call_tests.rs
[MEM]: ../src/memory_tests.rs
[GLOBAL]: ../src/global_tests.rs
[DISC]: ../src/syntax_discard_tests.rs
[DONE]: ../src/completion_tests.rs
[CONST]: ../src/constant_tests.rs
[VALUE]: ../src/constant_value_tests.rs
[INFER]: ../src/inferred_array_tests.rs
[IDENT]: ../src/type_identity_tests.rs
[FT]: ../src/function_type_tests.rs
[OLD-C]: ../../compiler/src/pipeline/tests.rs

### P0追補で修正した不一致

未importのpackageが存在すると、startup validatorが全packageを初期化順に要求し内部エラーになっていた。
L §11のmain起点のimport到達範囲に合わせ、初期化順は到達するpackageだけを要求するよう修正。
未到達packageの型・global・initializer signature・import先IDの検証は維持し、[GLOBAL]のmutationテストで確認する。

### P0数値表の追補

`cargo test --workspace --quiet` 全297テスト成功（compiler-core 190件）。
数値表内751ケースとRust test関数数は別に数える。f32除算が誤ってChecked/trappingになっていたMIR生成を修正した。

### P1制御フロー・評価順の追補

初回追加時は383成功・2 ignored。ユーザー承認後にparserを修正し、保留2件を解除、24件を追加した。
この追補時点ではworkspace全409成功・ignoredなし（compiler-core: 302成功）。最新値は§1を参照。
詳細・再現コマンド・検証範囲は[制御表](./control-flow-conformance.md)を参照。
