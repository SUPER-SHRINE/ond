# Kagura上の数値変換vector
<!-- ond-spec-id: conformance/cases/kagura-casts -->

本書の各行はCAST-03の入力と固定期待bitsを定める。\
共通manifestとINPUT/OUTPUT/SENTINELは[整数実行vector](./kagura-integers.md)と同じとする。\
sourceはmain.ondで、INPUTのwordをsource型として読み、`value as 変換先型`をOUTPUTへ格納する。\
f32はbit再解釈helperでwordとの間を運び、整数→f32は結果の32bit全体を、f32→整数は変換先型幅で書く。\
狭いstoreの上位byteは事前のSENTINELを保持する。

int→f32の期待はroundTiesToEven、f32→intは0方向丸め後の型範囲検査から導く。\
NaN/Infと範囲違反はfault、fault時は出力全体を保持する。\
定義元は[型変換](../../language/v1/types.md#型同一性)、[binary32](../../language/v1/expressions.md#floating-point-arithmetic)である。\
比較は表の入力と固定期待bitで行う。

sourceのTYPEは表のsource整数型または変換先整数型とし、[bit転送fixture関数](./numeric-float-vectors.md#kagura-runtimeの共有入力)の`bits`と`frombits`の宣言をpackage clause直後に置く。\
整数からf32の`main.ond`は次のtemplateを使う。

```ond
package main
BITS_FUNCTIONS
func main(){var address:u32=536870912;var input=address as *u32;var a=(*input) as TYPE;*(536871168 as u32 as *u32)=bits(a as f32)}
```

f32から整数は次のtemplateを使う。\
BITS_FUNCTIONSは上記2関数の宣言で置換し、命令上限は両方向とも2,000,000とする。

```ond
package main
BITS_FUNCTIONS
func main(){var address:u32=536870912;var input=address as *u32;var a=frombits(*input);var p=536871168 as *TYPE;*p=a as TYPE}
```

| Case ID | source型 | 変換先型 | INPUT word | 期待OUTPUT word | 期待停止 |
| --- | --- | --- | --- | --- | --- |
| <code>CAST-03.u8.f32.0</code> | <code>u8</code> | <code>f32</code> | <code>0x00000000</code> | <code>0x00000000</code> | <code>return</code> |
| <code>CAST-03.u8.f32.255</code> | <code>u8</code> | <code>f32</code> | <code>0x000000ff</code> | <code>0x437f0000</code> | <code>return</code> |
| <code>CAST-03.i8.f32.4294967168</code> | <code>i8</code> | <code>f32</code> | <code>0xffffff80</code> | <code>0xc3000000</code> | <code>return</code> |
| <code>CAST-03.i8.f32.127</code> | <code>i8</code> | <code>f32</code> | <code>0x0000007f</code> | <code>0x42fe0000</code> | <code>return</code> |
| <code>CAST-03.u16.f32.0</code> | <code>u16</code> | <code>f32</code> | <code>0x00000000</code> | <code>0x00000000</code> | <code>return</code> |
| <code>CAST-03.u16.f32.65535</code> | <code>u16</code> | <code>f32</code> | <code>0x0000ffff</code> | <code>0x477fff00</code> | <code>return</code> |
| <code>CAST-03.i16.f32.4294934528</code> | <code>i16</code> | <code>f32</code> | <code>0xffff8000</code> | <code>0xc7000000</code> | <code>return</code> |
| <code>CAST-03.i16.f32.32767</code> | <code>i16</code> | <code>f32</code> | <code>0x00007fff</code> | <code>0x46fffe00</code> | <code>return</code> |
| <code>CAST-03.u32.f32.0</code> | <code>u32</code> | <code>f32</code> | <code>0x00000000</code> | <code>0x00000000</code> | <code>return</code> |
| <code>CAST-03.u32.f32.4294967295</code> | <code>u32</code> | <code>f32</code> | <code>0xffffffff</code> | <code>0x4f800000</code> | <code>return</code> |
| <code>CAST-03.u32.f32.16777215</code> | <code>u32</code> | <code>f32</code> | <code>0x00ffffff</code> | <code>0x4b7fffff</code> | <code>return</code> |
| <code>CAST-03.u32.f32.16777216</code> | <code>u32</code> | <code>f32</code> | <code>0x01000000</code> | <code>0x4b800000</code> | <code>return</code> |
| <code>CAST-03.u32.f32.16777217</code> | <code>u32</code> | <code>f32</code> | <code>0x01000001</code> | <code>0x4b800000</code> | <code>return</code> |
| <code>CAST-03.u32.f32.16777218</code> | <code>u32</code> | <code>f32</code> | <code>0x01000002</code> | <code>0x4b800001</code> | <code>return</code> |
| <code>CAST-03.u32.f32.16777219</code> | <code>u32</code> | <code>f32</code> | <code>0x01000003</code> | <code>0x4b800002</code> | <code>return</code> |
| <code>CAST-03.i32.f32.2147483648</code> | <code>i32</code> | <code>f32</code> | <code>0x80000000</code> | <code>0xcf000000</code> | <code>return</code> |
| <code>CAST-03.i32.f32.2147483647</code> | <code>i32</code> | <code>f32</code> | <code>0x7fffffff</code> | <code>0x4f000000</code> | <code>return</code> |
| <code>CAST-03.i32.f32.16777215</code> | <code>i32</code> | <code>f32</code> | <code>0x00ffffff</code> | <code>0x4b7fffff</code> | <code>return</code> |
| <code>CAST-03.i32.f32.4278190081</code> | <code>i32</code> | <code>f32</code> | <code>0xff000001</code> | <code>0xcb7fffff</code> | <code>return</code> |
| <code>CAST-03.i32.f32.16777216</code> | <code>i32</code> | <code>f32</code> | <code>0x01000000</code> | <code>0x4b800000</code> | <code>return</code> |
| <code>CAST-03.i32.f32.4278190080</code> | <code>i32</code> | <code>f32</code> | <code>0xff000000</code> | <code>0xcb800000</code> | <code>return</code> |
| <code>CAST-03.i32.f32.16777217</code> | <code>i32</code> | <code>f32</code> | <code>0x01000001</code> | <code>0x4b800000</code> | <code>return</code> |
| <code>CAST-03.i32.f32.4278190079</code> | <code>i32</code> | <code>f32</code> | <code>0xfeffffff</code> | <code>0xcb800000</code> | <code>return</code> |
| <code>CAST-03.i32.f32.16777218</code> | <code>i32</code> | <code>f32</code> | <code>0x01000002</code> | <code>0x4b800001</code> | <code>return</code> |
| <code>CAST-03.i32.f32.4278190078</code> | <code>i32</code> | <code>f32</code> | <code>0xfefffffe</code> | <code>0xcb800001</code> | <code>return</code> |
| <code>CAST-03.i32.f32.16777219</code> | <code>i32</code> | <code>f32</code> | <code>0x01000003</code> | <code>0x4b800002</code> | <code>return</code> |
| <code>CAST-03.i32.f32.4278190077</code> | <code>i32</code> | <code>f32</code> | <code>0xfefffffd</code> | <code>0xcb800002</code> | <code>return</code> |
| <code>CAST-03.f32.u8.0</code> | <code>f32</code> | <code>u8</code> | <code>0x00000000</code> | <code>0xdeadbe00</code> | <code>return</code> |
| <code>CAST-03.f32.u8.bf800000</code> | <code>f32</code> | <code>u8</code> | <code>0xbf800000</code> | <code>0xdeadbeef</code> | <code>InvalidInstruction</code> |
| <code>CAST-03.f32.u8.437f0000</code> | <code>f32</code> | <code>u8</code> | <code>0x437f0000</code> | <code>0xdeadbeff</code> | <code>return</code> |
| <code>CAST-03.f32.u8.43800000</code> | <code>f32</code> | <code>u8</code> | <code>0x43800000</code> | <code>0xdeadbeef</code> | <code>InvalidInstruction</code> |
| <code>CAST-03.f32.u8.3fe00000</code> | <code>f32</code> | <code>u8</code> | <code>0x3fe00000</code> | <code>0xdeadbe01</code> | <code>return</code> |
| <code>CAST-03.f32.u8.bfe00000</code> | <code>f32</code> | <code>u8</code> | <code>0xbfe00000</code> | <code>0xdeadbeef</code> | <code>InvalidInstruction</code> |
| <code>CAST-03.f32.u8.80000000</code> | <code>f32</code> | <code>u8</code> | <code>0x80000000</code> | <code>0xdeadbe00</code> | <code>return</code> |
| <code>CAST-03.f32.u8.bf400000</code> | <code>f32</code> | <code>u8</code> | <code>0xbf400000</code> | <code>0xdeadbe00</code> | <code>return</code> |
| <code>CAST-03.f32.u8.7fc00000</code> | <code>f32</code> | <code>u8</code> | <code>0x7fc00000</code> | <code>0xdeadbeef</code> | <code>InvalidInstruction</code> |
| <code>CAST-03.f32.u8.7f800000</code> | <code>f32</code> | <code>u8</code> | <code>0x7f800000</code> | <code>0xdeadbeef</code> | <code>InvalidInstruction</code> |
| <code>CAST-03.f32.u8.ff800000</code> | <code>f32</code> | <code>u8</code> | <code>0xff800000</code> | <code>0xdeadbeef</code> | <code>InvalidInstruction</code> |
| <code>CAST-03.f32.i8.c3000000</code> | <code>f32</code> | <code>i8</code> | <code>0xc3000000</code> | <code>0xdeadbe80</code> | <code>return</code> |
| <code>CAST-03.f32.i8.c3010000</code> | <code>f32</code> | <code>i8</code> | <code>0xc3010000</code> | <code>0xdeadbeef</code> | <code>InvalidInstruction</code> |
| <code>CAST-03.f32.i8.42fe0000</code> | <code>f32</code> | <code>i8</code> | <code>0x42fe0000</code> | <code>0xdeadbe7f</code> | <code>return</code> |
| <code>CAST-03.f32.i8.43000000</code> | <code>f32</code> | <code>i8</code> | <code>0x43000000</code> | <code>0xdeadbeef</code> | <code>InvalidInstruction</code> |
| <code>CAST-03.f32.i8.3fe00000</code> | <code>f32</code> | <code>i8</code> | <code>0x3fe00000</code> | <code>0xdeadbe01</code> | <code>return</code> |
| <code>CAST-03.f32.i8.bfe00000</code> | <code>f32</code> | <code>i8</code> | <code>0xbfe00000</code> | <code>0xdeadbeff</code> | <code>return</code> |
| <code>CAST-03.f32.i8.80000000</code> | <code>f32</code> | <code>i8</code> | <code>0x80000000</code> | <code>0xdeadbe00</code> | <code>return</code> |
| <code>CAST-03.f32.i8.bf400000</code> | <code>f32</code> | <code>i8</code> | <code>0xbf400000</code> | <code>0xdeadbe00</code> | <code>return</code> |
| <code>CAST-03.f32.i8.7fc00000</code> | <code>f32</code> | <code>i8</code> | <code>0x7fc00000</code> | <code>0xdeadbeef</code> | <code>InvalidInstruction</code> |
| <code>CAST-03.f32.i8.7f800000</code> | <code>f32</code> | <code>i8</code> | <code>0x7f800000</code> | <code>0xdeadbeef</code> | <code>InvalidInstruction</code> |
| <code>CAST-03.f32.i8.ff800000</code> | <code>f32</code> | <code>i8</code> | <code>0xff800000</code> | <code>0xdeadbeef</code> | <code>InvalidInstruction</code> |
| <code>CAST-03.f32.u16.0</code> | <code>f32</code> | <code>u16</code> | <code>0x00000000</code> | <code>0xdead0000</code> | <code>return</code> |
| <code>CAST-03.f32.u16.bf800000</code> | <code>f32</code> | <code>u16</code> | <code>0xbf800000</code> | <code>0xdeadbeef</code> | <code>InvalidInstruction</code> |
| <code>CAST-03.f32.u16.477fff00</code> | <code>f32</code> | <code>u16</code> | <code>0x477fff00</code> | <code>0xdeadffff</code> | <code>return</code> |
| <code>CAST-03.f32.u16.47800000</code> | <code>f32</code> | <code>u16</code> | <code>0x47800000</code> | <code>0xdeadbeef</code> | <code>InvalidInstruction</code> |
| <code>CAST-03.f32.u16.3fe00000</code> | <code>f32</code> | <code>u16</code> | <code>0x3fe00000</code> | <code>0xdead0001</code> | <code>return</code> |
| <code>CAST-03.f32.u16.bfe00000</code> | <code>f32</code> | <code>u16</code> | <code>0xbfe00000</code> | <code>0xdeadbeef</code> | <code>InvalidInstruction</code> |
| <code>CAST-03.f32.u16.80000000</code> | <code>f32</code> | <code>u16</code> | <code>0x80000000</code> | <code>0xdead0000</code> | <code>return</code> |
| <code>CAST-03.f32.u16.bf400000</code> | <code>f32</code> | <code>u16</code> | <code>0xbf400000</code> | <code>0xdead0000</code> | <code>return</code> |
| <code>CAST-03.f32.u16.7fc00000</code> | <code>f32</code> | <code>u16</code> | <code>0x7fc00000</code> | <code>0xdeadbeef</code> | <code>InvalidInstruction</code> |
| <code>CAST-03.f32.u16.7f800000</code> | <code>f32</code> | <code>u16</code> | <code>0x7f800000</code> | <code>0xdeadbeef</code> | <code>InvalidInstruction</code> |
| <code>CAST-03.f32.u16.ff800000</code> | <code>f32</code> | <code>u16</code> | <code>0xff800000</code> | <code>0xdeadbeef</code> | <code>InvalidInstruction</code> |
| <code>CAST-03.f32.i16.c7000000</code> | <code>f32</code> | <code>i16</code> | <code>0xc7000000</code> | <code>0xdead8000</code> | <code>return</code> |
| <code>CAST-03.f32.i16.c7000100</code> | <code>f32</code> | <code>i16</code> | <code>0xc7000100</code> | <code>0xdeadbeef</code> | <code>InvalidInstruction</code> |
| <code>CAST-03.f32.i16.46fffe00</code> | <code>f32</code> | <code>i16</code> | <code>0x46fffe00</code> | <code>0xdead7fff</code> | <code>return</code> |
| <code>CAST-03.f32.i16.47000000</code> | <code>f32</code> | <code>i16</code> | <code>0x47000000</code> | <code>0xdeadbeef</code> | <code>InvalidInstruction</code> |
| <code>CAST-03.f32.i16.3fe00000</code> | <code>f32</code> | <code>i16</code> | <code>0x3fe00000</code> | <code>0xdead0001</code> | <code>return</code> |
| <code>CAST-03.f32.i16.bfe00000</code> | <code>f32</code> | <code>i16</code> | <code>0xbfe00000</code> | <code>0xdeadffff</code> | <code>return</code> |
| <code>CAST-03.f32.i16.80000000</code> | <code>f32</code> | <code>i16</code> | <code>0x80000000</code> | <code>0xdead0000</code> | <code>return</code> |
| <code>CAST-03.f32.i16.bf400000</code> | <code>f32</code> | <code>i16</code> | <code>0xbf400000</code> | <code>0xdead0000</code> | <code>return</code> |
| <code>CAST-03.f32.i16.7fc00000</code> | <code>f32</code> | <code>i16</code> | <code>0x7fc00000</code> | <code>0xdeadbeef</code> | <code>InvalidInstruction</code> |
| <code>CAST-03.f32.i16.7f800000</code> | <code>f32</code> | <code>i16</code> | <code>0x7f800000</code> | <code>0xdeadbeef</code> | <code>InvalidInstruction</code> |
| <code>CAST-03.f32.i16.ff800000</code> | <code>f32</code> | <code>i16</code> | <code>0xff800000</code> | <code>0xdeadbeef</code> | <code>InvalidInstruction</code> |
| <code>CAST-03.f32.u32.0</code> | <code>f32</code> | <code>u32</code> | <code>0x00000000</code> | <code>0x00000000</code> | <code>return</code> |
| <code>CAST-03.f32.u32.bf800000</code> | <code>f32</code> | <code>u32</code> | <code>0xbf800000</code> | <code>0xdeadbeef</code> | <code>InvalidInstruction</code> |
| <code>CAST-03.f32.u32.4f7fffff</code> | <code>f32</code> | <code>u32</code> | <code>0x4f7fffff</code> | <code>0xffffff00</code> | <code>return</code> |
| <code>CAST-03.f32.u32.4f800000</code> | <code>f32</code> | <code>u32</code> | <code>0x4f800000</code> | <code>0xdeadbeef</code> | <code>InvalidInstruction</code> |
| <code>CAST-03.f32.u32.3fe00000</code> | <code>f32</code> | <code>u32</code> | <code>0x3fe00000</code> | <code>0x00000001</code> | <code>return</code> |
| <code>CAST-03.f32.u32.bfe00000</code> | <code>f32</code> | <code>u32</code> | <code>0xbfe00000</code> | <code>0xdeadbeef</code> | <code>InvalidInstruction</code> |
| <code>CAST-03.f32.u32.80000000</code> | <code>f32</code> | <code>u32</code> | <code>0x80000000</code> | <code>0x00000000</code> | <code>return</code> |
| <code>CAST-03.f32.u32.bf400000</code> | <code>f32</code> | <code>u32</code> | <code>0xbf400000</code> | <code>0x00000000</code> | <code>return</code> |
| <code>CAST-03.f32.u32.7fc00000</code> | <code>f32</code> | <code>u32</code> | <code>0x7fc00000</code> | <code>0xdeadbeef</code> | <code>InvalidInstruction</code> |
| <code>CAST-03.f32.u32.7f800000</code> | <code>f32</code> | <code>u32</code> | <code>0x7f800000</code> | <code>0xdeadbeef</code> | <code>InvalidInstruction</code> |
| <code>CAST-03.f32.u32.ff800000</code> | <code>f32</code> | <code>u32</code> | <code>0xff800000</code> | <code>0xdeadbeef</code> | <code>InvalidInstruction</code> |
| <code>CAST-03.f32.i32.cf000000</code> | <code>f32</code> | <code>i32</code> | <code>0xcf000000</code> | <code>0x80000000</code> | <code>return</code> |
| <code>CAST-03.f32.i32.cf000001</code> | <code>f32</code> | <code>i32</code> | <code>0xcf000001</code> | <code>0xdeadbeef</code> | <code>InvalidInstruction</code> |
| <code>CAST-03.f32.i32.4effffff</code> | <code>f32</code> | <code>i32</code> | <code>0x4effffff</code> | <code>0x7fffff80</code> | <code>return</code> |
| <code>CAST-03.f32.i32.4f000000</code> | <code>f32</code> | <code>i32</code> | <code>0x4f000000</code> | <code>0xdeadbeef</code> | <code>InvalidInstruction</code> |
| <code>CAST-03.f32.i32.3fe00000</code> | <code>f32</code> | <code>i32</code> | <code>0x3fe00000</code> | <code>0x00000001</code> | <code>return</code> |
| <code>CAST-03.f32.i32.bfe00000</code> | <code>f32</code> | <code>i32</code> | <code>0xbfe00000</code> | <code>0xffffffff</code> | <code>return</code> |
| <code>CAST-03.f32.i32.80000000</code> | <code>f32</code> | <code>i32</code> | <code>0x80000000</code> | <code>0x00000000</code> | <code>return</code> |
| <code>CAST-03.f32.i32.bf400000</code> | <code>f32</code> | <code>i32</code> | <code>0xbf400000</code> | <code>0x00000000</code> | <code>return</code> |
| <code>CAST-03.f32.i32.7fc00000</code> | <code>f32</code> | <code>i32</code> | <code>0x7fc00000</code> | <code>0xdeadbeef</code> | <code>InvalidInstruction</code> |
| <code>CAST-03.f32.i32.7f800000</code> | <code>f32</code> | <code>i32</code> | <code>0x7f800000</code> | <code>0xdeadbeef</code> | <code>InvalidInstruction</code> |
| <code>CAST-03.f32.i32.ff800000</code> | <code>f32</code> | <code>i32</code> | <code>0xff800000</code> | <code>0xdeadbeef</code> | <code>InvalidInstruction</code> |
