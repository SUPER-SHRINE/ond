# MIR → Kagura backend 適合チェックリスト

監査日: 2026-09-20。
対象はsource → core/MIR → Kagura backend → Object → linker/startup → Kagura CPU。
規範は[言語仕様](../../v1/README.md)、[Machine Profile](./machine.md)、[ABI](./abi.md)。実装契約は[MIR backend設計](./mir-backend.md)。
初回は棚卸しのみ。2026-09-20追補で§5の共通実行口・数値/評価順・複合ケースを実装した。標準backend切り替えは行わない。

## 1. 判定と証拠の区別

- **検証済**: 行に限定した契約をassertしている。全入力・組み合わせの網羅を意味しない。
- **部分**: 代表例は検証しているが、仕様表の型/境界/組み合わせに不足がある。
- **不足**: 調査対象のMIR backendテストで直接の証拠を確認できない。未実装・バグと断定しない。
- **別工程**: coreの受理/拒否、将来の最適化、Enbu等。backendの不足と混同しない。

証拠の層:

| 記号 | 意味 | 限界 |
| --- | --- | --- |
| K | 通常sourceからMIRを生成し、link後の命令をKagura CPUで実行 | テストがassertした出力・状態だけが証拠 |
| KM | source由来MIRを加工してKagura実行 | 加工した構造をsourceから生成できる証拠ではない |
| KL | Emitter/Objectを直接構築してKagura実行 | source/MIR lowering全体の証拠ではない |
| O | layout/ABI/Object/relocationの具体的assert | CPU実行ではない |
| D | 診断原因等のassert | 単に実行がfaultしたこととは別 |
| C / R | coreテスト・簡易MIR VM / 旧AST backend実行 | どちらも新backend実行済みに数えない |

2026-09-22に`cargo test --workspace --quiet`を再実行: **499 test関数成功、失敗/ignored 0**。
CLI library 6、CLI process 6、compiler 108、compiler-core 314、LSP 65。compiler件数には設定等も含むため、すべてをK検証とはしない。vector数をtest関数数に加算せず、仕様達成率も算出しない。

coreの[チェックリスト](../../../../crates/compiler-core/docs/conformance-checklist.md)は当時の監査記録として維持する。そこにある「backend待ち」の最新の対応状況は本表を参照する。
旧runtimeケース表の判定は新backendへ転記しない。machineの共通suiteは標準MIR入口の`tests`で実行する（§7）。旧ケース表の全行を自動的にMIR検証済みとはしない。

## 2. チェックリスト

証拠欄の略称は§3のリンク先suiteを指す。関数名は実在するtest名。
core IDは元表への対応付けであり、その行の全契約をbackendが引き受けるという意味ではない。

### 数値・型・メモリ

