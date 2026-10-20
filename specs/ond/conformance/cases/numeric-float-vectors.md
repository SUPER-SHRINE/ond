# numeric-float-vectorsの入力と期待値
<!-- ond-spec-id: conformance/cases/numeric-float-vectors -->

入力bit patternに対応するbinary32値へ浮動小数点演算規則を適用し、結果をu32 bitsで記す。

規範定義元: [FP-01の定義元](../../language/v1/expressions.md#floating-point-arithmetic)、[FP-02の定義元](../../language/v1/expressions.md#floating-point-arithmetic)。\
要件索引は[requirements](../requirements.md)、既存fixture宣言は[registry](../../../../crates/compiler-core/src/conformance/numeric/float_vectors.rs)を参照する。

共通のproject入力は`ond.toml`の`[project]`に`name = "test"`を置いた実行projectである。\
generated numeric/型境界のsourceはすべて`main.ond`の内容として読み、static表は各file名を入力欄に記す。

`accept`単独は受理を期待し、値の記載がある行はその型・値まで照合する。\
構文解釈は表のParse phaseで照合し、内部AST/MIRの構造検査は外部適合判定へ含めない。\
診断の原文は照合対象を示し、規範上の失敗結果はcompile errorとして読む。

| Case ID | 要件ID | 入力 | 仕様由来の期待結果 | Phase・診断比較 |
| --- | --- | --- | --- | --- |
| <code>FP-02.bits.add_even</code> | `FP-01`, `FP-02` | <code>package main<br>const A: f32 = 0x800000p-23<br>const B: f32 = 0x800000p-47<br>const X = A + B<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x3f800000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-02.bits.add_odd</code> | `FP-01`, `FP-02` | <code>package main<br>const A: f32 = 0x800001p-23<br>const B: f32 = 0x800000p-47<br>const X = A + B<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x3f800002</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-02.bits.sub_even</code> | `FP-01`, `FP-02` | <code>package main<br>const A: f32 = 0x800001p-23<br>const B: f32 = 0x800000p-47<br>const X = A - B<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x3f800000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-02.bits.sub_odd</code> | `FP-01`, `FP-02` | <code>package main<br>const A: f32 = 0x800002p-23<br>const B: f32 = 0x800000p-47<br>const X = A - B<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x3f800002</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-02.bits.mul_even</code> | `FP-01`, `FP-02` | <code>package main<br>const A: f32 = 0x1p-149<br>const B: f32 = 0x800000p-24<br>const X = A * B<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x00000000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-02.bits.mul_odd</code> | `FP-01`, `FP-02` | <code>package main<br>const A: f32 = 0x3p-149<br>const B: f32 = 0x800000p-24<br>const X = A * B<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x00000002</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-02.bits.div_even</code> | `FP-01`, `FP-02` | <code>package main<br>const A: f32 = 0x1p-149<br>const B: f32 = 0x800000p-22<br>const X = A / B<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x00000000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-02.bits.div_odd</code> | `FP-01`, `FP-02` | <code>package main<br>const A: f32 = 0x3p-149<br>const B: f32 = 0x800000p-22<br>const X = A / B<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x00000002</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-02.bits.div_third</code> | `FP-01`, `FP-02` | <code>package main<br>const A: f32 = 0x800000p-23<br>const B: f32 = 0xc00000p-22<br>const X = A / B<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x3eaaaaab</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-02.bits.add_subnormal</code> | `FP-01`, `FP-02` | <code>package main<br>const A: f32 = 0x1p-149<br>const B: f32 = 0x1p-149<br>const X = A + B<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x00000002</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-02.bits.sub_subnormal</code> | `FP-01`, `FP-02` | <code>package main<br>const A: f32 = 0x800000p-149<br>const B: f32 = 0x7fffffp-149<br>const X = A - B<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x00000001</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-02.bits.negative_underflow</code> | `FP-01`, `FP-02` | <code>package main<br>const A: f32 = -0x1p-149<br>const B: f32 = 0x800000p-22<br>const X = A / B<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x80000000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-02.bits.zero_add</code> | `FP-01`, `FP-02` | <code>package main<br>const A: f32 = -0x0p-149<br>const B: f32 = -0x0p-149<br>const X = A + B<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x80000000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-02.bits.zero_sub</code> | `FP-01`, `FP-02` | <code>package main<br>const A: f32 = -0x0p-149<br>const B: f32 = 0x0p-149<br>const X = A - B<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x80000000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-02.bits.zero_mul</code> | `FP-01`, `FP-02` | <code>package main<br>const A: f32 = 0x0p-149<br>const B: f32 = -0x800000p-23<br>const X = A * B<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x80000000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-02.bits.zero_div</code> | `FP-01`, `FP-02` | <code>package main<br>const A: f32 = 0x0p-149<br>const B: f32 = -0x800000p-23<br>const X = A / B<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x80000000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-02.bits.add_overflow</code> | `FP-01`, `FP-02` | <code>package main<br>const A: f32 = 0xffffffp104<br>const B: f32 = 0xffffffp104<br>const X = A + B<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x7f800000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-02.bits.sub_overflow</code> | `FP-01`, `FP-02` | <code>package main<br>const A: f32 = -0xffffffp104<br>const B: f32 = 0xffffffp104<br>const X = A - B<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0xff800000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-02.bits.mul_overflow</code> | `FP-01`, `FP-02` | <code>package main<br>const A: f32 = 0xffffffp104<br>const B: f32 = 0x800000p-22<br>const X = A * B<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x7f800000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-02.bits.div_overflow</code> | `FP-01`, `FP-02` | <code>package main<br>const A: f32 = 0xffffffp104<br>const B: f32 = 0x800000p-24<br>const X = A / B<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x7f800000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-02.bits.inf</code> | `FP-01`, `FP-02` | <code>package main<br>const A: f32 = 0x800000p-23<br>const B: f32 = 0x0p-149<br>const X = A / B<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x7f800000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-02.bits.nan_div</code> | `FP-01`, `FP-02` | <code>package main<br>const A: f32 = 0x0p-149<br>const B: f32 = 0x0p-149<br>const X = A / B<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x7fc00000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-02.bits.nan_mul</code> | `FP-01`, `FP-02` | <code>package main<br>const A: f32 = 1.0 / 0.0<br>const B: f32 = 0x0p-149<br>const X = A * B<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x7fc00000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-02.bits.nan_sub</code> | `FP-01`, `FP-02` | <code>package main<br>const A: f32 = 1.0 / 0.0<br>const B: f32 = 1.0 / 0.0<br>const X = A - B<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x7fc00000</code> | <code>Core; accept; diagnostics=0</code> |
| <code>FP-02.bits.nan_add</code> | `FP-01`, `FP-02` | <code>package main<br>const A: f32 = 1.0 / 0.0<br>const B: f32 = -1.0 / 0.0<br>const X = A + B<br>func main() {}<br></code> | <code>accept; X: f32 = f32 bits 0x7fc00000</code> | <code>Core; accept; diagnostics=0</code> |

## Kagura runtimeの共有入力
<!-- ond-spec-id: conformance/cases/float-vector-runtime-input -->

同じcase IDをruntimeで実行するときは、[Kagura共通状態](./kagura-integers.md)を用いる。\
各Core sourceのA、Bをbinary32として解釈したu32 bitsをINPUTとINPUT+4へ格納し、Xの式の二項演算子をOPへ置換して次の`main.ond`を構成する。\
結果は同じ行のXの期待bitsとOUTPUTのu32 wordを比較する。\
命令上限は2,000,000とする。

```ond
package main
func bits(x:f32)->u32{return *((&x) as u32 as *u32)}
func frombits(x:u32)->f32{var f:f32;*((&f) as u32 as *u32)=x;return f}
func main(){var address:u32=536870912;var input=address as *u32;var a=frombits(input[0]);var b=frombits(input[1]);*(536871168 as u32 as *u32)=bits(a OP b)}
```

`bits`と`frombits`はこのcase群でu32 wordとf32の32bit表現をraw accessで転送するfixture関数とし、数値変換の`as f32`とは区別する。\
数値変換vectorは同じ関数の宣言2個をsourceへ含める。
