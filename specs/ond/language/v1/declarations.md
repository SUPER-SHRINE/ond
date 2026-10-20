# 宣言と関数
<!-- ond-spec-id: language/v1-declarations -->

本章は宣言、関数、call、const適格性を定義する。

## 構文
<!-- ond-spec-id: language/v1-declarations-grammar -->

次のEBNFは本章の構文を定義する。\
他章のproductionは[文法索引](./lexical.md#文法の定義元)を参照する。

```ebnf
TopLevelDecl  = ConstDecl | VarDecl | TypeDecl | FuncDecl | OperatorDecl .
OperatorDecl  = "operator" Receiver ( "+" | "-" | "*" | "/" | "%" | "[" "]" [ "=" ] | "len" ) Signature Block .
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
Signature     = "(" [ ParameterList ] ")" [ Result ] .
Result        = "->" Type | "->" "(" TypeList [ "," ] ")" .
ParameterList = ParameterDecl { "," ParameterDecl } [ "," ] .
ParameterDecl = identifier ":" Type .
IdentifierList= identifier { "," identifier } .
```

## 静的成立条件
<!-- ond-spec-id: language/v1-declarations-conditions -->

- `VarSpec` の `IdentifierList "=" ExpressionList` は初期値から型推論する
- `VarSpec` は右辺全体が 1 個の multi-value function call の場合、その戻り値を同数の左辺へ展開できる
- 関数宣言の `Signature` では parameter 名を必須とする
  - 関数型の `TypeSignature` では各 parameter 名を省略しても記載してもよい
  - 関数型の名前は説明用として解釈し、同一性はparameter/result型列で定める
- `Result` の括弧内には少なくとも1個の型が必要であり、`-> ()`はcompile errorとなる
  - 戻り値が0個の場合はarrowを省略する

## 宣言
<!-- ond-spec-id: language/declarations -->

再代入可能な束縛は `var`、定数は `const` で宣言する。

```ond
var count: i32 = 0
var limit = 10
const Width: u32 = 320
```

- 初期値がある宣言では型注釈を省略できる
- 推論する型は宣言時の初期値だけから決める
- 初期値がない `var` には型注釈が必要で、指定型の zero value で初期化する
- `const` にはコンパイル時に評価できる初期値が必要である
- 定数として許可する値は整数、`f32`、`bool`、文字列リテラルが生成するbyte配列を含む全要素が定数である配列・struct、および型が明示された `nil` とする
  - defined type にも underlying type の同じ規則を適用する
- 配列・struct の定数では入れ子の要素にも同じ条件を適用し、省略された要素は指定された要素型の zero value とする
  - 型付き aggregate 内の `nil` は field・要素型を型文脈として使用できる
- 定数の field・index 参照も compile-time に評価できる
  - 定数値をoperandとする演算・変換・field/index参照、型layout query、および定数配列を引数とする組み込み`len`のcallから定数式を構成する。call式のconst適格性はこの組み込み`len`に限る
  - 通常call、allocation、外部memory read、非`nil` pointer/function値を含むconst initializerはcompile errorとなる
- 配列・struct定数は実行時参照のための静的な読み取り専用実体を持つ
  - 定数自身のfield・要素はaddressableだが、定数を根とする代入はcompile errorとする
  - 値として`var`へ代入した場合は通常の値copyを行い、そのcopyは変更できる
- multi-value callは`var`初期値に展開できる
  - constの適格性は上記定数値規則に従う
- `:=` は関数内でのみ使える `var` 宣言の短縮形とする
- `:=` の左辺には未宣言の名前が少なくとも 1 個必要である
  - 既存の名前には代入する
- 再代入には `=`、または対応する二項演算と代入をまとめた `+=`、`-=`、`*=`、`/=`、`%=`、`&=`、`|=`、`^=`、`&^=`、`<<=`、`>>=` を使う
- `x op= y`の成立条件と評価順は[代入先ごとの規則](./statements.md#静的成立条件)に従う
- `x++` と `x--` はそれぞれ `x += 1` と `x -= 1` に相当する後置 statement であり、integer と `f32` に使える
  - statementとして実行する

```ond
value := 79
a, b := pair()
a, b = pair()
```

## 関数
<!-- ond-spec-id: functions/declarations-and-calls -->

関数はトップレベルで宣言する。

```ond
func split(ptr: *u8, length: i32) -> (*u8, i32) {
    return ptr, length
}
```

- 戻り値は固定個数で、0 個、1 個、または複数個を指定できる
- 複数の戻り値は関数のreturnとcall結果の値列として扱う
- `return` の値の個数と型は関数宣言に一致しなければならない
- 複数の戻り値は同数の左辺へ宣言または代入できる

function pointer 型は parameter 型と戻り値型を含む。

```ond
var callback: func(i32) -> i32 = update
var namedCallback: func(value: i32) -> i32 = update
```

- トップレベル関数名は、その関数と同じ signature の function pointer 値として使える
- 関数宣言は各parameterの名前を持ち、関数型は各parameter名を省略または説明用として記載できる
  - bindingは関数宣言のparameterが導入する
- 関数型の同一性は[型同一性](./types.md#型同一性)に従う
  - 上記の `callback` と `namedCallback` は同じ型である
- 戻り値がない型は arrow を省略し、複数戻り値は `-> (R1, R2)` と書く
- assignment と引数渡しでは signature が完全に一致しなければならない
- function pointer は通常の call 式で間接呼び出しできる
- nil function pointerの呼出しはnull addressへの間接callとなり、観測結果は[targetのfaultとmemory map規則](../../targets/kagura-v1/machine.md#faultと観測結果)に従う

Kagura target の関数呼び出し規約は [Ond ABI for Kagura v1](../../targets/kagura-v1/abi.md) に従う。

## 構文と静的条件の対応
<!-- ond-spec-id: syntax/grammar-notes -->

各構文の静的成立条件は、宣言について本章、型について[型章](./types.md)、式について[式章](./expressions.md)、文について[文の章](./statements.md)を定義元とする。\
文法に合致した入力にも各章の型、名前、値範囲、位置条件を適用し、不成立ならcompile errorとなる。

## 診断と成功結果
<!-- ond-spec-id: language/v1-diagnostics -->

sourceの名前、型、定数値、構文の成立条件を満たすprojectに対して成功結果を返す。\
sourceの不成立はsource位置付きのcompile errorとなり、独立した後続エラーも収集する。\
失敗した宣言名への依存だけで生じる派生診断は元の原因に対応づける。\
成功結果は全宣言の意味検査を完了した全体を表し、失敗したprojectはerrorを返す。\
source位置はUTF-8 byte範囲に対応づけ、表示位置は当該fileのline/columnで示す。\
generic具体化の主位置と関連位置は[generic規則](./types.md#型parameter)に従う。