| ID / 対応するcore項目 | 確認対象 | 判定 / 証拠 | 残検証 |
| --- | --- | --- | --- |
| KB-01 / TYPE、MEM-02 | scalar size/align、struct padding、array stride、ABI layout | 検証済 O/K: BASIC `scalar_layout_and_abi_are_independent_of_stack_slots`、MEM `layout_and_abi_have_independent_fixed_expectations`、`natural_struct_layout_nested_arrays_and_byte_literals_execute` | 再帰pointer・defined型を含むより広い実行行列は追加候補 |
| KB-02 / NUM-03/05 | 6整数型のwrap、bitwise、shift、signed/unsigned比較 | 検証済 K（列挙範囲）: CON-NUM `shared_336_integer_vectors_execute_on_kagura`、BASIC | 全operator×6型、shift count 0/N−1/N/N+1/u32::MAX、比較境界を共有vectorで照合。全入力bit列の網羅ではない |
| KB-03 / NUM-04 | div/rem、0、signed min/-1 | 検証済 K（列挙範囲）: NUM `integer_division_remainder_widths_and_boundaries_execute`、`signed_division_vectors_cover_all_sign_combinations` | 6型の通常値/0、signed min/-1、u32高bit、i32固定乱数は確認済。coreの全vector共通化は未実施 |
| KB-04 / CAST-02 | 整数castの幅・符号・wrap | 検証済 K（列挙範囲）: CON-NUMの共有336vector中72 castケース | 36方向×source min/maxを照合。全destination境界×source値の追加は候補 |
| KB-05 / FP-01/02 | f32四則・丸め・NaN/Inf/zero/subnormal | 検証済 K（列挙範囲）: NUMの固定/乱数、CON-NUM `shared_25_float_bit_vectors_execute_on_kagura` | coreと同じ25入力/期待bitsを接続済。NaN全payload、全bit列の網羅ではない |
| KB-06 / FP、CAST-03 | f32比較6種・negate・整数変換 | 検証済 K（列挙範囲）: NUM、CON-NUM `all_twelve_float_conversion_directions_and_boundaries_execute` | 12方向・min/max/直外側/2^24 ties-even/NaN/Inf/小数を93ケースで照合。新テストは失敗時の格納先sentinel非破壊も確認。比較全入力の網羅ではない |
| KB-07 / PTR-01、MEM-02 | typed pointer、二重pointer、index、狭幅load/store | 検証済 K（代表例）: MEM `pointer_locals_narrow_accesses_and_offsets_execute` | 全scalar型・defined型のcross-call aliasは追加候補 |
| KB-08 / NIL-01/02 | pointer/function nilの値・比較・call fault | 検証済 K（代表例）: MEM/GLOBAL、CON-EVAL `faults_do_not_execute_following_effects_or_overwrite_destination` | nil function callのBusFaultと後続副作用なしを追加済。nil比較の全位置/全型行列とは別 |
| KB-09 / AGG、STR | array/struct/byte文字列literalの値コピー | 部分 K: MEM `natural_struct_layout_nested_arrays_and_byte_literals_execute`、`mixed_memory_returns_and_register_returns_execute` | raw/Unicode/escape/空literal、推論長・defined aggregate・global/引数/returnの対照。`str`は廃止済みなので対象に戻さない |
| KB-10 / MEM-02 | ゼロサイズのlayoutと物理storage | 検証済 O/K: MEM `zero_size_storage_is_non_nil_and_layout_remains_zero`、GLOBAL/HEAPの空struct・[0]T | 異なるゼロサイズ値のaddress同一性は仕様保証外でありassertしない |
| KB-11 / BOUND-01、MEM-03 | bounds guardとfault | 部分 K: MEM、CON-EVALの負index/length/0長/length−1 | 格納先と前後guard・後続副作用をsentinelで照合済。nested indexの各段階fault順は不足 |
| KB-12 / MEM-02、EVAL | Ordered copy/read/writeの順序・padding・alias・途中fault | 検証済 K+bus trace: ORDER全5件 | 宣言/index順、全read→write、重複領域、非rollback、bool正規化、u16 alignment fault、raw offset wrapをassert。全幅/全fault位置の行列ではない |

### 制御フロー・ABI・frame

| ID / core項目 | 確認対象 | 判定 / 証拠 | 残検証 |
| --- | --- | --- | --- |
| KB-13 / FLOW、EVAL-01 | if/for、break/continue、短絡・block parameter | 部分 K: BASIC `locals_branches_loops_and_boolean_block_parameters_execute`、BRANCHの大loop | 短絡側fault抑止と基本loopは確認済。入れ子loop・post/condition順・選択edgeだけの副作用・parallel copy循環の専用実行例が不足 |
| KB-14 / EVAL-01/02 | callee→引数、複数LHS/RHS、書込順、snapshot | 検証済 K（列挙範囲）: CON-EVAL `shared_25_evaluation_cases_preserve_trace_and_final_values`、`discard_and_callee_evaluation_keep_effects_and_multi_result_positions`、MEM/ORDER | coreの25 sourceを共有しtrace＋全戻り値を照合。callee→引数も追補。任意CFG/全alias形状の網羅ではない |
| KB-15 / DECL-03 | `_`で破棄してもcall/fault/評価順を維持 | 検証済 K（列挙範囲）: CON-EVALのdiscard/破棄した除算fault | global/local破棄・複数戻り値の穴・blank parameter・副作用順を確認。storage不生成の構造的証拠はcore側 |
| KB-16 / CALL | 直接/間接call、再帰、6/7引数境界、引数spill | 検証済 K/O（代表例）: BASIC `direct_calls_recursion_stack_arguments_and_spills_execute`、MEMの間接aggregate call | scalarとaggregate再帰は確認済。相互再帰・遠距離・大frameの複合はKB-23 |
| KB-17 / CALL、AGG | aggregate引数copy、hidden return pointer、複数戻り値 | 検証済 K/O（代表例）: MEM `aggregate_arguments_returns_and_snapshots_execute`、`mixed_memory_returns_and_register_returns_execute` | 4/5 scalar戻り値、mixed record、hidden pointer＋stack引数を確認。長距離thunk越しの同一行列は不足 |
| KB-18 / MEM-02 | >32 KiB frame、spill/address、引数/return領域 | 検証済 K/KM/KL: FRAMEの大frame、36 KiB配列、offset 32764/32768/65536/131072 | 多数local追加MIRを用いた40 KiB超のABI検証と通常source配列を区別する。outgoing引数領域自体が32 KiB超のend-to-endは不足 |
| KB-19 / MEM-03 | stack不足検出、SP/callee-save維持、32-bit計算overflow | 検証済 K/KM/D（列挙範囲）: BASIC/FRAME、CON-COMB `large_recursive_frames_fit_exact_stack_then_fault_one_level_later` | 大frame再帰のexact fit/次段不足/物理guard/SP維持、人工的な低SPで減算underflowを追補。全容量組合せではない |

