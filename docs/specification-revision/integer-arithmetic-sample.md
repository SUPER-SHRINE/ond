# 整数演算と定数評価の改稿見本

この文書は決裁時に用いた非規範の改稿見本である。\
規範定義元は[移行後の式章](../../specs/ond/language/v1/expressions.md)に置く。\
編集原則とソース集約方針は[仕様書の編集方針](./editorial-policy.md)、決裁記録は[未決定事項一覧](./open-questions.md)、配置候補は[目次・移行表](./migration-map.md)を参照する。

根拠資料はOnd repository commit `06e87c39d99f2389af163d0394d9808f2c3078bd`の`specs/ond/v1/language.md`、`syntax.md`、`builtins.md`およびKagura target文書である。\
適用範囲は組み込み整数型の式、定数式、変換と、ここで参照する配列長およびlayout builtinである。\
各規則の出典は末尾の編集用トレーサビリティ表に示す。

## 適用範囲

Ond v1の組み込み整数型は`u8`、`u16`、`u32`、`i8`、`i16`、`i32`である。\
整数値、型同一性、整数演算、整数定数演算、変換、整数値を返すbuiltinの成立条件と結果をこの節で記す。

整数演算はKagura targetを含む各適用targetで、言語規則と明示されたtarget委譲の組合せに従う。\
f32の演算はIEEE 754 binary32規則、user-defined operatorはその宣言・解決規則、raw pointerのindex/address演算はtarget pointer規則をそれぞれ定義元とする。

## 構文

構文の定義元は[Syntax v1](../../specs/ond/language/v1/lexical.md)であり、演算子の優先順位と文法は[式章](../../specs/ond/language/v1/expressions.md)を参照する。\
この見本では既存EBNFを参照する。

整数単項演算子は`+`、`-`、`^`である。\
整数二項演算子は`+`、`-`、`*`、`/`、`%`、`&`、`|`、`^`、`&^`、`<<`、`>>`である。\
比較演算子は`==`、`!=`、`<`、`<=`、`>`、`>=`であり、明示変換は`value as T`の形式である。

shift以外のbinary operandは同じ整数型を使い、整数算術・bitwise結果はoperandと同じ型を使う。\
shift countは`u32`を使い、count位置の整数literalは`u32`として扱う。

整数演算のoperand/result型は演算群ごとに次の対応で定める。

| 演算群 | operand型 | result型 |
| --- | --- | --- |
| 単項整数演算`+`, `-`, `^` | `T` | `T` |
| 整数算術・bitwise二項演算 | `T`, `T` | `T` |
| shift | 左辺`T`、count`u32` | `T` |
| 比較 | 同一整数型`T`, `T` | `bool` |

## 静的制約

整数literalの既定型は`i32`である。\
型文脈が整数literalの型を定める場合、その型で表現できる値を持つliteralが成立する。

各defined typeはその宣言に固有の型同一性を持つ。\
assignment、argument、returnでは要求型との一致を成立条件とし、整数型間の変換は明示的な`as`で表す。

