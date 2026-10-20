# 式、演算、定数評価
<!-- ond-spec-id: language/v1-expressions -->

本章は式、演算子の優先順位、評価順、定数評価、文字列値を定義する。

## 構文
<!-- ond-spec-id: language/v1-expressions-grammar -->

次のEBNFは本章の構文を定義する。\
他章のproductionは[文法索引](./lexical.md#文法の定義元)を参照する。

```ebnf
Expression    = OrExpr .
OrExpr        = AndExpr { "||" AndExpr } .
AndExpr       = CompareExpr { "&&" CompareExpr } .
CompareExpr   = AddExpr { CompareOp AddExpr } .
CompareOp     = "==" | "!=" | "<" | "<=" | ">" | ">=" .
AddExpr       = MulExpr { AddOp MulExpr } .
AddOp         = "+" | "-" | "|" | "^" .
MulExpr       = UnaryExpr { MulOp UnaryExpr } .
MulOp         = "*" | "/" | "%" | "<<" | ">>" | "&" | "&^" .
UnaryExpr     = PrimaryExpr | UnaryOp UnaryExpr .
UnaryOp       = "+" | "-" | "!" | "^" | "*" | "&" .
PrimaryExpr   = Operand { Selector | Index | TypeApplication | Arguments | Conversion } .
TypeApplication = TypeArguments .
Selector      = "." identifier .
Index         = "[" Expression "]" .
Arguments     = "(" [ ExpressionList [ "," ] ] ")" .
Conversion    = "as" Type .
Operand       = Literal | NewExpr | LayoutExpr | identifier | QualifiedIdent | "(" Expression ")" .
NewExpr       = "new" "(" Type ")" .
LayoutExpr    = ( "sizeof" | "alignof" ) "(" Type ")" .
Literal       = BasicLit | CompositeLit .
BasicLit      = int_lit | float_lit | string_lit | "true" | "false" | "nil" .
CompositeLit  = LiteralType "{" [ ElementList [ "," ] ] "}" .
LiteralType   = ArrayType | InferredArrayType | AppliedType | NamedType .
InferredArrayType = "[" "..." "]" Type .
ElementList   = Element { "," Element } .
Element       = [ Key ":" ] Expression .
Key           = Expression .
ExpressionList= Expression { "," Expression } .
ConstExpression = Expression .
CallExpr      = PrimaryExpr Arguments .
```

## 静的成立条件
<!-- ond-spec-id: language/v1-expressions-conditions -->

- generic関数callはnamed functionに明示的な型argumentを付けて `Identity[u32](value)` と書く
  - 宣言型parameterと個数が一致する型argument列を明示する
  - `TypeApplication`はgeneric関数名へ適用し、型argumentを`Type`として解釈する
  - 例は`Identity[*u8](pointer)`である
  - 同じtoken列になり得る`callbacks[index]()`は`callbacks`がgeneric関数名なら型適用、それ以外は通常のindex accessとcallとして解釈する

## 式と評価順
<!-- ond-spec-id: expressions/overview -->

v1 は次の式を持つ。\
operand の型が適合しない式はコンパイルエラーとする。

- integer、`f32`、`true`、`false`、`nil`、文字列、array、struct の literal
- 関数 call、method call、配列 index、raw pointer index、struct field access
- addressable な変数、配列要素、struct field に対する `&value` と、`*pointer` による参照外し
- integer の単項 `+`, `-`, `^`
- `f32` の単項 `+`, `-`
- integer の `+`, `-`, `*`, `/`, `%`, `&`, `|`, `^`, `&^`, `<<`, `>>`
- `f32` の `+`, `-`, `*`, `/`
- 同じ integer 型、`bool`、同じ raw pointer 型、同じ function pointer 型に対する `==`, `!=`
- `f32` に対する `==`, `!=`, `<`, `<=`, `>`, `>=`
- integer に対する `<`, `<=`, `>`, `>=`
- `bool` に対する `!`, `&&`, `||`

短絡演算を除くoperandと関数argumentはsource順に左から右へ評価する。\
callはcalleeを先に評価し、次にargumentを左から右へ評価する。

| 演算子 | 左辺値 | 値評価対象 | 結果 |
| --- | --- | --- | --- |
| `&&` | `false` | 左辺 | `false` |
| `&&` | `true` | 左辺、右辺 | 右辺値 |
| <code>&#124;&#124;</code> | `true` | 左辺 | `true` |
| <code>&#124;&#124;</code> | `false` | 左辺、右辺 | 右辺値 |

名前解決と型検査は両辺に行い、値評価および定数cast範囲検査は表の値評価経路で行う。\
const適格性は式全体を検査し、通常callを含むconst initializerは短絡経路によらずcompile errorとなる。

## 演算子の優先順位
<!-- ond-spec-id: syntax/operator-precedence -->

高い方から低い方へ、expression の優先順位は次の通りとする。

| Level | Operators |
| --- | --- |
| 7 | primary postfix: selector, index, call, `as` |
| 6 | unary: `+`, `-`, `!`, `^`, `*`, `&` |
| 5 | `*`, `/`, `%`, `<<`, `>>`, `&`, `&^` |
| 4 | `+`, `-`, <code>&#124;</code>, `^` |
| 3 | `==`, `!=`, `<`, `<=`, `>`, `>=` |
| 2 | `&&` |
| 1 | <code>&#124;&#124;</code> |

- 同じ precedence level の binary operator は左結合とする
- `as` は postfix conversion で、`x + y as u32` は `x + (y as u32)` と解釈する

### Integer Arithmetic
<!-- ond-spec-id: expressions/integer-arithmetic -->

幅 `N` bit の integer に対する runtime 演算は、[ISO/IEC 10967-1:2012](https://www.iso.org/standard/51317.html) の modulo integer model に従う。

| 演算群 | operand型 | result型 |
| --- | --- | --- |
| 単項整数演算 `+`, `-`, `^` | `T` | `T` |
| 整数算術・bitwise二項演算 | `T`, `T` | `T` |
| shift | 左辺`T`、count`u32` | `T` |
| 整数比較 | 同一整数型`T`, `T` | `bool` |
- `+`, `-`, `*`, 単項 `-` は数学的結果を modulo `2^N` で wrap する
- 整数の`/`と`%`は上表の同一整数型operandに適用する
- quotient は 0 方向へ丸める
- remainder は `a == (a / b) * b + (a % b)` を満たし、`0` でない場合は被除数 `a` と同じ符号を持つ
- `b == 0` の `/` と `%` は compile-time なら compile error、実行時なら fault とする
- signed division の `minInt / -1` は compile-time なら compile error、実行時なら fault とする
  - 対応する remainder `minInt % -1` は `0` とする
- bitwise 演算は `N` bit の表現に対して行う
- signed comparison は two's-complement の符号付き値、unsigned comparison は符号なし値を比較する
- shift の左 operand と結果は同じ integer 型とする
- shift count の型は `u32` とする
  - integer literal はこの文脈で `u32` として扱う
- shift count `k` の実効値は `k mod N` とする
- left shift は上位 bit を破棄して `N` bit に wrap する
- unsigned right shift は上位を `0`、signed right shift は元の最上位 bit で埋める

runtime shift は [WebAssembly integer semantics](https://webassembly.github.io/spec/core/exec/numerics.html) と同じ規則とする。

### Integer Constant Arithmetic
<!-- ond-spec-id: expressions/integer-constant-arithmetic -->

整数定数部分式は数学的な正確値で計算する。\
中間値はsource整数型の幅とcompilerの固定幅から独立して保持する。\
literal既定型、型文脈とoperand型一致は[型規則](./types.md#型同一性)に従う。

宣言・代入・argument・return・明示cast等の値要求境界で、正確値が要求整数型の範囲に入ることを検査し、範囲外ならcompile errorを返す。\
配列lengthとindexは[配列規則](./types.md#配列とstruct)、shift countは`u32`の成立条件で検査する。\
定数shiftの実効countは左operandの幅`N`に対する`k mod N`であり、left shiftは数学的に左移動した結果を値要求境界で検査する。\
right shiftの符号規則はruntimeと同じで、zero divisionとsigned division overflowはcompile errorとなる。

| Source | 仕様から導出した結果 |
| --- | --- |
| `const X: u8 = (250 + 10) - 10` | 受理、値`250` |
| `const Y: u8 = (250 + 10) as u8` | cast位置で`260`が範囲外、compile error |
| `const Z: u8 = 250 + 10` | 宣言境界で`260`が範囲外、compile error |

```ond
const A: u8 = 1 << 8     // 実効countは0なので1
const B: u8 = 128 << 1   // 256はu8で表現できないためcompile error
const C: u8 = 250 + 10   // 260はu8で表現できないためcompile error
```

定数式としての意味は最適化の有無に依存しない。\
runtime 式を最適化で定数畳み込みする場合は、元のruntime式のwrap・fault規則を保持する。\
`f32` の定数演算は以下のbinary32規則に従い、整数の正確値計算とは区別する。

### Floating-Point Arithmetic
<!-- ond-spec-id: expressions/floating-point-arithmetic -->

- `f32` は IEEE 754 binary32 とする
- 丸めはroundTiesToEvenに固定する
- `+0` と `-0`、`+Inf` と `-Inf`、subnormal を保持する
- 演算結果の `NaN` は canonical quiet NaN `0x7fc00000` へ正規化する
- `==` はどちらかが `NaN` なら常に `false`、`!=` はどちらかが `NaN` なら常に `true`、`<`, `<=`, `>`, `>=` はどちらかが `NaN` なら常に `false` とする
- `f32`の`/`はIEEE 754に従い、zero divisionもそのbinary32値を返す
- compile-time の `f32` 定数評価は runtime と同じ結果 bit pattern を生成しなければならない

## 文字列リテラル
<!-- ond-spec-id: literals/strings -->

文字列literalはdecodeしたbyte配列値を表す。

- 字句とescapeのdecodeは[字句章](./lexical.md#source形式)に従う
- 通常のsource文字とUnicode escapeはUTF-8へencodeする
  - `\xNN`と3桁octal escapeは任意の1 byteを表せる
- decode後のbyte数を`N`として、リテラルの型は`[N]u8`とする
  - 長さはUnicode scalar数や表示文字数ではない
- `""`の型は`[0]u8`とする
- decode結果が論理値と型を定め、明示した`\000`も通常の配列要素としてbyte数と型に含める
- 代入、引数、return、global初期化では通常の型同一性規則に従い、配列長と要素型の完全一致を要求する
  - 型が一致しない入力はcompile errorとなる
- 値は通常の配列と同じくcopyされ、変数へ格納した後は要素を書き換えられる
- 文字列リテラルのbyte列は静的な読み取り専用実体として配置し、storage上では論理値の直後にhidden sentinel byte `0`を1個置く
  - sentinelは配列要素ではなく、`len`、型、代入、比較には含めない
- リテラルの要素はaddressableとする
  - 例えば`&"text"[0]`はプログラム実行中有効な`*u8`を生成し、raw pointerからは論理値直後のhidden sentinelを読み取れる
  - リテラルを根とする要素への直接代入はcompile errorとする
- 文字列リテラルを変数や別のaggregateへ格納する場合は論理値だけを通常の配列値としてcopyする
  - copy後の要素は書き換えられ、copy先のstorageは通常の配列layoutに従う
- 読み取り専用の文字列storageは、完全一致またはhidden sentinelを含むsuffix一致によって共有してよい
  - 各literalは論理byte列とsentinelが一致する実体を参照し、storage共有の選択は実装裁量とする
- `len(literal)`とindexは通常の配列規則をそのまま使用する
- 静的な文字列・定数実体から得たpointerの寿命はプログラム全体とする
  - 範囲と、raw pointerを介した読み取り専用領域への書き込み禁止はプログラマーが管理する

値としての文字列とimport pathのquoted/raw string構文は同じdecoderを共有するが、import pathは有効なUTF-8でなければならない構文要素であり、`[N]u8`値ではない。

`str`は通常の識別子として宣言できる。\
文字列・slice相当の型が必要なら、pointerや配列と長さを通常のstructおよびlibraryで組み合わせる。

## User-defined operator
<!-- ond-spec-id: functions/user-defined-operators -->

- top-levelの`operator`宣言はreceiver型に属する特殊名methodとして扱い、kindとoperand数により解決する
  - 通常の関数名とmethod名はそれぞれのnamespaceで一意とする
- 対象は unary `+` / `-`、binary `+` / `-` / `*` / `/` / `%`、`[]`、`[]=`、`len` とする
  - 宣言のoperator tokenはこの集合に属することを成立条件とする
- 解決はreceiver型と残りのoperand型で行い、戻り値型は選択後のsignature検査に用い、operatorは以下の演算構文で起動する
- receiverは同じpackageで宣言された名前付き型またはそのpointerに限る
- operator候補はoperandの静的な型に完全一致する宣言から選ぶ
- `[]` は値を 1 個返す
  - pointer を返すこともでき、その後の `.field` は通常の pointer field access と同様に暗黙 dereference する
  - operator index結果を直接address取得する`&collection[index]`はcompile errorとなる
- `[]=` はreceiver、index、valueを受け取り、値を返さない
- `len` は戻り値型に `u32` を1個明示し、`len(value)` は単一の `u32` を返す
  - 戻り値の省略、別の型、複数戻り値は未使用の宣言でも compile error とし、generic operatorにも同じ規則を適用する
- compound assignment は対応する binary operator と代入から導出し、左辺の base と index は一度だけ評価する
- 同じreceiver基底型ではoperator kindとoperand数が同じ宣言を重複できない
  - 重複判定はreceiver基底型、kindとoperand数を使い、単項と二項、`[]`と`[]=`を区別する
  - operator は import alias を介した明示参照なしに解決候補となる
- operator候補はreceiver基底の名前付き型を宣言したpackageだけから選ぶ
  - 呼出し元fileがそのpackageを直接importしているか、別package経由で値を得たか、import aliasを使うかには依存しない
- generic operatorは `operator (self: List[T]) [](index: u32) -> T` のように宣言する
  - receiver型引数は宣言型parameterと個数が一致するfreshで相異なる名前とし、これらがoperator内の型parameterを導入する
  - 使用時の型argumentはreceiverから決まる
- 呼出し時のreceiverだけはmethodと同様に1段の暗黙参照化または参照外しを許す
  - pointer receiverへ値を渡すにはaddressableな値が必要であり、残りのoperandは型の完全一致を要する
  - 型引数ごとに静的関数へ具体化する