### Link・startup・runtime・配布経路

| ID / core項目 | 確認対象 | 判定 / 証拠 | 残検証 |
| --- | --- | --- | --- |
| KB-20 / OUT | 関数内の正負距離境界・条件不成立時のscratch保持 | 検証済 K/KL/O: BRANCH `conditional_local_islands_execute_both_directions_and_skip_literals`、`large_loop_and_conditional_edges_execute_from_source`、BASIC距離境界 | ±signed16境界、2 load baseを検証。中継共有/圧縮は実装契約外 |
| KB-21 / OUT、CALL | CallSlot/JumpSlot、元r15・引数維持、literal skip | 検証済 KL/K/O: LINK-BRANCH全3件、BRANCH `far_startup_initializer_calls_stack_arguments_and_function_addresses_execute` | 正負境界、addend、複数site＋Abs32配置不変、7引数、startup/initを検証。旧Call16距離超過は明示拒否のまま |
| KB-22 / INIT | 全BSS zero→MIR指定順init→main、fault停止 | 検証済 K/O/D: GLOBAL `all_globals_start_zero_before_any_initializer`、`startup_preserves_mir_order_diamond_once_and_skips_unreachable`、`initializer_fault_stops_before_later_initializers_and_main`等 | package並びを逆転しても順序維持、diamond一度、未到達init除外を確認。最後のfault種はgeneric err検出であり厳密分類ではない |
| KB-23 / CALL、MEM | 長距離＋大frame＋aggregate ABI＋helperの組合せ | 検証済 KM/K/O（列挙範囲）: CON-COMB `far_calls_large_frames_hidden_returns_stack_arguments_and_helpers_coexist` | 遠距離aggregate return＋hidden pointer＋stack args、大frame再帰＋f32、初期化から遠距離new helperを確認。既存MIRにunused localを追加したstressであることを明示する |
| KB-24 / MEM-01 | alloc/free/new、zero/alignment、split/coalesce、容量不足 | 検証済 K（列挙範囲）: HEAP全4件 | global init中new、new(0-size)非nil成功/失敗、alloc(0)、free(nil)、exact heap容量を確認。長い操作列のモデル比較は追加候補。不正free/UAFの保証は追加しない |
| KB-25 / OUT | helper依存閉包・dedup・予約symbol・MIR供給 | 検証済 O/D/K: RUNTIME全5件、HELPERの実CPUテスト、NUM/HEAP | compile全helper成功を全helper入力の実行適合とはみなさない |
| KB-26 / OUT、DIAG | relocation/RAM/サイズ/未対応MIR診断 | 部分 D/O: LINK-BRANCH、GLOBALの容量、BASIC `unsupported_checked_operations_never_silently_wrap` | backend診断は主に原因substring。一部はsource span非syntheticまで。複数file/正確なbyte span、旧Jump16等の負例、異常Object全般の網羅は不足 |
| KB-27 / OUT | Enbu CLIのbuildと機種側のrunと失敗時の成果物保護 | 検証済 K/D（smoke）: CLI、MACHINEの新経路suite | CLIの数値/heap/global/aggregateとframe overflow時の既存cart非上書き、machine適合projectの完全出力列を確認。標準CLIはMIRへ切替済み。廃止した`--mir`/単独`.kg`入力も拒否 |
| KB-28 / OUT | LinkedImage直接実行による適合project/負例 | 検証済 K（20件）: MACHINE suite | 正常終了/出力列、nil call/array bounds/NaN cast/0除算fault、retired count。Enbu loaderの検証とは別 |
| KB-29 / OUT | asset/cassette、Enbu banked ROM | 別工程 | この監査は既存Ond-Kagura profile。resource ID生成、Enbu配置、bank切替の完成を主張しない |
| KB-30 / SYN、NAME、TYPE、DIAG | 字句/構文/型/名前の受理・拒否行列 | 別工程 C | coreの適合表が正本。受理された値の物理表現/実行は上記KB項目へ分離する |