定数評価の中間値は数学的整数として扱い、source整数型6種と評価用数学整数の役割を分ける。\
宣言・代入・argument・return・明示cast等の値要求境界で、要求整数型の表現範囲を検査する。\
配列length、index、shift countは各専用の成立条件で検査する。\
この範囲検査と数学的中間値の区分は[SPEC-Q02](./open-questions.md#spec-q02--定数式の型文脈と範囲検査時点)で決裁された規則である。

評価される定数`as`変換は変換先の表現範囲を検査し、範囲外の値にcompile errorを返す。\
runtime整数変換は変換先幅の下位bitとsource符号に基づく拡張で結果を定める。

定数値の集合は整数、`f32`、`bool`、string literalが生成するbyte配列、全要素が定数である配列・struct、型が明示された`nil`である。\
defined typeはunderlying typeの同じ定数値規則に従い、定数のfield/index参照はcompile-timeに評価する。\
定数式はこれらの定数値と現行仕様で定める演算・変換を使って構成する。\
通常callを含むconst初期値の検査結果はcompile errorであり、短絡経路にあるcallも同じconst適格性検査の対象となる。\
短絡経路ごとの名前解決と型検査は両辺に行い、値評価および定数castの範囲検査は値評価対象となる経路で行う。

整数定数式は数学的な正確値で計算する。\
評価値はsource整数型の幅およびcompilerの中間整数幅と独立して保持する。

## 意味

### 実行時演算

幅`N` bitの整数runtime演算は、現行仕様が参照する[ISO/IEC 10967-1:2012](https://www.iso.org/standard/51317.html)のmodulo integer modelに従う。\
単項`+`、`-`、`^`の結果型はoperandと同じ整数型である。\
二項算術・bitwiseの結果型もoperandと同じ整数型である。

`+`、`-`、`*`および単項`-`は、数学上の結果を`2^N` moduloでwrapする。\
`^`、`&`、`|`、`&^`は`N` bit表現上で計算する。\
signed comparisonはtwo's-complement値を比較し、unsigned comparisonは符号なし値を比較する。

短絡演算を除くoperandと関数argumentはsource順に左から右へ評価する。\
`&&`と`||`は左operandを先に評価し、結果は編集方針の短絡表に示す左辺値と評価経路で定まる。

整数`/`は商を0方向へ丸める。\
整数`%`は`a == (a / b) * b + (a % b)`を満たす剰余を返し、非zero剰余の符号は被除数と同じである。

```ond
func AddU8(a: u8, b: u8) -> u8 { return a + b }
func AddI8(a: i8, b: i8) -> i8 { return a + b }
func DivideI8(a: i8, b: i8) -> i8 { return a / b }
func RemainderI8(a: i8, b: i8) -> i8 { return a % b }
```

| 関数呼び出しと入力 | 期待値 |
| --- | --- |
| `AddU8(255, 1)` | `u8`値`0` |
| `AddI8(127, 1)` | `i8`値`-128` |
| `DivideI8(-7, 3)` | `i8`値`-2` |
| `RemainderI8(-7, 3)` | `i8`値`-1` |
| `RemainderI8(-128, -1)` | `i8`値`0` |

shiftの左operandと結果は同じ整数型を使う。\
幅`N`とcount`k`から実効count `k mod N`を求める。\
runtime left shiftは上位bitを捨ててwrapする。\
unsigned right shiftは0を補い、signed right shiftはoperandの最上位bitを補う。\
runtime shiftの参照規範は[WebAssembly integer semantics](https://webassembly.github.io/spec/core/exec/numerics.html)である。

### 定数演算

整数定数式は数学的な正確値で計算する。\
要求される整数型が定まると、その型の範囲を検査し、範囲外の結果にcompile errorを返す。

| Ond source | 期待結果 |
| --- | --- |
| `const A: u8 = 1 << 8` | 受理、実効countは`0`、値`1` |
| `const B: u8 = 128 << 1` | compile error、正確値`256`は`u8`範囲外 |
| `const C: u8 = 250 + 10` | compile error、正確値`260`は`u8`範囲外 |

| Ond source | 決裁済み期待結果 |
| --- | --- |
| `const X: u8 = (250 + 10) - 10` | 受理、`u8`値`250` |
| `const Y: u8 = (250 + 10) as u8` | compile error、cast時の正確値`260`は`u8`範囲外 |
| `const Z: u8 = 250 + 10` | compile error、要求値`260`は`u8`範囲外 |

定数shift countは`u32`であり、幅`N`に対する実効値は`k mod N`である。\
定数left shiftは実効countに従って数学的に左移動し、結果は要求型の範囲で検査する。\
right shiftの符号規則はruntime演算と同じである。

const式と通常値式の`&&`・`||`は、同じ短絡評価経路で結果を定める。\
値評価対象の辺だけを値評価し、両辺の名前解決と型検査を行う。\
定数castの範囲検査は値評価対象となる経路で行う。\
定数式の構成適格性は式全体について検査し、通常callを含むconst初期値の検査結果はcompile errorとなる。

| 演算子 | 左辺値 | 値評価対象 | 結果 |
| --- | --- | --- | --- |
| `&&` | `false` | 左辺 | `false` |
| `&&` | `true` | 左辺、右辺 | 右辺値 |
| `||` | `true` | 左辺 | `true` |
| `||` | `false` | 左辺、右辺 | 右辺値 |

```ond
const C: bool = false && ((250 + 6) as u8 == 0)
const D: bool = true || ((250 + 6) as u8 == 0)
const E: bool = false && missingName
const Safe: bool = false && (1 / 0 == 0)
```

| 式 | 期待結果 |
| --- | --- |
| `C` | 受理、`false`。値評価対象は短絡表の`&&`・左辺`false`の経路 |
| `D` | 受理、`true`。値評価対象は短絡表の`||`・左辺`true`の経路 |
| `E` | compile error、右辺の名前解決で未定義名を検出 |
| `Safe` | 受理、`false`。値評価対象は短絡表の`&&`・左辺`false`の経路 |
| `false && f()`を含むconst式 | compile error、式全体の定数適格性検査でcallを検出 |

`C`と`D`は[Grammar Notes](../../specs/ond/language/v1/declarations.md#構文と静的条件の対応)（`ond-spec-id: syntax/grammar-notes`）に明示された受理例である。\
比較演算のresult型は上記の対応表どおり`bool`である。

### 整数変換

定数整数変換は正確値が変換先範囲に入ることを成立条件とする。\
runtime整数変換は変換先幅の下位bitを保持し、source符号に応じて符号拡張またはzero拡張する。

| Ond source | 期待結果 |
| --- | --- |
| `const A: u8 = 255 as u8` | 受理、`u8`値`255` |
| `const B: u8 = 256 as u8` | compile error、定数cast範囲外 |
| `const C: u8 = (250 + 10) as u8` | compile error、正確値`260`はcast先範囲外 |

### 配列長とlayout builtin

`len(array)`はoperandを通常の評価規則に従って1回評価し、配列長をcompile-time constant `u32`で返す。\
文字列literalは`[N]u8`であり、`len("日本語")`はbyte数の`9`を返す。\
operand中の定数式は通常の短絡、型検査、cast範囲検査に従う。\
operandにruntime callが含まれる式は定数値として確定せず、const値を要求する箇所でcompile errorとなる。

配列以外の`len`は一致する`operator len`を使い、結果型は`u32`である。\
`sizeof(T)`と`alignof(T)`はlayout値を`u32`で返すbuiltin構文であり、Kagura v1ではmachine文書が定める配列byte size上限を検査する。

## 異常時

| 条件 | 結果 |
| --- | --- |
| 定数評価で除数がzero | compile error |
| 評価対象となる整数`/`または`%`の除数がzero | compile-timeはcompile error、runtimeはfault |
| 定数`minInt / -1` | compile error |
| runtime signed `minInt / -1` | fault |
| `minInt % -1` | 整数値`0` |
| 評価対象の定数castが変換先範囲外 | compile error |
| 定数式全体に通常function callを含む | compile error |

```ond
const BadDivision: i8 = -128 / -1

func DivideByZero() -> u8 {
    var zero: u8 = 0
    return 1 / zero
}

func SignedOverflow() -> i8 {
    var min: i8 = -128
    var negativeOne: i8 = -1
    return min / negativeOne
}
```

`BadDivision`の検査結果はcompile errorであり、`DivideByZero`と`SignedOverflow`の実行結果はfaultである。\
これらは定数式とruntime式それぞれの明示された異常結果を示す。

## 委譲・最適化

整数定数式の結果は最適化有無にかかわらず正確値規則で定まる。\
runtime式を定数畳み込みする最適化は、元のruntime式が持つwrapとfaultの観測結果を保つ。

f32演算、user-defined operator、raw pointer演算の成立条件と結果は、各定義元で規範化する。\
Kagura v1のlayoutは[Machine Profile v1](../../specs/ond/targets/kagura-v1/machine.md)（`ond-spec-id: targets/kagura-v1/data-layout`）に従う。

## 出典と照合事項

以下の`ond-spec-id`は現行文書の定義元を参照するinline表記として使う。\
適合要件IDは定義元本文を参照する索引として示す。

| 見本の規則 | 現行文書の定義元 | 対応する適合要件 |
| --- | --- | --- |
| 整数型、defined type、型同一性 | [Language v1「型」](../../specs/ond/language/v1/types.md#型)（`ond-spec-id: type-system/types`）、[「型同一性」](../../specs/ond/language/v1/types.md#型同一性)（`ond-spec-id: type-system/type-identity`） | `TYPE-02`, `NUM-01` |
| literal既定型と型文脈 | [Language v1「型同一性」](../../specs/ond/language/v1/types.md#型同一性)（`ond-spec-id: type-system/type-identity`） | `NUM-01` |
| runtime wrap、除算・剰余、bitwise、signedness、shift、比較result型 | [Language v1「Integer Arithmetic」](../../specs/ond/language/v1/expressions.md#integer-arithmetic)（`ond-spec-id: expressions/integer-arithmetic`）、[式一覧](../../specs/ond/language/v1/expressions.md#式と評価順)（`ond-spec-id: expressions/overview`） | `NUM-04`, `NUM-05`, `OP-01` |
| 正確値定数、範囲、定数shift、最適化 | [Language v1「Integer Constant Arithmetic」](../../specs/ond/language/v1/expressions.md#integer-constant-arithmetic)（`ond-spec-id: expressions/integer-constant-arithmetic`） | `NUM-02`, `NUM-03` |
| cast範囲、runtime整数変換 | [Language v1「型同一性」](../../specs/ond/language/v1/types.md#型同一性)（`ond-spec-id: type-system/type-identity`） | `CAST-01`, `CAST-02` |
| operand・argument評価順、short-circuit、const call適格性 | [Language v1「式」](../../specs/ond/language/v1/expressions.md#式と評価順)（`ond-spec-id: expressions/overview`）、[Syntax v1「Grammar Notes」](../../specs/ond/language/v1/declarations.md#構文と静的条件の対応)（`ond-spec-id: syntax/grammar-notes`） | `DECL-02`, `EVAL-01` |
| builtin `len`、operand評価、operator len | [Built-ins v1「len」](../../specs/ond/language/v1/builtins.md#4-len)（`ond-spec-id: builtins/len`）、[配列len](../../specs/ond/language/v1/builtins.md#41-配列に対する-len)（`ond-spec-id: builtins/len-arrays`） | `LEN-01`, `BI-02` |
| `sizeof`, `alignof`とKagura layout | [Built-ins v1の一覧](../../specs/ond/language/v1/builtins.md#組み込み構文)（`ond-spec-id: builtins/syntax`）、[Kagura Data Layout](../../specs/ond/targets/kagura-v1/machine.md)（`ond-spec-id: targets/kagura-v1/data-layout`） | 既存直接対応IDなし |
| f32、operator、nilとpointer委譲 | [Language v1「Floating-Point Arithmetic」](../../specs/ond/language/v1/expressions.md#floating-point-arithmetic)（`ond-spec-id: expressions/floating-point-arithmetic`）、[user-defined operators](../../specs/ond/language/v1/expressions.md#user-defined-operator)（`ond-spec-id: functions/user-defined-operators`）、[nilとポインタ](../../specs/ond/language/v1/types.md#nilとpointer)（`ond-spec-id: type-system/nil-and-pointers`） | `FP-01`, `FP-02`, `OP-01`, `PTR-01` |

本表は見本の規則と現行定義元を照合する編集記録であり、決裁済みの整数型対応と定数評価境界を反映する。\
本移行の本文、要件IDと期待値の照合結果は[本移行記録](./migration-record.md)に記す。
