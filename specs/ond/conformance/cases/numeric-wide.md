# 64 bit数値と固定小数点の適合case
<!-- ond-spec-id: conformance/cases/numeric-wide -->

適用版はOnd Language v1、target `kagura-v1`、ABI `ond-kagura-v2`とする。\
各code blockを独立したprojectの `main.ond` とし、manifestは[共通case契約](./README.md#入力と期待結果の読み方)に従う。\
Kagura実行では[整数実行vectorの配置と起動](./kagura-integers.md#kagura上の整数実行vector)を使う。\
このfamilyのRAMは `[0x1000,0x11000)`、stack sizeは `0x2000` とし、初期SPは `0x11000`、stack下限は `0xf000` とする。\
出力 `0xffff8050` は0、終了通知 `0xffff8040` は `0xdeadbeef` で初期化する。\
計算領域 `0x20000000` をmapするのはWIDE.memoryのときであり、範囲と初期値をそのcaseに示す。\
caseが変更する領域以外は共通初期状態とし、正常終了時はSPとcallee-save registerも初期値と照合する。\
正常終了を期待するcaseの `0xffff8050` は書込み可能な4 byteの出力領域とし、そこに書かれたu32を比較する。

## WIDE.integer
<!-- ond-spec-id: conformance/numeric-wide/integer -->

要件は `NUM-01`〜`NUM-05` と `CAST-02`。\
`maximum` の定数加算・減算の正確値は2^64−1となる。\
実行時の加算・減算・乗算は64 bitでwrapし、shift countの65は1になる。\
符号付き最小値の右shiftと余り、符号をまたぐ変換、および32 bit raw pointerを経由する変換が成立すると出力は1となる。

```ond
package main
const maximum: u64 = (18446744073709551615 + 1) - 1
func check(a: u64, b: i64, count: u32) -> u32 {
    if a + 1 != 0 || a * 2 != 18446744073709551614 { return 0 }
    if (a + 1) - 1 != a || a << count != 18446744073709551614 { return 0 }
    if b >> count != -4611686018427387904 || b % -1 != 0 { return 0 }
    if a / 3 != 6148914691236517205 || a % 3 != 0 { return 0 }
    if b / 3 != -3074457345618258602 || b % 3 != -2 { return 0 }
    if a & 0x8000000000000001 != 0x8000000000000001 { return 0 }
    var cross = count - 32
    var one = a / a
    if one << cross != 8589934592 || a >> cross != 2147483647 { return 0 }
    if b >> cross != -1073741824 { return 0 }
    if a as i64 != -1 || (a as i64) as u64 != a { return 0 }
    var pointer = a as *u8
    if pointer as u64 != 4294967295 || pointer as i64 != 4294967295 { return 0 }
    return 1
}
func main() {
    *(4294934608 as u32 as *u32) = check(maximum, -9223372036854775808, 65)
}
```

## WIDE.float
<!-- ond-spec-id: conformance/numeric-wide/float -->

要件は `FP-01`、`FP-02`、`CAST-03`。\
以下はbinary64の格納bitsを読み出し、順に正確な1/10からの丸め、binary32からの拡張、加算のtie、canonical NaN、負のzero、最小subnormal、型間変換を比較する。\
NaNとの順序比較はfalseとなり、元のNaN payloadはコピーしても保持される。\
有限値の算術オーバーフローはInf、最小subnormalの半分は+0になる。\
すべて一致すると出力は1となる。

```ond
package main
func bits(x: f64) -> u64 { return *((&x) as u32 as *u64) }
func frombits(x: u64) -> f64 { return *((&x) as u32 as *f64) }
const halfSum: f64 = 1.0 + 0x1p-53
func check(a: f64, z: f64, narrow: f32) -> u32 {
    if bits(0.1 as f64) != 0x3fb999999999999a { return 0 }
    if bits(narrow as f64) != 0x3fb99999a0000000 { return 0 }
    if bits(a + 0x1p-53) != 0x3ff0000000000000 || bits(halfSum) != bits(a) { return 0 }
    var nan = z / z
    if bits(nan) != 0x7ff8000000000000 || bits(-z) != 0x8000000000000000 { return 0 }
    if nan == nan || !(nan != nan) || nan < a || nan <= a || nan > a || nan >= a { return 0 }
    if bits(a * 1.5) != 0x3ff8000000000000 || bits(a / 3) != 0x3fd5555555555555 { return 0 }
    if bits(a - 0.5) != 0x3fe0000000000000 { return 0 }
    if bits(frombits(0x7fefffffffffffff) * 2) != 0x7ff0000000000000 { return 0 }
    if bits(frombits(1) * 0.5) != 0 { return 0 }
    if bits(0x1p-1074 as f64) != 1 || bits(frombits(0x7ff8000000000001)) != 0x7ff8000000000001 { return 0 }
    if (9007199254740993 as u64 as f64) as u64 != 9007199254740992 { return 0 }
    return 1
}
func main() { *(4294934608 as u32 as *u32) = check(1.0, 0.0, 0.1) }
```

## WIDE.fixed
<!-- ond-spec-id: conformance/numeric-wide/fixed -->

要件は `FIX-01`、`CAST-03`。\
以下はquantizationの境界、正負のtie、演算ごとの丸め、格納整数幅を超える中間積、除算と余りの異なる丸め、数値型間の変換を照合する。\
`small` の定数積と同じ実行時積は、0.5刻みを偶数の0に丸めてから0と加える。\
固定小数点で表現できない商を持つ `minimum % -0x1p-32` の結果は0となる。\
すべて一致すると出力は1となる。

```ond
package main
func bits16(x: q16) -> i32 { return *((&x) as u32 as *i32) }
func bits32(x: q32) -> i64 { return *((&x) as u32 as *i64) }
const minimum: q32 = -2147483648.0
const small: q16 = (0x1p-16 * 0.5) + (0x1p-16 * 0.5)
func check(a: q32, b: q32, step: q16, f: f32) -> u32 {
    if bits16(0x1p-17 as q16) != 0 || bits16(0x3p-17 as q16) != 2 { return 0 }
    if bits16((-0x3p-17) as q16) != -2 || bits16((-32768.0) as q16) != -2147483648 { return 0 }
    if bits32(0x1p-33 as q32) != 0 || bits32((-0x3p-33) as q32) != -2 { return 0 }
    if bits16(small) != 0 || bits16(step * 0.5 + step * 0.5) != 0 { return 0 }
    if bits32(a * b) != 0x600000000 || bits32(a / b) != 0xaaaaaaab { return 0 }
    if bits32(minimum) != -9223372036854775808 || bits32(minimum % -0x1p-32) != 0 { return 0 }
    var rem: q32 = 1.5
    rem %= -1
    if rem != 0.5 || (-rem - 1) % 1 != -0.5 { return 0 }
    rem++
    if rem != 1.5 || (-rem) as i32 != -1 || rem as f64 != 1.5 { return 0 }
    if bits32(0.1 as q32) != 0x1999999a || bits32(f as q32) != 0x199999a0 { return 0 }
    if bits16(rem as q16) != 98304 || bits32((2 as i64) as q32) != 8589934592 { return 0 }
    return 1
}
func main() { *(4294934608 as u32 as *u32) = check(2, 3, 0x1p-16, 0.1) }
```

## WIDE.fault
<!-- ond-spec-id: conformance/numeric-wide/fault -->

要件は `NUM-04`、`FIX-01`、`CAST-03`、`BOUND-01`。\
`OPERATION`を表の各式へ置き換えて独立に実行する。\
各行は `InvalidInstruction` faultで停止する。\
停止時にはdebug operationを表と比較し、出力への書込みが1のままであることを確認する。

```ond
package main
func test(n: i64, divisor: i64, q: q16, zero: q16, v: f64, index: u64) {
    var array: [1]u8
    _ = OPERATION
}
func main() {
    *(4294934608 as u32 as *u32) = 1
    test(-9223372036854775808, -1, -32768.0, 0, 1e300, 4294967296)
    *(4294934608 as u32 as *u32) = 2
}
```

| 式 | debug operation |
| --- | --- |
| `n / divisor` | `SignedDivisionOverflow` |
| `n / (divisor + 1)` | `DivisionByZero` |
| `-q` | `FixedPointOverflow` |
| `q % zero` | `DivisionByZero` |
| `q / zero` | `DivisionByZero` |
| `v as f32` | `InvalidConversion` |
| `(v / 0) as q16` | `InvalidConversion` |
| `array[index]` | `BoundsCheck` |

## WIDE.reject
<!-- ond-spec-id: conformance/numeric-wide/reject -->

要件は `NUM-02`、`FIX-01`、`CAST-03`、`OP-01`、`ASM-02`、`NAME-03`。\
次のprojectはCoreで受理する。\
`DECLARATION` を表の各宣言へ置き換えると、記載された原因でCoreは拒否する。

```ond
package main
func main() {}
var validU: u64 = 1
var validI: i64 = -1
var validF: f64 = 0.1
var validQ: q16 = -32768.0
var validQ32: q32 = 1.0
// DECLARATION
```

| 宣言 | 原因 |
| --- | --- |
| `const value: u64 = 18446744073709551616` | 整数定数の範囲外 |
| `const value: q16 = 32768.0` | 丸めたリテラルの範囲外 |
| `const value: q16 = -((-32768.0) as q16)` | 定数単項演算の範囲外 |
| `const value: q16 = (1 as q16) % 0` | 固定小数点剰余の除数zero |
| `const value: f32 = (1e300 as f64) as f32` | 有限値を変換すると範囲外 |
| `func check(v: q16) -> q16 { return v << 1 }` | 固定小数点型のshift |
| `var q16:i32=0` | 組み込み型名のbinding |
| `func check(v: u64) { asm (r1 <- v) { ADD r0, r0, r0, 0 } }` | Direct-2を単一registerにbinding |

## WIDE.abi
<!-- ond-spec-id: conformance/numeric-wide/abi -->

要件は `OUT-01` と `BI-04`。\
`split` へ制御が移った時点の `r1..r6` は順に1、2、3、4、5、`0x55667788`。\
そのときの `[CFA]`、`[CFA+4]` は `0x11223344`、9となる。\
return直後の `r1..r4` は順に `0x55667788`、`0x11223344`、9、15である。\
`memory` はDirectの合計5 wordを返すため、entryの `r1` がreturn area、`r2..r3` が引数の低位・高位wordとなる。\
return後のareaに並ぶ5 wordは `0x55667788`、`0x11223344`、0、`0x3ff00000`、`0x00020000`。\
正常終了時、出力は1となる。

```ond
package main
type L struct { a:u8; b:f64; c:u8 }
func split(a:u32,b:u32,c:u32,d:u32,e:u32,v:u64,last:u32) -> (u64,u32,u32) {
    return v,last,a+b+c+d+e
}
func memory(v:u64) -> (u64,f64,q16) { return v,1.0,2 }
func main() {
    var wide: u64 = 0x1122334455667788
    var value, last, sum = split(1,2,3,4,5,wide,9)
    var same, float, fixed = memory(value)
    var layout:L
    var delta = (&layout.b) as u32 - (&layout) as u32
    if same == wide && last == 9 && sum == 15 && float == 1 && fixed == 2 &&
        sizeof(u64) == 8 && alignof(q32) == 4 && sizeof(L) == 16 && delta == 4 {
        *(4294934608 as u32 as *u32) = 1
    }
}
```

## WIDE.memory
<!-- ond-spec-id: conformance/numeric-wide/memory -->

要件は `PTR-01` と `FAULT-01`、定義元は[raw pointer access](../../targets/kagura-v1/machine.md#6-raw-pointer-access)。\
`0x20000000` から64 byteを `0xcc` で初期化し、offset 0・2・4のそれぞれのu8・u16・u8を11・1234・22で上書きする。\
この範囲への32 bit accessの列を、読取りoffset 0、4、書込みoffset 16、20の順と比較する。\
表のfaultは当該accessの応答としてbusが返す。\
値は4 byte little-endian単位で比較し、停止後に後続のaccessを行わない。

| fault注入 | 停止 | offset 16のword | offset 20のword |
| --- | --- | --- | --- |
| なし | 正常終了 | `0x04d2cc0b` | `0xcccccc16` |
| 2番目の読取り | BusFault | `0xcccccccc` | `0xcccccccc` |
| 2番目の書込み | BusFault | `0x04d2cc0b` | `0xcccccccc` |

```ond
package main
func main() {
    var p = 536870912 as u32 as *u64
    var q = 536870928 as u32 as *u64
    *q = *p
}
```

## WIDE.retention
<!-- ond-spec-id: conformance/numeric-wide/retention -->

要件は `FP-02` と `MEM-02`、定義元は[一時値の配置裁量](../../targets/kagura-v1/machine.md#34-非公開ローカル集約値)。\
通常のstack容量で実行し、出力のbinary32 bits `0x00000000` と正常終了を比較する。\
`1 + 2^-24` は偶数側の1へ丸められた後に1を引く。\
`save` が参照する変数への値の格納と呼出しを保持し、一時値のRAM内容とframe byte数は比較対象に含めない。

```ond
package main
func add(a:f32,b:f32) -> f32 { return a+b }
func save(p:*f32, v:f32) { *p = v }
func main() {
    var input:f32 = 1.0
    var middle = add(input,0x1p-24)
    var other = add(middle,-input)
    var result:f32 = 8.0
    save(&result,other)
    *(4294934608 as u32 as *u32) = *((&result) as u32 as *u32)
}
```