## 3. 証拠索引

- BASIC: [mir_backend/tests.rs](../../../../crates/compiler/src/mir_backend/tests.rs)。`run`/`execute`は新経路。正常終了時のSP/callee-saveも照合。
- NUM: [numeric_tests.rs](../../../../crates/compiler/src/mir_backend/numeric_tests.rs)。動的引数またはbus入力を使い、定数畳み込みだけで通らない構成。
- MEM: [memory_tests.rs](../../../../crates/compiler/src/mir_backend/memory_tests.rs)。型配置、pointer、array/struct、ABI。
- ORDER: [ordered_tests.rs](../../../../crates/compiler/src/mir_backend/ordered_tests.rs)。実CPUのbus access traceと書込結果。
- GLOBAL: [global_tests.rs](../../../../crates/compiler/src/mir_backend/global_tests.rs)。init計画、BSS、容量、fault。
- HEAP: [heap_tests.rs](../../../../crates/compiler/src/mir_backend/heap_tests.rs)。allocator操作と境界。
- BRANCH: [branch_tests.rs](../../../../crates/compiler/src/mir_backend/branch_tests.rs)。source統合とEmitter直接試験。
- FRAME: [frame_tests.rs](../../../../crates/compiler/src/mir_backend/frame_tests.rs)。source/KM/KLをそれぞれ区別。
- LINK-BRANCH: [test_support/branch_tests.rs](../../../../crates/compiler/src/test_support/branch_tests.rs)。Object直接構築・link・CPU。
- RUNTIME: [runtime/tests.rs](../../../../crates/compiler/src/mir_backend/runtime/tests.rs)。依存選択/診断/生成Object。
- HELPER: [test_support/runtime_tests.rs](../../../../crates/compiler/src/test_support/runtime_tests.rs)。Object call targetをhelperへ差し替えた実行。通常sourceからの命令選択はNUM/HEAPが担当。
- CLI: 別リポジトリの `enbu/crates/enbu-cli/tests/cli.rs`。標準MIR経路。
- MACHINE: [machine_suite.rs](../../../../crates/compiler/src/test_support/machine_suite.rs)。テスト専用busへLinkedImageを直接loadし、実Kagura CPUで実行する。

## 4. 次に追加する順序と合格条件

- [x] **棚卸し**: 実行層を区別し、既存のassert範囲と不足を対応付ける（本書）。
- [x] **P0: Kagura適合ケースの共通実行口**（KB-02〜06/14）: §5のrunnerとfeature-gated fixture共有を実装。coreをtarget非依存のまま維持し、既存テストはadapterで継続。production machine suite全体の移行は下記P1として残す。
- [x] **P0: 整数/変換/floatの表接続**（KB-02〜06）: 336整数vector・25float bitsを同じデータからKagura実行。int↔f32の12方向93境界ケースを追加。期待値はcompilerの定数評価器から作らない。
- [x] **P1: 評価順・破棄・fault**（KB-08/11/14/15）: evaluation 25ケースのtrace＋値、callee評価順、破棄、副作用抑止、nil call、bounds負値/0長、cast失敗時のsentinelを接続。制御header全26件・nested bounds・任意parallel copy循環等はKB-11/13の追補として残す。
- [x] **P1: ABI/容量の組合せ**（KB-17〜19/23）: 長距離aggregate return＋stack引数＋大frame再帰＋helperと、stack exact fit/次段faultを追加。32 KiB超のoutgoing領域そのものはKB-18に残す。
- [x] **P1: production machineへの適合suite接続**（KB-27/28）: 既存21件を共通suiteへ分離し、旧/MIR両入口で実行。runtime-pass期待出力列を変更せず、fault種/未終了状態を照合（§6）。
- [ ] **P2: 診断と状態更新**（KB-26）: backend/linkの失敗原因・source spanの保証範囲を固定。各追加ケースのID/証拠を本表へ追記し、実行されていないcaseや別工程を完了にしない。

