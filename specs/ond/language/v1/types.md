# 型と値構造
<!-- ond-spec-id: language/v1-types -->

本章は型、型同一性、pointer、array、struct、methodを定義する。

## 構文
<!-- ond-spec-id: language/v1-types-grammar -->

次のEBNFは本章の構文を定義する。\
他章のproductionは[文法索引](./lexical.md#文法の定義元)を参照する。

```ebnf
TypeArguments = "[" Type { "," Type } "]" .
Type          = AppliedType | NamedType | PointerType | ArrayType | StructType | FuncType .
NamedType     = identifier | QualifiedIdent .
AppliedType   = NamedType TypeArguments .
PointerType   = "*" Type .
ArrayType     = "[" ConstExpression "]" Type .
StructType    = "struct" "{" { FieldDecl ";" } "}" .
FieldDecl     = identifier ":" Type .
FuncType      = "func" TypeSignature .
TypeSignature = "(" [ TypeParameterList ] ")" [ Result ] .
TypeParameterList = TypeParameter { "," TypeParameter } [ "," ] .
TypeParameter = [ identifier ":" ] Type .
TypeList      = Type { "," Type } .
```

## 静的成立条件
<!-- ond-spec-id: language/v1-types-conditions -->

- `NamedType` と `QualifiedIdent` は type 名としても value 名としても文脈で解釈する
  - package-qualified name の右辺は exported な top-level 名でなければならない

## 型
<!-- ond-spec-id: type-system/types -->

v1の型は以下の組み込み型と構成型である。

| 整数型 | bit幅 | 値の範囲 |
| --- | ---: | --- |
| `u8` | 8 | 0〜255 |
| `i8` | 8 | -128〜127 |
| `u16` | 16 | 0〜65535 |
| `i16` | 16 | -32768〜32767 |
| `u32` | 32 | 0〜4294967295 |
| `i32` | 32 | -2147483648〜2147483647 |

- `u8`, `u16`, `u32`
- `i8`, `i16`, `i32`
- `f32`
- `bool`
- `*T`
- `func(T1, T2) -> R`
- `[N]T`
- `struct`

関数が値を返さない場合はarrowと戻り値型を書かない。\
空の戻り値リスト`-> ()`は関数宣言・関数型ともcompile errorとなる。

- `type Name T`は、underlying typeを`T`とする宣言固有のdefined typeを導入する
- defined type は、同じ underlying type を持つ他の型とも異なる型である
- defined typeはunderlying typeのzero valueとmemory layoutを引き継ぐ
  - 演算は[custom優先・builtin継承の解決規則](./expressions.md#user-defined-operator)に従う
- assignment、argument、returnは要求型との同一性を成立条件とする
  - defined typeとunderlying typeの値を渡す場合もこの条件で検査する
- defined type とその underlying type、または同一の underlying type を持つ defined type の間は `value as T` で明示変換できる
- direct recursive type は compile error とし、pointer を介した recursive type は許可する
- struct field の先頭が ASCII 大文字なら他 package から参照でき、それ以外は宣言 package 内だけから参照できる

### 型parameter
<!-- ond-spec-id: type-system/type-parameters -->

型とtop-level関数は、明示的な型parameterを持てる。\
generic型のmethodは、receiverの型argument位置でmethod内の型parameterを宣言する。

```ond
type Box[T] struct { value: T }

func Identity[T](value: T) -> T {
    return value
}

func (box: *Box[Element]) Set(value: Element) {
    box.value = value
}

box := Box[u32]{value: Identity[u32](7)}
```

- 型parameterは型を表すidentifierであり、同じ宣言内で相異なる名前を持つ
- generic型とtop-level関数の使用時は型parameter数と一致する型argument列を明示する
  - generic methodは具体的なreceiver型から型argumentを推論する
- generic型は型argument列ごとに具体化する
  - 同じ宣言と同じ型argument列は同じdefined typeを表し、異なる型argument列は異なるdefined typeを表す
- generic関数も型argument列ごとに具体化し、通常の静的関数callとして実行する
  - function pointer値に使うgeneric関数は型argumentを明示して具体化する
- 宣言名と型parameter listの間には空白を入れない
  - `type Box[T] ...` はgeneric型宣言、`type Buffer [N]u8` は長さ`N`の配列をunderlying typeとする通常の型宣言である
- packageを跨ぐ参照には通常の公開規則を適用する
  - 宣言名と、外部から直接参照するfieldなどはASCII大文字で始める
- generic methodのreceiverは、generic型の各型argument位置へfreshで相異なる識別子を1個ずつ書き、それらがmethod内の型parameterを導入する
  - receiver側の型parameter名は元の型宣言と独立して選べる
- `operator`宣言の型parameterは[operator規則](./expressions.md#user-defined-operator)に従う
- generic関数、generic method、generic operatorの本体は、型argumentに依存しない値名と字句scopeを宣言時に検査する
  - 一度も具体化されない宣言でも未定義名はcompile errorとする
- generic関数、generic method、generic operatorの型argumentに依存する本体エラーは、失敗した定義内の式を主位置、具体化を要求した使用箇所と入れ子の具体化箇所を関連位置として報告する
  - 同じ具体化の同じ定義エラーを複数箇所が要求した場合は、1個の診断に各使用箇所を関連位置として付ける
  - 独立した別の具体化失敗は続けて検査する
- generic型のunderlying typeを具体化して初めて判明するエラーは、その適用型を要求した箇所を主位置、失敗したgeneric型定義を関連位置として報告する

### 型同一性
<!-- ond-spec-id: type-system/type-identity -->

- 型の同一性は以下の宣言identityと構造により定まる
- defined typeは各宣言に固有の型であり、構造が一致しても別の宣言の型とは同一にならない
- 無名のstruct型はfieldの個数・順序・名前・型が一致する場合に同一となる
  - 非公開fieldの名前には宣言packageの同一性も含める
  - 公開fieldの名前はpackageに依存しない
- 配列型は長さと要素型、pointer型は参照先の型が一致する場合に同一となる
  - 推論長配列も長さ確定後は同じ規則を使う
- 関数型の同一性はparameter型の列と戻り値型の列により定まる
- 要素型・field型・参照先型・parameter型・戻り値型の比較では、defined typeの区別を維持する
  - 比較する各構成型に宣言固有identityを適用する

整数リテラルの既定型は `i32` とする。\
整数型が文脈から決まる場合、その型で表現できるリテラルだけを受理する。\
浮動小数点リテラルの既定型は `f32` とする。\
字句は[字句章](./lexical.md#source形式)に従い、compile-time に `f32` で表現できない値は compile error とする。

型変換は `value as T` と書く。\
次の変換には明示的な型変換が必要である。

- 異なる整数型の間
- 整数型と `f32` の間
- 整数とポインタの間

評価される定数変換の結果を表現できない場合はコンパイルエラーとする。\
短絡評価により値が評価されない部分では、定数変換の範囲検査も省略する。\
名前解決と型検査は両辺について行う。\
実行時の整数間変換は、source型の幅と符号で解釈した数学的整数値vに対し、変換先の幅Mのbit patternをv mod 2^M（0以上2^M未満）とする。\
結果値はこのbit patternを変換先型の符号で解釈し、[整数表現](./expressions.md#integer-arithmetic)に従う。\
整数からf32への変換は整数の数学値nに対し、n<0ならs=1、他はs=0、m=abs(n)として[binary32のR(s,m)](./expressions.md#floating-point-arithmetic)を1回適用する。\
f32から整数は、有限値を0方向へ切り捨てた整数nを先に求め、そのnが変換先型の範囲内ならnを返す。\
NaN、Inf、またはnが範囲外の場合はcompile-timeならcompile error、runtimeならtargetのfaultとなる。

Kagura target における型のサイズ、alignment、zero value は [Ond for Kagura Machine Profile v1](../../targets/kagura-v1/machine.md) に従う。

## `nil`とpointer
<!-- ond-spec-id: type-system/nil-and-pointers -->

- `nil` は任意の `*T`、function pointer に代入できる
- `nil`はpointerまたはfunction型を与える文脈を成立条件とし、`value := nil`のように型文脈がない入力はcompile errorとなる
- `*pointer` で参照外しを行う
- [operator解決](./expressions.md#user-defined-operator)でbuiltin raw pointer indexを選んだ場合、`*T`の`pointer[index]`はindexをu32とするunchecked element accessである
- pointerの有効領域と生存期間は利用側が管理し、access結果は[targetのraw pointer規則](../../targets/kagura-v1/machine.md#6-raw-pointer-access)で定まる
- 配列要素と struct field へのアクセスに必要なアドレス計算はコンパイラが生成する

`*T`はaddress値であり、allocationの所有と解放は[実行契約](./execution.md#メモリ管理)に従い利用側が管理する。

## 配列とstruct
<!-- ond-spec-id: type-system/arrays-and-structs -->

- `[N]T` は長さ `N` の固定長配列である
- 固定長配列と推論長配列に共通して、長さ `N` は整数で `0 <= N <= 2^32-1` とする
  - 配列要素の有効な index は `0 <= index < N` とする
  - `N = 0` の配列に有効な index はない
- Kagura targetの配列型のbyte size上限、適用箇所、paddingとゼロサイズの扱いは[Data Layout](../../targets/kagura-v1/machine.md#5-data-layout)に従う
- 配列の長さはコンパイル時定数でなければならない
- 配列のindex演算は[operator解決](./expressions.md#user-defined-operator)を先に行い、builtinを選んだ場合に以下の配列access規則を適用する
- builtin配列アクセスは`array[index]`とする
- index は integer でなければならない
- 配列長取得は[builtin len](./builtins.md#41-配列に対する-len)に従う
- 定数 index が `0 <= index < N` を満たさない場合はコンパイルエラーとする
- 実行時に決まる index には bounds check を生成し、範囲外なら回復不能なtrapで停止する
- 配列読取り、address取得と代入のbounds check時点は[評価と代入の順序](./statements.md#静的成立条件)に従う
- compiler は index が範囲内だと証明できる場合だけ runtime bounds check を省略できる
- struct は `type Name struct { field: T }` と宣言し、`value.field` で field へアクセスする
- 配列と struct の代入および引数渡しは値コピーとする

array と struct は次の composite literal を持つ。

```ond
point := Point{x: 10, y: 20}
values := [4]u32{10, 20, 30, 40}
lookup := [8]u32{2: 100, 5: 200}
inferred := [...]u32{10, 20, 30}
```

- struct literal は `T{field: value}` の field 名付き形式だけを許可する
- array literal は key のない要素と、compile-time integer index を key とする `index: value` を許可する
  - 最初の key なし要素の index は 0、以降は直前の要素の index に 1 を加えた値とする
- `[...]T` の長さは最大 index に 1 を加えた値とし、その長さが `2^32-1` を超える場合は compile error とする
  - 要素がない `[...]T{}` は compile error とする
- field、index の重複と、固定長配列の範囲外 index は compile error とする
- 指定されなかった field と要素は zero value とする
- 初期化式は source 上の左から右へ 1 回ずつ評価する
- nested composite literalもそれぞれ型を明示する
- composite literalは指定型の値を生成し、[heap確保](./execution.md#メモリ管理)は`new`/`alloc`の形式で行う
- address取得の対象はaddressableなstorageを持つ値であり、`&T{...}`はcompile errorとなる

### Method
<!-- ond-spec-id: type-system/methods -->

methodはnamed typeに結び付いた通常関数の糖衣構文である。

```ond
type Counter struct { value: i32 }
func (counter: Counter) Value() -> i32 { return counter.value }
func (counter: *Counter) Add(amount: i32) { counter.value += amount }

type Box[T] struct { value: T }
func (box: *Box[Element]) Set(value: Element) { box.value = value }
```

- receiverは `T`、`*T`、generic型の `T[P1, ...]`、または `*T[P1, ...]` で、`T`を宣言したpackage内でだけ宣言できる
  - `T`のunderlying typeはstructに限らない
- generic methodの型parameterはreceiverの具体型から位置ごとに推論し、その型argument列ごとに静的関数へ具体化する
  - method本体の値名と字句scopeは宣言時に検査し、型argumentに依存する型検査は具体化時に行う
- value receiverは値をcopyし、pointer receiverはpointer値を渡す
  - method呼出しでは完全一致、addressableなreceiverの1段のaddress取得、pointer receiverの1段の参照外しの順で候補を選ぶ
  - 通常のfieldアクセスと明示変換はそれぞれの規則で解決する
- `nil`のpointer receiverも呼び出せる
  - 呼出し自体はfaultせず、method内でそのpointerを参照外しした時点で通常のraw pointer規則を適用する
- 同じbase named typeではvalue/pointer receiverを通してmethod名を一意とする
  - field名とmethod名は相異なる
  - method名のnamespaceはtop-level関数名のnamespaceから独立する
- method名の先頭がASCII大文字なら他packageから呼べる
  - 非公開named typeの値でも、公開APIから型推論で得られた値に対する公開method呼出しは許可する
- `receiver.Method(arguments...)` はreceiverを1回評価した後、argumentsを左から右へ1回ずつ評価する
- method selectorは上記のmethod callのcalleeとして使う
  - callを伴わないmethod値と`Type.Method`による関数値参照はcompile errorとなる
  - function pointer値として渡す関数は[トップレベル関数](./declarations.md#関数)の規則に従う
