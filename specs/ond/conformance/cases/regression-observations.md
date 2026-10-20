# 回帰入力の外部観測
<!-- ond-spec-id: conformance/cases/regression-observations -->

版、manifest、phase、fresh初期状態、値と診断の比較は[共通契約](./README.md#入力と期待結果の読み方)に従う。\
各節の識別名は対応する回帰入力の追跡名であり、入力番号は同じ節の独立したprojectを表す。\
同じ入力内の`main.ond`、`other.ond`、`shared/shared.ond`は同じprojectへ置く。\
規範定義元は各要件IDの[定義元索引](../requirements.md)を参照する。\
比較対象は各節に記す型、値、診断、stateと停止結果とする。

## delimiters_empty_statements_and_trailing_call_comma_are_supported
<!-- ond-spec-id: conformance/regression/delimiters-empty-statements-and-trailing-call-comma-are-supported -->

要件: `SYN-08`。\
操作・期待: MemoryVm: main.result([]) -> [5]

入力1・`main.ond`:
```ond
package main
const (A = 1)
var (B: i32)
type (S struct { Value: i32 })
func id(x: i32) -> i32 { return x }
func main() { ;; }
func result() -> i32 {
    ; var (x = A); const (C = 2)
    for ; x < 3; x = x + 1 { ; }
    for ;; { break }
    if ; x == 3 { x = x + C }
    return id(x,)
}
```

## call_statements_are_allowed_in_control_flow_clauses
<!-- ond-spec-id: conformance/regression/call-statements-are-allowed-in-control-flow-clauses -->

要件: `SYN-09`。\
操作・期待: MemoryVm: main.result([]) -> [5]

入力1・`main.ond`:
```ond
package main
var N: i32
func step() { N = N + 1 }
func main() {}
func result() -> i32 {
    if step(); N == 1 { step() }
    for step(); N < 5; step() {}
    return N
}
```

## discards_preserve_calls_and_multi_value_positions
<!-- ond-spec-id: conformance/regression/discards-preserve-calls-and-multi-value-positions -->

要件: `DECL-03`。\
操作・期待: MemoryVm: main.result([]) -> [3,12121234]

入力1・`main.ond`:
```ond
package main
var Log: i32
func mark(v: i32) -> i32 { Log = Log * 10 + v; return v }
func pair() -> (i32, i32) { return mark(1), mark(2) }
func main() {}
func result() -> (i32, i32) {
    var _, a = pair()
    _, b := pair()
    a, _ = pair()
    var _, _: i32 = mark(3), mark(4)
    return a + b, Log
}
```

## blank_parameters_keep_argument_slots_but_no_binding
<!-- ond-spec-id: conformance/regression/blank-parameters-keep-argument-slots-but-no-binding -->

要件: `DECL-03`。\
操作・期待: MemoryVm: main.result([]) -> [42]

入力1・`main.ond`:
```ond
package main
func choose(_: i32, value: i32, _: bool) -> i32 { return value }
func main() {}
func result() -> i32 { return choose(10, 42, true) }
```

## discard_never_suppresses_evaluation_faults
<!-- ond-spec-id: conformance/regression/discard-never-suppresses-evaluation-faults -->

要件: `DECL-03`。\
操作・期待: MemoryVm: main.result([]) -> division by zero fault

入力1・`main.ond`:
```ond
package main
func main() {}
func result() { var z: i32; _ = 1 / z }
```

## short_declarations_reuse_only_current_scope_variables
<!-- ond-spec-id: conformance/regression/short-declarations-reuse-only-current-scope-variables -->

要件: `DECL-04`。\
操作・期待: MemoryVm: main.result([]) -> [5,6]

入力1・`main.ond`:
```ond
package main
func main() {}
func result() -> (i32, i32) {
    a := 1
    { a, _ := 2, true; a, b := 3, 4; _ = b }
    a, b := 5, 6
    const _, _ = 1, true
    const _: *u8 = nil
    var _: func() = nil
    return a, b
}
```

## anonymous_structs_match_across_files_and_function_signatures
<!-- ond-spec-id: conformance/regression/anonymous-structs-match-across-files-and-function-signatures -->

要件: `TYPE-01`。\
操作・期待: Core: 全fileを同じprojectとして受理。匿名structの位置と関数parameterの説明名は型同一性へ影響しない

入力1・`main.ond`:
```ond
package main
func main() {}
func result() -> i32 { var a: struct { x: i32; }; a.x = 7; var b: struct { x: i32; }; b = echo(a); var f: func(v: struct { x: i32; }) -> struct { x: i32; } = echo; b = f(b); return b.x; }
```

入力1・`other.ond`:
```ond
package main
func echo(v: struct { x: i32; }) -> struct { x: i32; } { return v; }
```

## nested_arrays_pointers_and_function_parameter_names_are_structural
<!-- ond-spec-id: conformance/regression/nested-arrays-pointers-and-function-parameter-names-are-structural -->

要件: `TYPE-03`。\
操作・期待: Core: 受理

入力1・`main.ond`:
```ond
package main
func main() { var a: [2]*struct { x: [3]u8; }; var b: [2]*struct { x: [3]u8; }; b = a; var f: func(first: struct { x: i32; }) -> [2]u8; var g: func(second: struct { x: i32; }) -> [2]u8; g = f; }
```

## defined_structs_stay_distinct_but_matching_underlying_casts_work
<!-- ond-spec-id: conformance/regression/defined-structs-stay-distinct-but-matching-underlying-casts-work -->

要件: `TYPE-02`。\
操作・期待: 入力1: MemoryVm main.result([]) -> [7]。入力2: Core compile error、Error1件、contains type mismatch、main.ondの最後のaのUTF-8範囲

入力1・`main.ond`:
```ond
package main
type A struct { x: i32; }
type B struct { x: i32; }
func main() {}
func result() -> i32 { var a = A{x: 7}; var b = a as B; return b.x; }
```

入力2・`main.ond`:
```ond
package main
type A struct { x: i32; }
type B struct { x: i32; }
func main() { var a: A; var b: B = a; }
```

## nested_aggregates_copy_by_value_and_pointers_alias_storage
<!-- ond-spec-id: conformance/regression/nested-aggregates-copy-by-value-and-pointers-alias-storage -->

要件: `AGG-03, PTR-01`。\
操作・期待: MemoryVm: main.run([]) -> [1,7,9,7,0,0]

入力1・`main.ond`:
```ond
package main
type Point struct { x: i32; y: i32; }
type Box struct { p: Point; values: [3]i32; }
func main() {}
func run() -> (i32, i32, i32, i32, i32, i32) {
    var a = [3]i32{1: 7, 8}
    var s = Box{p: Point{x: 1}, values: a}
    var copied = s
    var p = &s.p.x
    *p = 9
    s.values[1] = 3
    a[0], a[1] = a[1], a[0]
    return copied.p.x, copied.values[1], s.p.x, a[0], a[1], s.p.y
}
```

## aggregate_arguments_returns_and_parallel_assignment_preserve_snapshots
<!-- ond-spec-id: conformance/regression/aggregate-arguments-returns-and-parallel-assignment-preserve-snapshots -->

要件: `AGG-03`。\
操作・期待: MemoryVm: main.run([]) -> [99,2,1,2]

入力1・`main.ond`:
```ond
package main
func main() {}
func change(a: [2]i32) -> [2]i32 { a[0] = 99; return a; }
func tick(p: *i32) -> i32 { *p = *p + 1; return *p; }
func run() -> (i32, i32, i32, i32) {
    var counter: i32
    var a = [2]i32{1: tick(&counter), 0: tick(&counter)}
    var b = change(a)
    a, b = b, a
    return a[0], b[0], b[1], counter
}
```

## string_literals_are_mutable_byte_array_values_and_copy_independently
<!-- ond-spec-id: conformance/regression/string-literals-are-mutable-byte-array-values-and-copy-independently -->

要件: `STR-02`。\
操作・期待: MemoryVm: main.run([]) -> [5,233,42,159,0]

入力1・`main.ond`:
```ond
package main
const Text = "音\xFF\000"
const Size = len(Text)
const First = Text[0]
func main() {}
func run() -> (u32, u8, u8, u8, u32) {
    var text = Text
    var copy = text
    text[3] = 42
    text[1] = 0
    var empty: [0]u8 = ""
    return Size, First, text[3], copy[1], len(empty)
}
```

## arrays_including_string_literals_are_bounds_checked_but_raw_pointers_are_not
<!-- ond-spec-id: conformance/regression/arrays-including-string-literals-are-bounds-checked-but-raw-pointers-are-not -->

要件: `BOUND-01, MEM-04`。\
操作・期待: MemoryVm: main.array([1]) -> [0]。main.array([2])、main.text([2])、main.array([18446744073709551615])、main.text([18446744073709551615])はbounds fault。各呼出しはfresh状態。rawのguard形状は実装追跡だけに含める

入力1・`main.ond`:
```ond
package main
func main() {}
func array(i: i32) -> u8 { var a: [2]u8; return a[i]; }
func text(i: i32) -> u8 { return "ab"[i]; }
func raw(p: *u8, i: u32) -> u8 { return p[i]; }
```

## array_len_preserves_one_operand_side_effect_and_runtime_faults
<!-- ond-spec-id: conformance/regression/array-len-preserves-one-operand-side-effect-and-runtime-faults -->

要件: `LEN-01`。\
操作・期待: MemoryVm: main.result([0]) -> [2,1]、fresh状態のmain.result([1]) -> bounds fault

入力1・`main.ond`:
```ond
package main
func main() {}
func array(calls: *u32, index: u32) -> [2]u8 {
    *calls += 1
    var values = [1]u8{}
    _ = values[index]
    return [2]u8{};
}
func result(index: u32) -> (u32, u32) {
    var calls: u32
    var size = len(array(&calls, index))
    return size, calls;
}
```

## runtime_elements_keep_source_order_and_holes_are_zero
<!-- ond-spec-id: conformance/regression/runtime-elements-keep-source-order-and-holes-are-zero -->

要件: `AGG-01`。\
操作・期待: MemoryVm: main.result([]) -> [123]

入力1・`main.ond`:
```ond
package main
var count: i32
func main() {}
func next() -> i32 { count = count + 1; return count; }
func result() -> i32 { var a = [...]i32{3: next(), 0: next(), next()}; return a[3] * 100 + a[0] * 10 + a[1] + a[2]; }
```

## multiple_results_are_evaluated_once_and_forwarded
<!-- ond-spec-id: conformance/regression/multiple-results-are-evaluated-once-and-forwarded -->

要件: `CALL-01`。\
操作・期待: MemoryVm: main.run([]) -> [11,12]

入力1・`main.ond`:
```ond
package main
func main() {}
func pair(x: i32) -> (i32, i32) { return x, x + 1; }
func forward(x: i32) -> (i32, i32) { return pair(x); }
func run() -> (i32, i32) {
    var a, b = pair(3)
    a, b = b, a
    a, c := forward(a)
    var d, e: i32 = pair(c)
    a, b = pair(d + e)
    return a, b
}
```

## statement_calls_discard_zero_one_or_many_results
<!-- ond-spec-id: conformance/regression/statement-calls-discard-zero-one-or-many-results -->

要件: `CALL-01`。\
操作・期待: MemoryVm: main.main([]) -> []

入力1・`main.ond`:
```ond
package main
func nothing() {}
func one() -> i32 { return 1; }
func pair() -> (i32, bool) { return 1, true; }
func main() { nothing()
    one()
    pair()
}
```

## calls_support_imports_forward_references_and_mutual_recursion
<!-- ond-spec-id: conformance/regression/calls-support-imports-forward-references-and-mutual-recursion -->

要件: `CALL-02`。\
操作・期待: MemoryVm: main.run([]) -> [8]、main.even([4]) -> [1]、main.even([3]) -> [0]。各呼出しはfresh状態

入力1・`main.ond`:
```ond
package main
import "test/shared"
func main() {}
func run() -> i32 { return shared.Double(later(3)); }
func later(x: i32) -> i32 { return x + 1; }
func even(x: i32) -> bool {
    if x == 0 { return true; }
    return odd(x - 1)
}
func odd(x: i32) -> bool {
    if x == 0 { return false; }
    return even(x - 1)
}
```

入力1・`shared/shared.ond`:
```ond
package shared
func Double(x: i32) -> i32 { return x * 2; }
```

## indirect_calls_evaluate_callee_then_arguments_and_keep_short_circuit
<!-- ond-spec-id: conformance/regression/indirect-calls-evaluate-callee-then-arguments-and-keep-short-circuit -->

要件: `CALL-02`。\
操作・期待: MemoryVm: main.run([]) -> [7,9]

入力1・`main.ond`:
```ond
package main
func main() {}
func pair(a: i32, b: i32) -> (i32, i32) { return a, b; }
func choose() -> func(a: i32, b: i32) -> (i32, i32) { return pair; }
func left() -> i32 { return 7; }
func right() -> i32 { return 9; }
func yes() -> bool { return true; }
func run() -> (i32, i32) {
    var skipped = false && yes()
    return choose()(left(), right())
}
func through(f: func(a: i32, b: i32) -> (i32, i32)) -> (i32, i32) { return f(1, 2); }
func local() -> (i32, i32) {
    var f = pair
    return through(f)
}
```

## globals_are_zeroed_or_initialized_from_compile_time_values
<!-- ond-spec-id: conformance/regression/globals-are-zeroed-or-initialized-from-compile-time-values -->

要件: `INIT-02`。\
操作・期待: MemoryVm: main.result([]) -> [0,9,4,5,3]。内部initializerの有無は比較しない

入力1・`main.ond`:
```ond
package main
type Pair struct { first: i32; second: i32; }
var Zero: i32
var Number = 9
var Values = Pair{first: 4, second: 5}
var Text = "abc"
func main() {}
func result() -> (i32, i32, i32, i32, u32) {
    return Zero, Number, Values.first, Values.second, len(Text)
}
```

## constants_resolve_across_packages_and_lexical_scopes
<!-- ond-spec-id: conformance/regression/constants-resolve-across-packages-and-lexical-scopes -->

要件: `PKG-04`。\
操作・期待: Core: 受理、Width:i32=6、Bufferのunderlying型=[12]u8、shared.Globalとshared.Functionを公開宣言として解決。MemoryVm: main.value([]) -> [18]

入力1・`main.ond`:
```ond
package main
import "test/shared"
const Width = shared.Base + 2
type Buffer [Width * 2]u8
func main() {}
func value() -> i32 {
const x = Width * 3
{ const x = 1
var y = x
}
return x
}
```

入力1・`shared/shared.ond`:
```ond
package shared
const Base = 4
var Global: i32
func Function() {}
```

## recursive_pointer_elements_are_checked_after_resolution
<!-- ond-spec-id: conformance/regression/recursive-pointer-elements-are-checked-after-resolution -->

要件: `TYPE-04`。\
操作・期待: 入力1: Core受理。入力2: Error1件、contains array layout overflow、最後の[1073741824]Node。入力3: Error1件、contains direct recursive type、最後のCycle [1]Cycle

入力1・`main.ond`:
```ond
package main
type Node struct { children: *[1073741823]Node }
func main() {}
```

入力2・`main.ond`:
```ond
package main
type Node struct { children: *[1073741824]Node }
func main() {}
```

入力3・`main.ond`:
```ond
package main
type Cycle [1]Cycle
func main() {}
```

## invalid_blank_uses_and_short_declarations_are_rejected
<!-- ond-spec-id: conformance/regression/invalid-blank-uses-and-short-declarations-are-rejected -->

要件: `DECL-04`。\
操作はCore検査であり、表の各行を別projectとして拒否する。\
診断messageはcontainsで比較し、位置の記載がある行は主位置を表の断片の最後の出現範囲と照合する。\
位置と個数の記載がない行は、対応するsourceのError診断が1件以上あることを照合する。

入力template1（波括弧内のparameterだけ表の文字列へ置換し、それ以外はliteralとして読む）:
```ond
package main
func main() { {body} }
```

| Parameter列 | message比較・位置 |
| --- | --- |
| <code>_ := 1</code> | <code>Error1件; contains short declaration requires at least one new name; span=_ := 1</code> |
| <code>_, _ := 1, 2</code> | <code>Error1件; contains short declaration requires at least one new name; span=_, _ := 1, 2</code> |
| <code>a := 1; a, _ := 2, 3</code> | <code>Error1件; contains short declaration requires at least one new name; span=a, _ := 2, 3</code> |
| <code>a := 1; a, a, b := 2, 3, 4</code> | <code>Error1件; contains duplicate name in short declaration; span=a</code> |
| <code>a, a := 1, 2</code> | <code>Error1件; contains duplicate name in short declaration; span=a</code> |
| <code>var _ = 1; var a = _</code> | <code>Error1件; contains blank identifier `_` cannot be used as a value; span=_</code> |
| <code>const _ = 1; var a = _</code> | <code>Error1件; contains blank identifier `_` cannot be used as a value; span=_</code> |
| <code>var _: u8 = 256</code> | <code>Error1件; contains integer literal out of range; span=256</code> |
| <code>const _: i32 = missing()</code> | <code>Error1件; contains constant initializer requires a compile-time value; span=missing()</code> |
| <code>_ = nil</code> | <code>Error1件; contains requires an explicit pointer or function type context; span=nil</code> |
| <code>_, _ = 1</code> | <code>Error1件; contains assignment value count mismatch; span=_, _ = 1</code> |
| <code>var _, _ = 1</code> | <code>Error1件; contains declaration value count mismatch; span=_, _ = 1</code> |
| <code>const _, _ = 1</code> | <code>Error1件; contains constant declaration value count mismatch; span=_, _ = 1</code> |
| <code>const a = 1; a, b := 2, 3</code> | <code>Error1件; contains cannot assign to a constant; span=a, b := 2, 3</code> |

入力template2（波括弧内のparameterだけ表の文字列へ置換し、それ以外はliteralとして読む）:
```ond
package main
{declaration}
func main() {}
```

| Parameter列 | message比較・位置 |
| --- | --- |
| <code>var _: u8 = 256</code> | <code>Error1件; contains integer literal out of range; span=256</code> |
| <code>const _ = missing()</code> | <code>Error1件; contains constant initializer requires a compile-time value; span=missing()</code> |
| <code>const _, _ = 1</code> | <code>Error1件; contains constant declaration value count mismatch; span=_, _ = 1</code> |
| <code>var _, _ = 1</code> | <code>Error1件; contains declaration value count mismatch; span=_, _ = 1</code> |

追加入力`main.ond`は`package main\nfunc f(_: i32) -> i32 { return _ }\nfunc main() {}\n`（`\n`はLF）とし、Error1件、contains `blank identifier`、主位置は最後の`_`とする。

## non_call_expression_statements_and_missing_separators_are_rejected
<!-- ond-spec-id: conformance/regression/non-call-expression-statements-and-missing-separators-are-rejected -->

要件: `SYN-09, SYN-10`。\
操作はCore検査であり、表の各行を別projectとして拒否する。\
診断messageはcontainsで比較し、位置の記載がある行は主位置を表の断片の最後の出現範囲と照合する。\
位置と個数の記載がない行は、対応するsourceのError診断が1件以上あることを照合する。

入力template1（波括弧内のparameterだけ表の文字列へ置換し、それ以外はliteralとして読む）:
```ond
package main
func f() -> i32 { return 1 }
func main() { {body} }
```

| Parameter列 | message比較・位置 |
| --- | --- |
| <code>1 + 2</code> | <code>Error1件; contains expression statement must be a function call; span=1 + 2</code> |
| <code>true</code> | <code>Error1件; contains expression statement must be a function call; span=true</code> |
| <code>new(i32)</code> | <code>Error1件; contains expression statement must be a function call; span=new(i32)</code> |
| <code>f() + 1</code> | <code>Error1件; contains expression statement must be a function call; span=f() + 1</code> |
| <code>f() as i32</code> | <code>Error1件; contains expression statement must be a function call; span=f() as i32</code> |
| <code>if 1; true {}</code> | <code>Error1件; contains expected simple statement; span=1</code> |
| <code>for 1; true; f() {}</code> | <code>Error1件; contains expected simple statement; span=1</code> |
| <code>for ; true; 1 {}</code> | <code>Error1件; contains expression statement must be a function call; span=1</code> |
| <code>var a = 1 var b = 2</code> | <code>Error1件; contains expected `;` or newline; span=var</code> |
| <code>f() f()</code> | <code>Error1件; contains expected `;` or newline; span=f</code> |
| <code>for var a = 1; true; f() {}</code> | <code>Error1件; contains expected expression; span=var</code> |
| <code>f(,)</code> | <code>Error1件; contains expected expression; span=,</code> |
| <code>f(1,,)</code> | <code>Error1件; contains expected expression; span=,</code> |

## field_order_names_and_nominal_element_types_remain_significant
<!-- ond-spec-id: conformance/regression/field-order-names-and-nominal-element-types-remain-significant -->

要件: `TYPE-03`。\
操作はCore検査であり、表の各行を別projectとして拒否する。\
診断messageはcontainsで比較し、位置の記載がある行は主位置を表の断片の最後の出現範囲と照合する。\
位置と個数の記載がない行は、対応するsourceのError診断が1件以上あることを照合する。

入力template1（波括弧内のparameterだけ表の文字列へ置換し、それ以外はliteralとして読む）:
```ond
package main
type A i32
type B i32
func main() { var a: {left}; var b: {right} = a; }
```

| Parameter列 | message比較・位置 |
| --- | --- |
| <code>left=struct { x: i32; y: u8; }; right=struct { y: u8; x: i32; }</code> | <code>Error1件; contains type mismatch; span=最後のa</code> |
| <code>left=struct { x: i32; }; right=struct { y: i32; }</code> | <code>Error1件; contains type mismatch; span=最後のa</code> |
| <code>left=struct { x: i32; }; right=struct { x: u32; }</code> | <code>Error1件; contains type mismatch; span=最後のa</code> |
| <code>left=[2]u8; right=[3]u8</code> | <code>Error1件; contains type mismatch; span=最後のa</code> |
| <code>left=*A; right=*B</code> | <code>Error1件; contains type mismatch; span=最後のa</code> |
| <code>left=[2]A; right=[2]B</code> | <code>Error1件; contains type mismatch; span=最後のa</code> |
| <code>left=struct { x: A; }; right=struct { x: B; }</code> | <code>Error1件; contains type mismatch; span=最後のa</code> |
| <code>left=func(x: A); right=func(x: B)</code> | <code>Error1件; contains type mismatch; span=最後のa</code> |

## public_fields_match_across_packages_and_private_fields_do_not
<!-- ond-spec-id: conformance/regression/public-fields-match-across-packages-and-private-fields-do-not -->

要件: `TYPE-01`。\
操作はCore検査であり、表の各行を別projectとして拒否する。\
診断messageはcontainsで比較し、位置の記載がある行は主位置を表の断片の最後の出現範囲と照合する。\
位置と個数の記載がない行は、対応するsourceのError診断が1件以上あることを照合する。

入力template1（波括弧内のparameterだけ表の文字列へ置換し、それ以外はliteralとして読む）:
```ond
package main
import "test/shared"
func main() { {body}; }
```

| Parameter列 | message比較・位置 |
| --- | --- |
| <code>var v: struct { x: i32; } = shared.Private()</code> | <code>Error1件; contains declaration value 1 type mismatch; span=v: struct { x: i32; } = shared.Private()</code> |
| <code>var v = shared.Private(); var x = v.x</code> | <code>Error1件; contains field `x` is not exported; span=v.x</code> |
| <code>var v = shared.Private(); var x = v as struct { x: i32; }</code> | <code>Error1件; contains invalid explicit type conversion; span=v</code> |

全行で`shared/shared.ond`:
```ond
package shared
func Public() -> struct { X: i32; } { var v: struct { X: i32; }; v.X = 7; return v; }
func Private() -> struct { x: i32; } { var v: struct { x: i32; }; return v; }
```

受理側の`main.ond`は次のsourceとし、MemoryVmで`main.result([]) -> [7]`を照合する。
```ond
package main
import "test/shared"
func main() {}
func result() -> i32 { var v: struct { X: i32; } = shared.Public(); return v.X; }
```

## invalid_inference_is_a_source_error
<!-- ond-spec-id: conformance/regression/invalid-inference-is-a-source-error -->

要件: `AGG-01`。\
操作はCore検査であり、表の各行を別projectとして拒否する。\
診断messageはcontainsで比較し、位置の記載がある行は主位置を表の断片の最後の出現範囲と照合する。\
位置と個数の記載がない行は、対応するsourceのError診断が1件以上あることを照合する。

入力template1（波括弧内のparameterだけ表の文字列へ置換し、それ以外はliteralとして読む）:
```ond
package main
func main() { {source}; }
```

| Parameter列 | message比較・位置 |
| --- | --- |
| <code>var x = [...]u8{}</code> | <code>Error1件; contains cannot infer array length from an empty literal; span=[...]u8{}</code> |
| <code>var x = [...]u8{-1: 0}</code> | <code>Error1件; contains array literal index out of bounds; span=-1: 0</code> |
| <code>var x = [...]u8{4294967295 as u32: 0}</code> | <code>Error1件; contains array literal index out of bounds; span=4294967295 as u32: 0</code> |
| <code>var x = [...]u8{4294967294 as u32: 0, 1}</code> | <code>Error1件; contains array literal index out of bounds; span=1</code> |
| <code>var x = [...]u8{2: 0, 2: 1}</code> | <code>Error1件; contains duplicate field or index; span=2: 1</code> |
| <code>var i = 2; var x = [...]u8{i: 0}</code> | <code>Error1件; contains array literal key must be a compile-time integer; span=i: 0</code> |
| <code>var x = [...]u8{true: 0}</code> | <code>Error1件; contains index must be an integer constant; span=true</code> |
| <code>var x = [...]u8{256}</code> | <code>Error1件; contains integer literal out of range; span=256</code> |
| <code>var x: [2]u8 = [...]u8{1}</code> | <code>Error1件; contains type mismatch; span=[...]u8{1}</code> |
| <code>var x: [...]u8</code> | <code>Error1件; contains inferred array length requires a composite literal; span=[...]u8</code> |
| <code>var x = new([...]u8)</code> | <code>Error1件; contains inferred array length requires a composite literal; span=[...]u8</code> |
| <code>var x = [...][...]u8{[...]u8{1}}</code> | <code>Error1件; contains inferred array length requires a composite literal; span=最初の[...]u8</code> |
| <code>var x = [. . .]u8{1}</code> | <code>Error1件; contains expected expression; span=最初の.</code> |
| <code>var x = [....]u8{1}</code> | <code>Error1件; contains expected `]`; span=.</code> |

追加の2入力は次の`main.ond`全文とし、各々Error1件、contains `inferred`、最後の`[...]u8`範囲を期待する。
```ond
package main
type T [...]u8
func main() {}
```
```ond
package main
func f(a: [...]u8) {}
func main() {}
```

## string_literal_lengths_and_pointer_types_require_exact_matches
<!-- ond-spec-id: conformance/regression/string-literal-lengths-and-pointer-types-require-exact-matches -->

要件: `STR-02`。\
操作はCore検査であり、表の各行を別projectとして拒否する。\
診断messageはcontainsで比較し、位置の記載がある行は主位置を表の断片の最後の出現範囲と照合する。\
位置と個数の記載がない行は、対応するsourceのError診断が1件以上あることを照合する。

入力template1（波括弧内のparameterだけ表の文字列へ置換し、それ以外はliteralとして読む）:
```ond
package main
type S struct { byte: u8; }
func main() {
{body}
}
```

| Parameter列 | message比較・位置 |
| --- | --- |
| <code>var x: [4]u8 = &quot;abc&quot;</code> | <code>contains exactly match</code> |
| <code>var x: [2]u8 = &quot;abc&quot;</code> | <code>contains exactly match</code> |
| <code>var x: [3]i8 = &quot;abc&quot;</code> | <code>contains exactly match</code> |
| <code>var x = &quot;abc&quot;<br>var p: *u8 = x</code> | <code>contains type mismatch</code> |
| <code>var values: [1]S<br>var p: *u8 = &amp;values[0]</code> | <code>contains type mismatch</code> |

受理側の追加入力は次の`main.ond`であり、Core検査の成功を期待する。

```ond
package main
type S struct { byte: u8; }
func main() { var text = "abc"; var head: *u8 = &text[0]; var values: [1]S; var first: *S = &values[0]; _ = head; _ = first; }
```

## memory_errors_have_source_locations
<!-- ond-spec-id: conformance/regression/memory-errors-have-source-locations -->

要件: `AGG-02`。\
操作はCore検査であり、表の各行を別projectとして拒否する。\
診断messageはcontainsで比較し、位置の記載がある行は主位置を表の断片の最後の出現範囲と照合する。\
位置と個数の記載がない行は、対応するsourceのError診断が1件以上あることを照合する。\
各行の主位置はmain.ondのsourceに対応するUTF-8 byte範囲とし、file範囲内で両端が文字境界にあることを比較する。

入力template1（波括弧内のparameterだけ表の文字列へ置換し、それ以外はliteralとして読む）:
```ond
package main
type Point struct { x: i32; }
func main() {
{body}
}
```

| Parameter列 | message比較・位置 |
| --- | --- |
| <code>var a: [2]u8<br>var x = a[-1]</code> | <code>contains out of bounds</code> |
| <code>var x = &quot;ab&quot;[2]</code> | <code>contains out of bounds</code> |
| <code>var a = [2]u8{2: 1}</code> | <code>contains out of bounds</code> |
| <code>var a = [2]u8{0: 1, 0: 2}</code> | <code>contains duplicate</code> |
| <code>var p = Point{x: 1, x: 2}</code> | <code>contains duplicate</code> |
| <code>var p = Point{1}</code> | <code>contains struct literal requires named fields</code> |
| <code>var p = Point{missing: 1}</code> | <code>contains unknown struct field</code> |
| <code>var p: Point<br>p.missing = 1</code> | <code>contains unknown struct field</code> |
| <code>var p = &amp;Point{x: 1}</code> | <code>contains not supported</code> |
| <code>var x = *1</code> | <code>contains requires a pointer</code> |
| <code>var p: *u8<br>var i: i32<br>var x = p[i]</code> | <code>contains type mismatch</code> |
| <code>var x = len(1)</code> | <code>contains requires an array</code> |
| <code>var x = &quot;\q&quot;</code> | <code>contains invalid string escape</code> |

## executes_cfg_scopes_loops_and_short_circuit
<!-- ond-spec-id: conformance/regression/executes-cfg-scopes-loops-and-short-circuit -->

要件は`FLOW-01`であり、`main.ond`は次の全sourceとする。\
各操作はMemoryVm phaseのfresh状態で行う。
```ond
package main
func main() {}
func shortAnd(x: i32) -> bool { return x != 0 && 10 / x > 1
}
func shortOr(x: i32) -> bool { return x == 0 || 10 / x > 1
}
func nested(x: i32) -> bool { return (x == 0 || 10 / x > 1) && (x == 0 || x < 5)
}
func loops() -> i32 {
    sum := 0
    for i := 0; i < 5; i = i + 1 {
        if i == 1 { continue
}
        for j := 0; j < 4; j = j + 1 {
            if j == 2 { break
}
            sum = sum + i
       
}
   
}
    for sum < 22 { sum = sum + 1
}
    for { break
}
    return sum
}
func scopes() -> i32 {
    x := 1
    { x := x + 2
      x = x + 4
   
}
    if x := x + 10; x == 11 { x = x + 1
} else { x = 0
}
    for x := 0; x < 2; x = x + 1 {
}
    return x
}
func branches(x: i32) -> i32 {
    if x == 0 { return 7
} else if x == 1 { return 8
} else { return 9
}
}
func swap(x: i32) -> i32 {
    a := false
    b := true
    a, b = b || x == 0, a && 10 / x > 1
    if a && b { return 1
}
    return 2
}
```

| 呼出し | 期待返値 |
| --- | --- |
| main.shortAnd([0]) | [0] |
| main.shortOr([0]) | [1] |
| main.shortAnd([2]) | [1] |
| main.shortOr([10]) | [0] |
| main.nested([0]) | [1] |
| main.nested([5]) | [0] |
| main.loops([]) | [22] |
| main.scopes([]) | [1] |
| main.branches([0]) | [7] |
| main.branches([1]) | [8] |
| main.branches([2]) | [9] |
| main.swap([0]) | [2] |

## runtime_global_initializers_are_rejected
<!-- ond-spec-id: conformance/regression/runtime-global-initializers-are-rejected -->

要件は`INIT-02`である。\
次のtemplateの`DECLARATION`を各行で置換してCore検査を行い、compile errorを期待する。\
Error診断の少なくとも1件が`compile-time value`または`value count mismatch`を含むことを照合する。
```ond
package main
DECLARATION
func make() -> i32 { return 1; }
func pair() -> (i32, i32) { return 1, 2; }
func main() {}
```

| DECLARATION |
| --- |
| <code>var Value = make()</code> |
| <code>var A = 1<br>var B = A</code> |
| <code>var Value = new(i32)</code> |
| <code>var A, B = pair()</code> |

## reports_cycles_and_private_imports
<!-- ond-spec-id: conformance/regression/reports-cycles-and-private-imports -->

要件は`PKG-04`である。\
次の各入力は独立projectのCore検査でcompile errorを期待する。\
少なくとも1件のError診断が表のmessageを含むことを比較する。

| main.ond全文 | shared/shared.ond全文 | message contains |
| --- | --- | --- |
| <code>package main<br>const N = -1<br>type Buffer [N]u8<br>func main() {}<br></code> | fileなし | array length |
| <code>package main<br>const A = B<br>const B = A<br>func main() {}<br></code> | fileなし | cycle |
| <code>package main<br>import &quot;test/shared&quot;<br>const X = shared.private<br>func main() {}<br></code> | <code>package shared<br>const private = 1<br></code> | not exported |

## length_is_maximum_index_plus_one_not_last_index
<!-- ond-spec-id: conformance/regression/length-is-maximum-index-plus-one-not-last-index -->

要件は`AGG-01`である。\
各行のEを`package main`、`const A = E`、`func main() {}`の3行（LF区切り・末尾LFあり）へ展開し、CoreでAの型とlengthを比較する。

| E | Aの型 |
| --- | --- |
| `[...]u8{10, 20, 30}` | [3]u8 |
| `[...]u8{4: 10, 20}` | [6]u8 |
| `[...]u8{8: 1, 2: 2, 3}` | [9]u8 |
| `[...]u8{(1 << 3): 7}` | [9]u8 |
| `[...]u8{4294967294 as u32: 7}` | [4294967295]u8 |

## compile_project_does_not_predeclare_asset
<!-- ond-spec-id: conformance/regression/compile-project-does-not-predeclare-asset -->

要件は`OUT-02`である。\
次の`main.ond`をCore検査し、未定義名`asset`のcompile errorを期待する。
```ond
package main

func main() {
    asset(0 as u32)
}
```