通常ビルドのMIR経路への切り替えと旧AST backendの撤去は完了した（§7）。中継領域圧縮、aggregate命令展開の抑制、SSA/最適化は適合検証完了とは別の課題。


## 5. 2026-09-20 接続実装の記録

- 共通runner: [runner.rs](../../../../crates/compiler/src/mir_backend/conformance/runner.rs)。複数source filesをcompileして得たLinkedImageに、case ID/spec IDs・動的入力word・期待出力word列・Return/FaultCode・step上限を与える。実CPUを使用し、正常終了時のexit=0/SP/callee-save、fault時のexit未実行と出力sentinelを照合する。traceは出力word列の一部として最終値と同時に検査する。誤った値・期待fault・step不足のrunner負例もある。
- 共有データ: coreの[test_vectors](../../../../crates/compiler-core/src/test_vectors.rs)。整数336件、float bits 25件、評価順25件を一箇所に保持。整数castはsource_tyも明示する。coreのoracle/定数/MemoryVm検証を維持し、compilerのdev-dependencyだけが `conformance-fixtures` を有効にする。通常ビルドには公開されない、不安定なテスト用API。coreにKagura依存を導入しない。
- CON-NUM: [numeric.rs](../../../../crates/compiler/src/mir_backend/conformance/numeric.rs)。busからの動的入力を用いて定数畳み込みを避ける。同sourceはcompile結果を再利用するが実行ごとにCPU/RAMをリセットする。float castの93ケースは既存core表と同じ境界を固定bitsで明示し、signed結果はu32 bit patternで比較する。狭いcastの成功時も上位sentinel保持を確認する。
- CON-EVAL: [evaluation.rs](../../../../crates/compiler/src/mir_backend/conformance/evaluation.rs)。共有sourceの空mainだけをtest()の戻り値出力wrapperへ置換する（置換対象一つをassert）。import用fileもそのまま渡す。25件＋callee/破棄2件＋fault5件＋bounds成功1件。nil callはBusFault、除算/配列guardはInvalidInstructionを区別。負indexと上限外の書込先にもsentinelを置く。
- CON-COMB: [combinations.rs](../../../../crates/compiler/src/mir_backend/conformance/combinations.rs)。sourceからのMIRへunused scalar localを追加して約40 KiB以上のframeにするため、KMとして扱う。symbol距離とhelper call relocationをassertして、単に大きなプログラムを生成しただけでは通らない。入力aggregateの非alias・複数戻り値・初期化中new・再帰内f32 helperを最終値で確認する。
- stack境界試験はframe planから必要容量を算出し、その明示設定をLinkPlanへ渡す。extra recursionでは同じ設定のままfaultさせる。fault後のSPとSTACK_BOTTOM直下guardを独立に確認。実装がプロジェクトstack設定を自動拡張したわけではない。

追加は9 Rust test関数。既存core 312を維持し、workspace計486成功。表内の入力数はRust test関数数に加算しない。
この時点でproduction machine適合suite接続が残っていたが、§6で実施済み。表に残した追加境界・診断、標準backend変更、Enbu対応は別工程として残る。


## 6. 適合suiteの実行口

MIR → Object → linker → LinkedImageをテスト用busへ直接読み込み、実Kagura CPUで実行する。ファイル形式のencode/decodeを経由しない。専用の旧配布形式に依存していた検証は撤去し、通常の言語動作・fault・retired countの20件をtest_supportへ移した。

- conformance projectの全出力列とexit=0を固定期待値で照合する。
- nil call、array bounds、無効float cast、整数0除算のfaultと副作用を確認する。
- step上限は有界であり、予算不足を成功扱いしない。
- Enbuのbank切替・boot・resourceはenbu-sdkとEnbu C machine APIで別途検証する。
- source → MIRの標準経路、LSPのcompiler-core独立性、CLI失敗時の既存ROM保持は維持する。

この変更はテストの入力形式をLinkedImageへ変更するものであり、表の未検証項目や将来の最適化を完了扱いにしない。
