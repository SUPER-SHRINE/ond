# 字句回帰の入力と期待値
<!-- ond-spec-id: conformance/cases/lexical-observations -->

適用版はOnd Language v1、Unicode分類は[字句規則](../../language/v1/lexical.md#source形式)の版とする。\
source欄は`main.ond`の全byte列または明記したtemplateのparameterであり、[共通の入力復元規則](./README.md#入力と期待結果の読み方)に従う。\
字句操作ではtoken化だけを行い、Coreと記した行ではproject全体の名前・型・定数を検査する。\
manifestは共通契約に従う。

字句の拒否は指定件数のError診断、message contains、primary位置が入力file内の非空UTF-8 byte範囲であること、および両端がUTF-8文字境界であることを照合する。\
末尾LFは表にある場合だけ置く。

## 識別子と公開名
<!-- ond-spec-id: conformance/lexical/identifiers-exports -->

要件は`SYN-01`である。\
字句入力は各name単独とし、受理行は1個のidentifier tokenとfile全体のbyte範囲を照合する。

| 入力name | 字句期待 |
| --- | --- |
| <code>日本語</code> | identifier、受理 |
| <code>αβ</code> | identifier、受理 |
| <code>𐐀</code> | identifier、受理 |
| <code>a٣</code> | identifier、受理 |
| <code>_９</code> | identifier、受理 |
| <code>é</code> | identifier、受理 |
| <code>ǅ</code> | identifier、受理 |
| <code>ʰ</code> | identifier、受理 |
| <code>٣a</code> | Error1件、contains `unexpected character` |
| <code>a²</code> | Error1件、contains `unexpected character` |
| <code>á</code> | Error1件、contains `unexpected character` |
| <code>ͅ</code> | Error1件、contains `unexpected character` |
| <code>a‌</code> | Error1件、contains `unexpected character` |
| <code>Ⅳ</code> | Error1件、contains `unexpected character` |
| <code>😀</code> | Error1件、contains `unexpected character` |

Unicode識別子のCore入力は次の`main.ond`とし、受理を期待する。

<code>package main<br>func main() { var 日本語 = 1; var α٢ = 日本語; _ = α٢ }</code>

Coreの公開名caseは`main.ond`と`shared/shared.ond`を次の内容で置いて受理する。\
両fileの`A値`を`値`へ置換した別caseはError1件、contains `not exported`、主位置をmainの`shared.値`範囲とする。

```ond
package main
import "test/shared"
func main() { _ = shared.A値 }
```

```ond
package shared
const A値 = 1
```

## 数値tokenと定数値
<!-- ond-spec-id: conformance/lexical/numeric-tokens -->

要件は`SYN-03`、`SYN-04`、`FP-01`である。\
入力を1個の数値literal tokenとして受理し、Core値のある行では`package main`、`const X: T = L`、`func main() {}`の3行（LF区切り・末尾LFなし）へ展開して型と値を比較する。\
Tは整数行で`u32`、float行で`f32`、Lは入力欄とする。

| 整数L | CoreのX:u32 |
| --- | --- |
| <code>0</code> | 0 |
| <code>42</code> | 42 |
| <code>4_2</code> | 42 |
| <code>0600</code> | 384 |
| <code>0_600</code> | 384 |
| <code>0o600</code> | 384 |
| <code>0O_600</code> | 384 |
| <code>0b_101</code> | 5 |
| <code>0B101</code> | 5 |
| <code>0x_dead_BEEF</code> | 3735928559 |
| <code>0X42</code> | 66 |

float tokenの受理入力は次の列とする。

<code>.5</code>, <code>1.</code>, <code>1.e2</code>, <code>1e-2</code>, <code>1E+2</code>, <code>0.15e+0_2</code>, <code>08.5</code>, <code>0x1p0</code>, <code>0X_1.Fp+2</code>, <code>0x.8p-0</code>, <code>0x1.p0</code>, <code>0x1_2.3_4P-1_0</code>

巨大整数`999999999999999999999999999999999999999999999999999`は字句受理を比較する。\
`0x1e2 + 1e2 - .5 * 1.`のtoken種別はinteger、`+`、float、`-`、float、`*`、floatとなる。

| 字句拒否入力 | message contains（既定Error1件） |
| --- | --- |
| <code>08</code> | <code>invalid digit for numeric radix</code> |
| <code>0_9</code> | <code>underscore must separate digits</code> |
| <code>0b2</code> | <code>invalid digit for numeric radix</code> |
| <code>0o8</code> | <code>invalid digit for numeric radix</code> |
| <code>0x</code> | <code>numeric literal requires digits</code> |
| <code>0x_</code> | <code>underscore must separate digits</code> |
| <code>42_</code> | <code>underscore must separate digits</code> |
| <code>4__2</code> | <code>underscore must separate digits</code> |
| <code>0_x1</code> | <code>underscore must separate digits</code> |
| <code>0b__1</code> | <code>underscore must separate digits</code> |
| <code>1_.0</code> | <code>underscore must separate digits</code> |
| <code>1._0</code> | <code>underscore must separate digits</code> |
| <code>1e_2</code> | <code>underscore must separate digits</code> |
| <code>1e2_</code> | <code>underscore must separate digits</code> |
| <code>1e</code> | <code>exponent requires decimal digits</code> |
| <code>1e+</code> | <code>exponent requires decimal digits</code> |
| <code>0x1.0</code> | <code>hexadecimal float requires a p exponent</code> |
| <code>0x1p</code> | <code>exponent requires decimal digits</code> |
| <code>0x1p-</code> | <code>exponent requires decimal digits</code> |
| <code>0x_ .1p0</code> | <code>Error2件: underscore must separate digits / decimal float requires an e exponent</code> |
| <code>0x_.1p0</code> | <code>underscore must separate digits</code> |
| <code>0x1._2p0</code> | <code>underscore must separate digits</code> |
| <code>0x1p_0</code> | <code>underscore must separate digits</code> |
| <code>0b1.0</code> | <code>floating-point literal requires decimal or hexadecimal radix</code> |
| <code>1p2</code> | <code>decimal float requires an e exponent</code> |

追加のParse入力は次の`main.ond`全文（末尾LFなし）とし、Error1件、contains <code>expected `;` or newline</code>、主位置は末尾の`i`のUTF-8範囲である。

<code>package main<br>func main() {}<br>var x = 1i</code>

規範定義元は[literalの型・範囲](../../language/v1/types.md#型同一性)と[f32規則](../../language/v1/expressions.md#floating-point-arithmetic)とし、次はCore templateのXのbinary32 bitsを比較する。

| float L | Xのu32 bits |
| --- | --- |
| <code>0x1p0</code> | `0x3f800000` |
| <code>0x1.000001p0</code> | `0x3f800000` |
| <code>0x1.00000100000000000001p0</code> | `0x3f800001` |
| <code>0x1.000003p0</code> | `0x3f800002` |
| <code>0x1.fffffep127</code> | `0x7f7fffff` |
| <code>0x1p-126</code> | `0x00800000` |
| <code>0x1p-149</code> | `0x00000001` |
| <code>0x1p-150</code> | `0x00000000` |
| <code>0x1.00000000001p-150</code> | `0x00000001` |
| <code>0x3p-150</code> | `0x00000002` |
| <code>0x1.fffffcp-127</code> | `0x007fffff` |
| <code>0x1.fffffep-127</code> | `0x00800000` |
| <code>0x0p9999999999999999999999</code> | `0x00000000` |
| <code>0x1p-9999999999999999999999</code> | `0x00000000` |

`0x1p128`、`0x1.ffffffp127`、`0x1p9999999999999999999999`、`1e100`はCoreでliteral範囲外のcompile errorとする。

## Stringとsource byte
<!-- ond-spec-id: conformance/lexical/strings-source-bytes -->

要件は`SYN-02`、`SYN-05`、`SYN-06`である。\
Coreでのstring値比較は、数値Core templateの宣言を`const X = L`として型とbyte列を比較する。

| 入力L | 型・decoded bytes |
| --- | --- |
| <code>&quot;日本語\x00\377\u0041\U0001F600&quot;</code> | [16]u8: e6 97 a5 e6 9c ac e8 aa 9e 00 ff 41 f0 9f 98 80 |
| <code>`a&#13;<br>b&#13;c`</code> | [4]u8: 61 0a 62 63 |
| <code>&quot;a&#13;b&quot;</code> | [3]u8: 61 0d 62 |

| 字句拒否入力 | 診断message contains（既定Error1件） |
| --- | --- |
| <code>&quot;\q&quot;</code> | invalid string escape |
| <code>&quot;\x0&quot;</code> | invalid numeric string escape |
| <code>&quot;\xgg&quot;</code> | invalid numeric string escape |
| <code>&quot;\400&quot;</code> | string byte escape out of range |
| <code>&quot;\uD800&quot;</code> | invalid Unicode scalar in string escape |
| <code>&quot;\U00110000&quot;</code> | invalid Unicode scalar in string escape |
| <code>&quot;a<br>b&quot;</code> | Error2件、両方unterminated string literal |
| <code>&quot;a\<br>b&quot;</code> | Error2件、両方unterminated string literal |

Parseで受理する完全sourceは次の3入力（末尾LFなし）とする。

| 入力構成 | 期待 |
| --- | --- |
| <code>package main<br>func main() {}</code> | Parse受理 |
| <code>package main<br>func main() {} // trailing comment</code> | Parse受理 |
| <code>﻿package main&#13;<br>func main() {}</code> | Parse受理 |

NUL/BOM制約の字句拒否入力は次のhex byte列であり、各々Error1件、contains `NUL or non-leading byte order mark in source`とする。

- `70 61 63 6b 61 67 65 00 6d 61 69 6e`
- `2f 2f 00`
- `60 00 60`
- `2f 2f ef bb bf`
- `70 61 63 6b 61 67 65 20 6d 61 69 6e 0a ef bb bf 66 75 6e 63 20 6d 61 69 6e 28 29 20 7b 7d`

`/* not closed`はError1件、contains `unterminated block comment`で拒否する。\
字句入力`値/* comment`+LF+`next */次`は、`値`の後と`次`の後にsemicolonを挿入する。\
この入力ではtoken境界をUTF-8文字境界と比較する。
