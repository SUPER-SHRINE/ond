# Ond Syntax v1

本書は `ond` v1 の字句規則、scope、構文、operator precedence を定義する。型、メモリ管理、組み込み関数、初期化順序などの意味規則は [Ond Language v1](./language.md) に従う。

記載していない構文は v1 ではサポートしない。

## 1. Source Form

- source file は UTF-8 text とする。
- 改行は `LF` または `CRLF` を許可する。意味上はすべて `LF` として扱う。
- 空白文字は space、tab、newline とする。
- line comment は `//` から行末までとし、空白と同様に無視する。
- block comment は `/*` と `*/` で囲み、空白と同様に無視する。入れ子は許可しない。
- block comment が 1 個以上の改行を含む場合、その comment は semicolon insertion 上 1 個の newline として扱う。
- `*/` で閉じていない block comment は compile error とする。
- identifier、integer literal、float literal、string literal の字句規則は Go と同じとする。ただし imaginary literal は持たない。公開判定は identifier の先頭 byte が ASCII 大文字かどうかだけで決める。

具体的には [Go の字句規則](https://go.dev/ref/spec#Lexical_elements) に従って次を扱う。Ond にない rune literal、operator、keyword を追加するものではない。

- identifier の先頭は Unicode の Letter カテゴリ（Lu/Ll/Lt/Lm/Lo）または `_`、後続はそれらに加えて Decimal Number（Nd）を許可する。結合文字やその他の数字カテゴリは許可せず、Unicode 正規化もしない。
- integer literal は10進、`0b`/`0B`の2進、先頭`0`または`0o`/`0O`の8進、`0x`/`0X`の16進を許可する。`_`は基数prefix直後または数字間だけに置ける。基数に合わない数字や連続・末尾の`_`はエラーとする。
- decimal float は `.5`、`1.`、`1.e2`、`1e-2`を含む。hexadecimal float は`0x`/`0X`で始め、`p`/`P`と10進指数を必須とする。指数の符号と数字間の`_`を許可する。
- quoted string のescapeはbyte値とUnicode scalarの範囲を検査する。raw stringはescapeを解釈せず、値からCRを除去する。quoted string内のCRは保持するが、LFは許可しない。
- source先頭のUTF-8 BOMは無視する。途中のBOMとsourceに直接含まれるNULはエラーとする。文字列のescapeによるNULやU+FEFFの表現は許可する。

## 2. Semicolons and Line Breaking

Ond の文法は semicolon 付きで定義し、source では自動挿入する。

- 行末の最終 token が identifier、integer literal、float literal、string literal、`true`、`false`、`nil`、`break`、`continue`、`return`、`++`、`--`、`)`、`]`、`}` のいずれかなら、その直後に semicolon を挿入する。
- ファイル末尾にも同じ挿入規則を適用する。末尾の改行は必須ではない。
- closing `)` または `}` の直前の semicolon は省略できる。
- 明示的な行継続記号は持たない。改行したい場合は、semicolon が挿入されない位置で改行する。

## 3. Packages and Imports

各 file は先頭に 1 個の package clause を持つ。

- `ond.toml` がある directory を project root とし、source package の探索起点とする。
- project root 自体は `main` package であり、`main.ond` を含まなければならない。project root 直下の全 `.ond` file は `package main` を宣言する。
- 1 directory を 1 package とし、directory 内の全 `.ond` file は同じ package 名を宣言しなければならない。
- project root 以外の package 名は directory 名と一致しなければならない。
- import path は project root または実装が提供する追加 library root からの相対 directory を、`/` 区切りで表した string literal とする。project root の `main` package は import できない。
- `import alias "path"` は明示的な import alias を宣言する。`.` import と grouped import は持たない。
- alias を省略した import は、その package の宣言名で参照する。

package-qualified name は `packageName.ExportedName` とする。

- `packageName` は import した package 名でなければならない。
- `ExportedName` は先頭が ASCII 大文字の top-level identifier でなければならない。
- package-qualified name は package value ではなく、import 先 package の top-level declaration を指す構文である。

## 4. Scope and Shadowing

- package-level declaration は package scope に属し、同一 package 内の全 file から見える。
- import 名の scope は宣言した file に限る。同じ package の別 file でも、使用する package は各 file で import する。
- 同一 import path は file ごとに異なる alias で宣言できる。
- 異なる import path が同じ参照名になる場合は compile error とし、少なくとも一方へ明示 alias を指定して区別する。
- import 名と package-level declaration が衝突した場合は compile error とする。
- 関数 parameter、method receiver、関数本体直下の local declaration は、その関数本体の block scope に属する。
- 内側の block で宣言した名前は、外側の block、import 名、package-level declaration を shadow できる。
- 同一 block 内で同名の `var`、`const`、parameter を重複宣言してはならない。
- 同一 package scope 内で同名の top-level declaration を重複宣言してはならない。
- method名はreceiverのbase named typeごとのnamespaceに属する。同じbase typeではvalue/pointer receiverを通して重複できないが、同名のtop-level関数とは衝突しない。
- `_` は blank identifier とし、宣言または代入の値を破棄する位置で使える。値として参照できず、binding を作らない。
- `:=` は現在の block に未宣言の `_` 以外の名前を 1 個以上含まなければならない。既存名の再利用は、その block 内で既に見えている変数に限る。
- local `var` と `const` の scope は宣言完了直後から始まる。package-level declaration の scope は package 全体とする。
- `if` の init で宣言した名前は condition、両 branch から見える。`for` の init で宣言した名前は condition、post、loop body から見える。
- 組み込み型名 `bool`, `u8`, `i8`, `u16`, `i16`, `u32`, `i32`, `f32` は shadow できない。`type`・`var`・`const`・`:=`・関数宣言・関数宣言のparameterでbindingを導入する名前として使用するとcompile errorとする。package/localの両scopeで禁止する。
- struct field名と関数型内の説明用parameter名は、型名を隠すbindingを作らないため、この禁止の対象外とする。型参照・field selector等では組み込み型名の綴りを通常どおり読み取れる。
- 組み込み名 `alloc`, `free`, `len`, `trap`, `sizeof`, `alignof`, `load32`, `store32`, `thisFile`, `thisLine`, `new` は binding として宣言できない。struct field 名など binding を作らない位置では使用できる。`str` は通常の識別子である。

## 5. Operator Precedence

高い方から低い方へ、expression の優先順位は次の通りとする。

| Level | Operators |
| --- | --- |
| 7 | primary postfix: selector, index, call, `as` |
| 6 | unary: `+`, `-`, `!`, `^`, `*`, `&` |
| 5 | `*`, `/`, `%`, `<<`, `>>`, `&`, `&^` |
| 4 | `+`, `-`, `|`, `^` |
| 3 | `==`, `!=`, `<`, `<=`, `>`, `>=` |
| 2 | `&&` |
| 1 | `||` |

- 同じ precedence level の binary operator は左結合とする。
- `as` は postfix conversion で、`x + y as u32` は `x + (y as u32)` と解釈する。

## 6. EBNF

次の EBNF で構文を定義する。`";"` は semicolon insertion 後の token を表す。

```ebnf
SourceFile    = PackageClause ";" { ImportDecl ";" } { TopLevelDecl ";" } .
PackageClause = "package" identifier .
ImportDecl    = "import" [ identifier ] string_lit .

TopLevelDecl  = ConstDecl | VarDecl | TypeDecl | FuncDecl | OperatorDecl .

OperatorDecl  = "operator" [ TypeParameters ] ( "+" | "-" | "*" | "/" | "%" | "[]" | "[]=" | "len" ) Signature Block .

ConstDecl     = "const" ConstSpec | "const" "(" { ConstSpec ";" } ")" .
ConstSpec     = IdentifierList [ ":" Type ] "=" ExpressionList .

VarDecl       = "var" VarSpec | "var" "(" { VarSpec ";" } ")" .
VarSpec       = IdentifierList ":" Type [ "=" ExpressionList ]
              | IdentifierList "=" ExpressionList .

TypeDecl      = "type" TypeSpec | "type" "(" { TypeSpec ";" } ")" .
TypeSpec      = identifier [ TypeParameters ] Type .

FuncDecl      = "func" [ Receiver ] identifier [ TypeParameters ] Signature Block .
Receiver      = "(" identifier ":" ReceiverType ")" .
ReceiverType  = [ "*" ] ( AppliedType | NamedType ) .
TypeParameters= "[" identifier { "," identifier } "]" .
TypeArguments = "[" Type { "," Type } "]" .
Signature     = "(" [ ParameterList ] ")" [ Result ] .
Result        = "->" Type | "->" "(" TypeList [ "," ] ")" .
ParameterList = ParameterDecl { "," ParameterDecl } [ "," ] .
ParameterDecl = identifier ":" Type .

Type          = AppliedType | NamedType | PointerType | ArrayType | StructType | InterfaceType | FuncType .
NamedType     = identifier | QualifiedIdent .
AppliedType   = NamedType TypeArguments .
QualifiedIdent= identifier "." identifier .
PointerType   = "*" Type .
ArrayType     = "[" ConstExpression "]" Type .
StructType    = "struct" "{" { FieldDecl ";" } "}" .
FieldDecl     = identifier ":" Type .
InterfaceType = "interface" "{" { InterfaceMethod ";" } "}" .
InterfaceMethod = identifier TypeSignature .
FuncType      = "func" TypeSignature .
TypeSignature = "(" [ TypeParameterList ] ")" [ Result ] .
TypeParameterList = TypeParameter { "," TypeParameter } [ "," ] .
TypeParameter = [ identifier ":" ] Type .

Block         = "{" { Statement ";" } "}" .
Statement     = DeclStmt
              | SimpleStmt
              | ReturnStmt
              | TrapStmt
              | DeferStmt
              | BreakStmt
              | ContinueStmt
              | IfStmt
              | ForStmt
              | Block .

DeclStmt      = ConstDecl | VarDecl .
SimpleStmt    = EmptyStmt | ExpressionStmt | Assignment | IncDecStmt | ShortVarDecl .
EmptyStmt     = .
ExpressionStmt= CallExpr .
Assignment    = ExpressionList "=" ExpressionList
              | Expression AssignOp Expression .
AssignOp      = "+=" | "-=" | "*=" | "/=" | "%="
              | "&=" | "|=" | "^=" | "&^=" | "<<=" | ">>=" .
IncDecStmt    = Expression ( "++" | "--" ) .
ShortVarDecl  = IdentifierList ":=" ExpressionList .
ReturnStmt    = "return" [ ExpressionList ] .
TrapStmt      = "trap" string_lit .
DeferStmt     = "defer" Block .
BreakStmt     = "break" .
ContinueStmt  = "continue" .
IfStmt        = "if" [ SimpleStmt ";" ] Expression Block [ "else" ( IfStmt | Block ) ] .
ForStmt       = "for" ( Block | ConditionBlock | ForClauseBlock ) .
ConditionBlock= Expression Block .
ForClauseBlock= [ SimpleStmt ] ";" [ Expression ] ";" [ SimpleStmt ] Block .

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

PrimaryExpr   = Operand { Selector | Index | Arguments | Conversion } .
Selector      = "." identifier .
Index         = "[" Expression "]" .
Arguments     = "(" [ ExpressionList [ "," ] ] ")" .
Conversion    = "as" Type .

Operand       = Literal | NewExpr | identifier | QualifiedIdent | "(" Expression ")" .
NewExpr       = "new" "(" Type ")" .
Literal       = BasicLit | CompositeLit .
BasicLit      = int_lit | float_lit | string_lit | "true" | "false" | "nil" .
CompositeLit  = LiteralType "{" [ ElementList [ "," ] ] "}" .
LiteralType   = ArrayType | InferredArrayType | AppliedType | NamedType .
InferredArrayType = "[" "..." "]" Type .
ElementList   = Element { "," Element } .
Element       = [ Key ":" ] Expression .
Key           = Expression .

ExpressionList= Expression { "," Expression } .
IdentifierList= identifier { "," identifier } .
TypeList      = Type { "," Type } .
ConstExpression = Expression .
CallExpr      = PrimaryExpr Arguments .
```

- `Receiver` を持つ `FuncDecl` はmethod宣言であり、receiverのnamed typeを宣言したpackage内だけに置ける。
- `InterfaceType` は `TypeSpec` の直下で名前付き型としてだけ宣言できる。無名interface型は構文上parseできても意味検査で拒否する。

## 7. Grammar Notes

- `ForStmt` の `for Block` は無条件 loop を表す。
- `DeferStmt` は `for` の body または別の `DeferStmt` の block に構文上包含されてはならない。
- `DeferStmt` の block は `ReturnStmt` を含んではならない。block 内にネストした `for` を対象とする `break` と `continue` は許可する。
- `ExpressionStmt` は call expression だけを許可する。副作用のない expression を単独文として置けない。
- `TrapStmt`のstring literalはUTF-8としてdecodeできなければならず、通常のexpressionや値ではない。
- `VarSpec` の `IdentifierList "=" ExpressionList` は初期値から型推論する。
- `VarSpec` は右辺全体が 1 個の multi-value function call の場合、その戻り値を同数の左辺へ展開できる。
- `ConstSpec` の右辺はcompile-time evaluableでなければならない。通常のfunction callは短絡評価の未評価側にあっても許可しない（例: `false && f()`）。組み込み `len` は例外で、定数文字列のbyte長と配列の型から決まる長さを定数として使える。
- 定数式も `&&` / `||` の短絡評価に従う。`false && (1 / 0 == 0)` の右辺の除算は評価しない。`len(array)` の operand は通常どおり一度評価するため、runtime callを含む場合は定数式として使えない。未評価部分も名前解決・型検査を行い、型不一致はcompile errorとする。評価される定数変換の範囲外もcompile errorとする。
- 左辺の定数値により短絡で未評価と決まる右辺では、明示castの値の評価と範囲検査も省略する。`false && ((250 + 6) as u8 == 0)` と `true || ((250 + 6) as u8 == 0)` は受理する。未定義名・不正な型変換・constでの通常call禁止は未評価側にも適用する。この非評価規則はconst宣言だけでなく、通常の値を求める式にも共通とする。
- `ArrayType` の長さは compile-time constant でなければならない。
- 型・関数の型parameter listは宣言名へ空白を挟まず `Box[T]`、`Identity[T]` と書く。これは既存の配列型宣言 `type Buffer [N]u8` と区別するための規則である。
- `TypeSpec`の型parameter listはstructだけでなくinterfaceにも使える。generic型のmethod receiverは`func (box: *Box[T]) Set(value: T)`のように適用型として書き、method名の後ろに追加の型parameter listは書かない。
- generic operatorは `operator[T] [](values: *Vec[T], index: u32) -> *T` のように、`operator`へ空白を挟まず型parameter listを書いてからoperator名を書く。
- generic関数callはnamed functionに明示的な型argumentを付けて `Identity[u32](value)` と書く。型argument推論は行わない。同じtoken列になり得る `callbacks[index]()` は、`callbacks` がgeneric関数名でなければ通常のindex accessとcallとして解釈する。
- 関数宣言の `Signature` では parameter 名を必須とする。関数型の `TypeSignature` では各 parameter 名を省略しても記載してもよい。関数型に記載した名前は binding を作らず、型の同一性に影響しない。
- `Result` の括弧内には少なくとも1個の型が必要であり、`-> ()` は許可しない。戻り値が0個の場合はarrowを省略する。
- `InferredArrayType` は composite literal の型指定でのみ使用でき、変数の型注釈や `new` の型引数には使用できない。長さは指定された最大 index に 1 を加えた値とし、要素が空の場合は compile error とする。key の省略・重複・zero value の規則は通常の array literal と同じとする。
- `CompositeLit` の `Key` は array literal では compile-time integer、struct literal では field 名とする。struct literal は field 名付き形式だけを許可する。
- `NamedType` と `QualifiedIdent` は type 名としても value 名としても文脈で解釈する。package-qualified name の右辺は exported な top-level 名でなければならない。
- `for init; cond; post {}` の `init` と `post` に使えるのは `SimpleStmt` のうち `ExpressionStmt`、`Assignment`、`IncDecStmt`、`ShortVarDecl` である。空欄は省略を表す。
- `if init; cond {}` の `init` に使えるのも同じである。
- 文法上 `Expression` は任意の式を許すが、型検査で `if` と条件付き `for` の条件は `bool` に制限する。
- multi-value function call は `VarSpec`、`Assignment`、`ShortVarDecl`、`ReturnStmt` で使える。v1 では `ConstSpec` や他の call の argument 位置へそのまま渡せない。
- assignment の左辺は `_`、変数、参照外し、配列または raw pointer の index、struct field のいずれかで、書き込み可能でなければならない。
- assignment は最初に左辺の index と pointer operand、および右辺を source 上の左から右へ評価し、その後で左辺への書き込みを左から右へ行う。
- compound assignment の左辺は `_` 以外の書き込み可能な式 1 個、右辺は式 1 個とする。左辺の index、pointer operand、および現在値は 1 回だけ評価する。
- `++` と `--` は後置の statement だけとし、前置形式や値を生成する expression としては使えない。
- 戻り値を持つ関数は、到達可能なすべての実行経路で宣言と一致する `return` に到達しなければならない。無条件 `for {}` は、外へ到達する `break` を含まない場合に限り終端とみなす。
