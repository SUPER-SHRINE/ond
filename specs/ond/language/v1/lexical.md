# 字句とsource形式
<!-- ond-spec-id: syntax/v1 -->

Ond v1のsourceをtoken列へ変換する規則を定義する。\
構文は本章の字句規則と各概念章のEBNFで定まる。

## Source形式
<!-- ond-spec-id: syntax/source-form -->

source fileはUTF-8 textである。\
sourceの直接のNULと先頭以外のBOMはcompile errorとなり、先頭のUTF-8 BOMは読み飛ばす。\
token間のspace、tab、CR、LFを空白として扱い、LFとCRLFを改行として扱う。

`//`から行末までをline comment、`/*`から最初の`*/`までをblock commentとして空白と同様に扱う。\
閉じる`*/`がないblock commentはcompile errorとなる。\
block comment内にLFがある場合、semicolon insertionでは一つの改行として扱う。

tokenはidentifier、keyword、literal、operator、punctuationからなり、現在位置から最長の有効tokenを取る。\
identifierの開始文字はUnicode 16.0.0のGeneral_Category `Lu`, `Ll`, `Lt`, `Lm`, `Lo`および`_`であり、継続文字には`Nd`も加わる。\
identifierの同一性はsourceの符号位置列で決まり、公開判定は先頭byteがASCII大文字`A`〜`Z`であることを条件とする。

予約語は次の集合である。

```text
package import const var type func struct return defer
break continue if else for true false nil as new
```

`operator`、`trap`、`len`は文脈により構文またはbuiltinを表すidentifierであり、bindingの成立条件は[scope規則](./names-and-scopes.md)に従う。\
構文を構成するoperatorとpunctuationは次のとおりである。

```text
+ - * / % & | ^ &^ << >> == != < <= > >= && || !
= += -= *= /= %= &= |= ^= &^= <<= >>= := ++ --
( ) [ ] { } , . : ; -> ...
```

integer literalは10進、`0b`/`0B` prefixの2進、先頭`0`または`0o`/`0O`の8進、`0x`/`0X`の16進で表す。\
各基数に属する数字を1個以上持ち、underscoreは基数prefix（8進の先頭`0`を含む）直後または数字間に一つ置ける。\
どちらもunderscoreの直後に当該基数の数字が必要である。\
`0_600`も8進literalとして成立する。\
数字の範囲違反、連続underscore、末尾underscoreはcompile errorとなる。

decimal floatは小数点または`e`/`E` exponentを持つ。\
小数点前後の少なくとも一方に数字が必要であり、`.5`、`1.`、`1.e2`、`1e-2`が成立する。\
hexadecimal floatは`0x`/`0X`に続く16進仮数と、`p`/`P`に続く10進exponentを持ち、仮数の小数点を省略できる。\
exponentは省略可能な`+`または`-`と1個以上の10進数字である。\
数字間のunderscore、および直後に16進数字を持つhex prefix直後の一つのunderscoreが成立する。\
float値への丸めは[型と変換](./types.md#型同一性)に従う。

数値literalの数学値は、underscoreをすべて除いた綴りから次のように定める。\
整数literalはprefixを除いた数字列を指定基数の非負整数として読む。単独の`0`の数学値は0であり、8進形式の先頭`0`は値0の数字として残しても同じ値となる。\
decimal floatでは小数点を除いた仮数の10進整数をD、小数点後の数字数をf、exponentの符号付き10進整数をe（省略時0）とし、正確値をD × 10^(e-f)とする。\
hexadecimal floatではprefixと小数点を除いた仮数の16進整数をH、小数点後の16進数字数をf、p/P後の符号付き10進整数をpとし、正確値をH × 2^(p-4f)とする。\
小数点を省略した仮数のfは0とする。\
単項`+`/`-`は[整数演算](./expressions.md#integer-arithmetic)と[f32演算](./expressions.md#floating-point-arithmetic)の定義に従う。

quoted stringは`"`で囲み、通常文字とescapeをbyte列へdecodeする。\
通常文字とUnicode escapeはUTF-8でencodeし、直接のCRは保持する。\
直接のLF、閉じquoteの欠落、不正escapeはcompile errorとなる。\
escapeの集合と値は次の表に従う。

| Escape | 値と成立条件 |
| --- | --- |
| `\a`, `\b`, `\f`, `\n`, `\r`, `\t`, `\v` | それぞれbyte 7, 8, 12, 10, 13, 9, 11 |
| `\\`, `\"` | backslashとdouble quote |
| `\xNN` | 16進2桁による1 byte |
| `\NNN` | 8進3桁で0〜255の1 byte |
| `\uNNNN`, `\UNNNNNNNN` | 16進4桁/8桁のUnicode scalar。U+0000〜U+10FFFFのうちsurrogate U+D800〜U+DFFFを除く |

raw stringはbacktickで囲み、内部をescape解釈せずUTF-8 byte列へ変換し、CRを値から除く。\
raw string内のLFは値に含まれ、閉じbacktickの欠落はcompile errorとなる。\
escapeによるNULとU+FEFFの表現は成立する。\
import pathとtrap reasonではdecode結果がUTF-8であることを追加の成立条件とする。

文字分類の規範版はUnicode 16.0.0である。

## Semicolonと改行
<!-- ond-spec-id: syntax/semicolons-and-line-breaking -->

各章のEBNFはsemicolonを明示したtoken列を記述する。\
sourceの改行またはEOFで、最後のtokenがidentifier、integer/float/string literal、`true`、`false`、`nil`、`break`、`continue`、`return`、`++`、`--`、`)`、`]`、`}`なら、その直後にsemicolonを挿入する。\
closing `)`または`}`直前のsemicolonは省略できる。\
行を継続するときはsemicolon挿入条件を満たさない位置で改行する。

## 文法の定義元
<!-- ond-spec-id: syntax/grammar -->

各productionは次の章で一度だけ定義する。\
EBNFの`{ X }`は0回以上、`[ X ]`は省略可能、`|`は選択、引用符内はtokenを表す。\
`identifier`、`int_lit`、`float_lit`、`string_lit`は本章で定める字句tokenである。

| Production群 | 定義元 |
| --- | --- |
| SourceFile、PackageClause、ImportDecl、QualifiedIdent | [package/import](./names-and-scopes.md#構文) |
| 宣言、receiver、signature、型parameter、IdentifierList | [宣言](./declarations.md#構文) |
| Type、型argument、関数型、TypeList | [型](./types.md#構文) |
| Block、Statement、assignment、if/for、return/defer/trap | [文](./statements.md#構文) |
| Expression、literal、call、conversion、ExpressionList | [式](./expressions.md#構文) |
