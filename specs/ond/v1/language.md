# Ond Language v1

Comment syntax for v1: `//` starts a line comment and `/* ... */` starts and ends a block comment. Comments are ignored like whitespace, block comments do not nest, and an unterminated block comment is a compile error.

`ond`（音頭）は Kagura runtime を主 target とする静的型付き言語である。ソースファイルの拡張子は `.ond` とする。Kagura 上の具体的な実行契約は [Ond for Kagura v1](../targets/kagura-v1/README.md) が定義する。

本書に記載した機能だけを v1 の言語機能とする。字句規則、scope、grammar、operator precedence は [Ond Syntax v1](./syntax.md) に従う。

## 1. ソースとパッケージ

- `ond.toml` がある directory を project root とする。
- project root 自体を `main` package とし、project root には `main.ond` が必須である。
- project root 直下の全 `.ond` file は `package main` を宣言する。
- 各ファイルは `package name` で所属パッケージを宣言する。
- 1 ディレクトリを 1 パッケージとし、ディレクトリ内の全 `.ond` ファイルは同じパッケージに属する。
- project root 以外のパッケージ名はディレクトリ名と一致しなければならない。
- `import` はパッケージ単位で行い、project root または実装が提供する追加 library root からの相対ディレクトリで解決する。project root の `main` package は import できない。
- `import alias "path"` で参照名を明示できる。省略時は import 先の package 宣言名を参照名とする。
- import 名の scope は宣言したファイルに限る。同じパッケージの別ファイルで使用する場合も、そのファイル自身が import を宣言する。
- 循環 import はコンパイルエラーとする。
- 識別子の先頭が ASCII 大文字なら他パッケージへ公開し、それ以外は所属パッケージ内だけへ公開する。

## 2. 宣言

再代入可能な束縛は `var`、定数は `const` で宣言する。

```ond
var count: i32 = 0
var limit = 10
const Width: u32 = 320
```

- 初期値がある宣言では型注釈を省略できる。
- 推論する型は宣言時の初期値だけから決める。
- 初期値がない `var` には型注釈が必要で、指定型の zero value で初期化する。
- `const` にはコンパイル時に評価できる初期値が必要である。
- 定数として許可する値は整数、`f32`、`bool`、文字列リテラルが生成するbyte配列を含む全要素が定数である配列・struct、および型が明示された `nil` とする。defined type にも underlying type の同じ規則を適用する。
- 配列・struct の定数では入れ子の要素にも同じ条件を適用し、省略された要素は指定された要素型の zero value とする。型付き aggregate 内の `nil` は field・要素型を型文脈として使用できる。
- 定数の field・index 参照も compile-time に評価できる。通常の関数呼び出し、allocation、外部のメモリ読み出し、非 `nil` の pointer・function pointer 値は定数に含めない。
- 配列・struct定数は実行時参照のための静的な読み取り専用実体を持つ。定数自身のfield・要素はaddressableだが、定数を根とする代入はcompile errorとする。値として`var`へ代入した場合は通常の値copyを行い、そのcopyは変更できる。
- multi-value function call は `var` 宣言の初期値に使えるが、通常の function call は `const` 宣言に使えない。
- `:=` は関数内でのみ使える `var` 宣言の短縮形とする。
- `:=` の左辺には未宣言の名前が少なくとも 1 個必要である。既存の名前には代入する。
- 再代入には `=`、または対応する二項演算と代入をまとめた `+=`、`-=`、`*=`、`/=`、`%=`、`&=`、`|=`、`^=`、`&^=`、`<<=`、`>>=` を使う。
- `x op= y` は `x op y` が有効で、その結果型が `x` の型と一致する場合に使える。`x` は 1 回だけ評価する。
- `x++` と `x--` はそれぞれ `x += 1` と `x -= 1` に相当する後置 statement であり、integer と `f32` に使える。値は生成せず、前置形式は持たない。

```ond
value := 79
a, b := pair()
a, b = pair()
```

## 3. 型

v1 は次の型を持つ。

- `u8`, `u16`, `u32`
- `i8`, `i16`, `i32`
- `f32`
- `bool`
- `*T`
- `func(T1, T2) -> R`
- `[N]T`
- `struct`
- 名前付き `interface`

関数が値を返さない場合はarrowと戻り値型を書かない。空の戻り値リスト `-> ()` は文法エラーとする（関数宣言・関数型の両方）。

- `type Name T` は、underlying type を `T` とする新しい defined type を宣言する。type alias は持たない。
- defined type は、同じ underlying type を持つ他の型とも異なる型である。
- defined type は underlying type で使用できる演算、zero value、memory layout を引き継ぐ。
- assignment、argument、return では型が一致しなければならず、defined type とその underlying type の間でも暗黙変換しない。
- defined type とその underlying type、または同一の underlying type を持つ defined type の間は `value as T` で明示変換できる。
- direct recursive type は compile error とし、pointer を介した recursive type は許可する。
- struct field の先頭が ASCII 大文字なら他 package から参照でき、それ以外は宣言 package 内だけから参照できる。

### 3.1 型parameter

型とtop-level関数は、明示的な型parameterを持てる。generic型のmethodは、receiverの型argument位置でmethod内の型parameterを宣言する。

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

- 型parameterは型だけを表し、constraint、既定値、可変長parameter listを持たない。同じ宣言内で名前を重複できない。
- generic型とtop-level関数の使用時は、すべての型argumentを明示する。generic methodだけは具体的なreceiver型から型argumentを推論する。部分適用は行わない。
- generic型は型argument列ごとに具体化する。同じ宣言と同じ型argument列は同じdefined typeを表し、異なる型argument列は異なるdefined typeを表す。
- generic型のunderlying typeにはstructだけでなくinterfaceも指定できる。`type List[T] interface { Get(index: u32) -> T }`のmethod signatureは、`List[u32]`の具体化時に`T`を`u32`へ置換する。
- generic関数も型argument列ごとに具体化し、通常の静的関数callとして実行する。具体化前のgeneric関数自体をfunction pointer値として取得できない。
- 宣言名と型parameter listの間には空白を入れない。`type Box[T] ...` はgeneric型宣言、`type Buffer [N]u8` は長さ`N`の配列をunderlying typeとする通常の型宣言である。
- packageを跨ぐ参照には通常の公開規則を適用する。宣言名と、外部から直接参照するfieldなどはASCII大文字で始める。
- generic methodのreceiverは、generic型の各型argument位置へfreshで重複しない識別子を1個ずつ書く。`Box[u32]`のような特定instanceだけへのmethod、receiver以外で宣言する追加のmethod型parameter、型argumentの省略は許可しない。receiver側の型parameter名は元の型宣言と同じでなくてよい。
- interface methodは型parameterを持たない。`operator`宣言は9.1節の規則で型parameterを持てる。
- generic関数、generic method、generic operatorの本体は、型argumentに依存しない値名と字句scopeを宣言時に検査する。一度も具体化されない宣言でも未定義名はcompile errorとする。
- generic関数、generic method、generic operatorの型argumentに依存する本体エラーは、失敗した定義内の式を主位置、具体化を要求した使用箇所と入れ子の具体化箇所を関連位置として報告する。同じ具体化の同じ定義エラーを複数箇所が要求した場合は、1個の診断に各使用箇所を関連位置として付ける。独立した別の具体化失敗は続けて検査する。
- generic型のunderlying typeを具体化して初めて判明するエラーは、その適用型を要求した箇所を主位置、失敗したgeneric型定義を関連位置として報告する。

### 型同一性

- 型の同一性はソース位置や記述ファイルに依存しない。
- defined typeは各宣言に固有の型であり、構造が一致しても別の宣言の型とは同一にならない。
- 無名のstruct型はfieldの個数・順序・名前・型が一致する場合に同一となる。非公開fieldの名前には宣言packageの同一性も含める。公開fieldの名前はpackageに依存しない。
- 配列型は長さと要素型、pointer型は参照先の型が一致する場合に同一となる。推論長配列も長さ確定後は同じ規則を使う。
- 関数型はparameter型の列と戻り値型の列で比較し、parameter名を比較しない。
- 要素型・field型・参照先型・parameter型・戻り値型の比較では、defined typeの区別を維持する。underlying typeを再帰的に展開して同一視しない。

整数リテラルの既定型は `i32` とする。整数型が文脈から決まる場合、その型で表現できるリテラルだけを受理する。
浮動小数点リテラルの既定型は `f32` とする。字句規則は Go の float literal と同じとし、compile-time に `f32` で表現できない値は compile error とする。

型変換は `value as T` と書く。次の変換には明示的な型変換が必要である。

- 異なる整数型の間
- 整数型と `f32` の間
- 整数とポインタの間

評価される定数変換の結果を表現できない場合はコンパイルエラーとする。短絡評価や配列の `len` により値が評価されない部分では、定数変換の範囲検査も省略する。ただし名前解決・型検査は省略しない。実行時の整数変換は変換先の幅の下位 bit を保持し、符号拡張またはゼロ拡張して変換する。
`f32` への変換は IEEE 754 binary32 の roundTiesToEven で丸める。`f32` から整数型への変換は 0 方向への丸めとし、`NaN`、無限大、変換先で表現できない値は compile-time なら compile error、実行時なら fault とする。

Kagura target における型のサイズ、alignment、zero value は [Ond for Kagura Machine Profile v1](../targets/kagura-v1/machine.md) に従う。

## 4. `nil` とポインタ

- `nil` は任意の `*T`、function pointer、interface に代入できる。
- `nil` は整数、配列、structには代入できない。
- 数値 `0` と `nil` は暗黙に変換しない。
- `nil` は既定型を持たないため、`value := nil` はコンパイルエラーとする。
- `*pointer` で参照外しを行う。
- `*T` に対する `pointer[index]` は、`index` を `u32` とする unchecked element access である。
- `pointer[index]` は allocation の範囲、pointer の寿命、`nil` を検査しない。
- raw pointer と integer に対する一般の加減算は提供しない。
- 配列要素と struct field へのアクセスに必要なアドレス計算はコンパイラが生成する。

`*T` は領域を所有せず、確保も解放も行わない。参照外しの実行規則は target の raw pointer 規則に従う。

## 5. 配列と struct

- `[N]T` は長さ `N` の固定長配列である。
- 配列の長さはコンパイル時定数でなければならない。
- 配列アクセスは `array[index]` とする。
- index は integer でなければならない。
- `len(array)` はoperandを通常どおり1回評価した後、配列長を表す `u32` の compile-time constant を返す。
- 定数 index が `0 <= index < N` を満たさない場合はコンパイルエラーとする。
- 実行時に決まる index には bounds check を生成し、範囲外なら回復不能なtrapで停止する。
- compiler は index が範囲内だと証明できる場合だけ runtime bounds check を省略できる。
- struct は `type Name struct { field: T }` と宣言し、`value.field` で field へアクセスする。
- 配列と struct の代入および引数渡しは値コピーとする。

array と struct は次の composite literal を持つ。

```ond
point := Point{x: 10, y: 20}
values := [4]u32{10, 20, 30, 40}
lookup := [8]u32{2: 100, 5: 200}
inferred := [...]u32{10, 20, 30}
```

- struct literal は `T{field: value}` の field 名付き形式だけを許可する。
- array literal は key のない要素と、compile-time integer index を key とする `index: value` を許可する。最初の key なし要素の index は 0、以降は直前の要素の index に 1 を加えた値とする。
- `[...]T` の長さは最大 index に 1 を加えた値とする。要素がない `[...]T{}` は compile error とする。
- field、index の重複と、固定長配列の範囲外 index は compile error とする。
- 指定されなかった field と要素は zero value とする。
- 初期化式は source 上の左から右へ 1 回ずつ評価する。
- nested composite literal でも型を省略しない。
- composite literal は値を生成するだけで、heap allocation を行わない。
- composite literal は addressable ではなく、`&T{...}` は compile error とする。

### 5.1 Method

methodはnamed typeに結び付いた通常関数の糖衣構文である。

```ond
type Counter struct { value: i32 }
func (counter: Counter) Value() -> i32 { return counter.value }
func (counter: *Counter) Add(amount: i32) { counter.value += amount }

type Box[T] struct { value: T }
func (box: *Box[Element]) Set(value: Element) { box.value = value }
```

- receiverは `T`、`*T`、generic型の `T[P1, ...]`、または `*T[P1, ...]` で、`T`を宣言したpackage内でだけ宣言できる。`T`のunderlying typeはstructに限らない。
- generic methodの型parameterはreceiverの具体型から位置ごとに推論し、その型argument列ごとに静的関数へ具体化する。method本体の値名と字句scopeは宣言時に検査し、型argumentに依存する型検査は具体化時に行う。
- value receiverは値をcopyし、pointer receiverはpointer値を渡す。呼出し側で暗黙のaddress取得・参照外し・copy・型変換を行わず、receiver型は完全一致を要求する。
- `nil`のpointer receiverも呼び出せる。呼出し自体はfaultせず、method内でそのpointerを参照外しした時点で通常のraw pointer規則を適用する。
- 同じbase named typeではvalue/pointer receiverを通してmethod名を一意とする。fieldと同名のmethodは宣言できない。top-level関数とは別namespaceとする。
- method名の先頭がASCII大文字なら他packageから呼べる。非公開named typeの値でも、公開APIから型推論で得られた値に対する公開method呼出しは許可する。
- `T.Method`のstatic参照、method expression、receiverをcaptureするmethod valueは提供しない。
- `receiver.Method(arguments...)` はreceiverを1回評価した後、argumentsを左から右へ1回ずつ評価する。実装はreceiverを第1引数とする一意な内部関数へloweringする。

### 5.2 Interface

interfaceはmethod集合を表す名前付きnominal typeである。

```ond
type Reader interface {
    Read(buffer: *u8, length: u32) -> i32
}

type List[T] interface {
    Get(index: u32) -> T
    Set(index: u32, value: T)
}
```

- interface同士の型同一性は宣言identityで決まり、method集合が同じでも別型である。空interfaceも `type Any interface {}` のように名前付きで宣言できるが、組み込みの `any` / `interface{}` はない。
- interfaceの型parameterは通常のgeneric型と同じ規則で明示し、型argument列ごとに異なるnamed interface型へ具体化する。interface method自身は追加の型parameterを持てない。
- interface methodはinterface名の公開・非公開にかかわらずASCII大文字で始める。private methodによるsealed interfaceは提供しない。interfaceへのmethod宣言、default実装、interface embedding、interface合成、abstract class相当の実装保持は提供しない。
- concrete typeがinterfaceを満たすには、要求された各methodと同名・同parameter型列・同return型列のpointer receiver methodが必要である。余分なmethodは許可する。`impl`宣言はない。
- generic型の具体化instanceについては、その具体型で単相化したgeneric methodを通常のmethodと同じ規則でmethod集合へ含める。
- interface値の生成は `pointer as InterfaceType` という明示変換だけで行う。sourceはnamed typeへのpointerでなければならず、valueからの暗黙address取得、value receiverの利用、interface間変換、boxingは行わない。
- interface値はdata pointerとvtable pointerからなる非所有の値である。代入・引数・returnはこの2-word値をcopyし、参照先の寿命を延長しない。
- zero valueは両pointerがnilである。typed nil pointerをinterfaceへ変換するとmethod tableは保持するが、`value == nil`はdata pointerがnilならtrueになる。tableがあるtyped nilへのmethod callはnil pointer receiverをdispatchする。tableのないnil interfaceへのmethod callはfaultする。
- 同じinterface型の `==` / `!=` だけを提供する。両data pointerがnilなら等しく、それ以外はdata pointerとvtable pointerがともに一致するとき等しい。参照先の内容は比較しない。
- interface method callはvtableから直接間接callする。vtableはconcrete pointer/interface型pairごとの静的値で、entryはinterface宣言順とする。type assertion、dynamic downcast、reflection、runtime type descriptor、interface method valueは提供しない。
- method signatureはinterface型（自分自身を含む）をparameter・returnに使用できる。interfaceをstruct field、配列要素、global、local、parameter、returnとして使用できる。

## 6. 式

v1 は次の式を持つ。operand の型が適合しない式はコンパイルエラーとする。

- integer、`f32`、`true`、`false`、`nil`、文字列、array、struct の literal
- 関数 call、method call、interface method call、配列 index、raw pointer index、struct field access
- addressable な変数、配列要素、struct field に対する `&value` と、`*pointer` による参照外し
- integer の単項 `+`, `-`, `^`
- `f32` の単項 `+`, `-`
- integer の `+`, `-`, `*`, `/`, `%`, `&`, `|`, `^`, `&^`, `<<`, `>>`
- `f32` の `+`, `-`, `*`, `/`
- 同じ integer 型、`bool`、同じ raw pointer 型、同じ function pointer 型、同じinterface型に対する `==`, `!=`
- `f32` に対する `==`, `!=`, `<`, `<=`, `>`, `>=`
- integer に対する `<`, `<=`, `>`, `>=`
- `bool` に対する `!`, `&&`, `||`

`&&` と `||` は左から評価し、結果が確定したら右辺を評価しない。それ以外の operand と関数 argument は左から右へ評価する。

### Integer Arithmetic

幅 `N` bit の integer に対する runtime 演算は、[ISO/IEC 10967-1:2012](https://www.iso.org/standard/51317.html) の modulo integer model に従う。

- shift 以外の binary operand は同じ integer 型とし、結果もその型とする。
- unary 演算の結果は operand と同じ型とする。
- `+`, `-`, `*`, 単項 `-` は数学的結果を modulo `2^N` で wrap する。
- `/` と `%` は integer 型にだけ定義する。
- quotient は 0 方向へ丸める。
- remainder は `a == (a / b) * b + (a % b)` を満たし、`0` でない場合は被除数 `a` と同じ符号を持つ。
- `b == 0` の `/` と `%` は compile-time なら compile error、実行時なら fault とする。
- signed division の `minInt / -1` は compile-time なら compile error、実行時なら fault とする。対応する remainder `minInt % -1` は `0` とする。
- bitwise 演算は `N` bit の表現に対して行う。
- signed comparison は two's-complement の符号付き値、unsigned comparison は符号なし値を比較する。
- shift の左 operand と結果は同じ integer 型とする。
- shift count の型は `u32` とする。integer literal はこの文脈で `u32` として扱う。
- shift count `k` の実効値は `k mod N` とする。
- left shift は上位 bit を破棄して `N` bit に wrap する。
- unsigned right shift は上位を `0`、signed right shift は元の最上位 bit で埋める。

runtime shift は [WebAssembly integer semantics](https://webassembly.github.io/spec/core/exec/numerics.html) と同じ規則とする。

### Integer Constant Arithmetic

- 整数定数式の算術計算は数学的な正確値で行い、runtime のような暗黙の wrap はしない。宣言・代入・引数・return・明示変換などで要求される整数型に結果を表現できない場合は compile error とする。
- 中間計算を compiler 実装上の固定幅整数（例: i128）の幅で制限しない。これは言語に新しい整数型を追加することではなく、literal の既定型・型文脈・operand の型一致規則は維持する。
- 定数 shift の左 operand の型の幅を `N` とする。count は runtime と同じく `u32` とし、実効値を `k mod N` とする。
- 定数 left shift は実効countだけ数学的に左shiftし、上位bitを暗黙に破棄しない。要求型に結果を表現できない場合は compile error とする。right shift の符号の扱いは runtime と同じとする。
- zero division と signed division overflow は前述の規則に従って compile error とする。

```ond
const A: u8 = 1 << 8     // 実効countは0なので1
const B: u8 = 128 << 1   // 256はu8で表現できないためcompile error
const C: u8 = 250 + 10   // 260はu8で表現できないためcompile error
```

定数式としての意味は最適化の有無に依存しない。runtime 式を最適化で定数畳み込みする場合は、元のruntime式のwrap・fault規則を保持する。`f32` の定数演算は以下のbinary32規則に従い、整数の正確値計算とは区別する。

### Floating-Point Arithmetic

- `f32` は IEEE 754 binary32 とする。
- 丸めモードは roundTiesToEven に固定し、動的変更はできない。
- `+0` と `-0`、`+Inf` と `-Inf`、subnormal を保持する。
- 演算結果の `NaN` は canonical quiet NaN `0x7fc00000` へ正規化する。
- `==` はどちらかが `NaN` なら常に `false`、`!=` はどちらかが `NaN` なら常に `true`、`<`, `<=`, `>`, `>=` はどちらかが `NaN` なら常に `false` とする。
- `f32` の `/` は IEEE 754 に従い、0 除算でも trap しない。
- 例外フラグ、trap handler、切替可能な丸めモードは持たない。
- compile-time の `f32` 定数評価は runtime と同じ結果 bit pattern を生成しなければならない。

## 7. 文字列リテラル

文字列リテラルはbyte配列値の糖衣構文であり、専用の文字列型やviewを作らない。

- interpreted string literalとraw string literalの字句・escape規則はGoと同じとする。raw string内のcarriage returnは値から除く。
- 通常のsource文字とUnicode escapeはUTF-8へencodeする。`\xNN`と3桁octal escapeは任意の1 byteを表せる。
- decode後のbyte数を`N`として、リテラルの型は`[N]u8`とする。長さはUnicode scalar数や表示文字数ではない。
- `""`の型は`[0]u8`とする。
- decode後の論理値と型には終端NULを自動付加しない。明示した`\000`は通常の配列要素としてbyte数と型に含める。
- 代入、引数、return、global初期化では通常の型同一性規則に従い、配列長と要素型の完全一致を要求する。暗黙の切り捨て、zero埋め、配列からpointerへの変換は行わない。
- 値は通常の配列と同じくcopyされ、変数へ格納した後は要素を書き換えられる。
- 文字列リテラルのbyte列は静的な読み取り専用実体として配置し、storage上では論理値の直後にhidden sentinel byte `0`を1個置く。sentinelは配列要素ではなく、`len`、型、代入、比較には含めない。
- リテラルの要素はaddressableとする。例えば`&"text"[0]`はプログラム実行中有効な`*u8`を生成し、raw pointerからは論理値直後のhidden sentinelを読み取れる。リテラルを根とする要素への直接代入はcompile errorとする。
- 文字列リテラルを変数や別のaggregateへ格納する場合は論理値だけを通常の配列値としてcopyする。copy後の要素は書き換えられ、copy先にhidden sentinelが続く保証はない。
- 読み取り専用の文字列storageは、完全一致またはhidden sentinelを含むsuffix一致によって共有してよい。異なるリテラル間のpointer identityは保証しない。
- `len(literal)`とindexは通常の配列規則をそのまま使用する。
- 静的な文字列・定数実体から得たpointerの寿命はプログラム全体とする。範囲と、raw pointerを介した読み取り専用領域への書き込み禁止はプログラマーが管理する。

値としての文字列とimport pathのquoted/raw string構文は同じdecoderを共有するが、import pathは有効なUTF-8でなければならない構文要素であり、`[N]u8`値ではない。

`str`は予約語でも組み込み型名でも組み込み関数名でもないため、通常の識別子として宣言できる。文字列・slice相当の型が必要なら、pointerや配列と長さを通常のstructおよびlibraryで組み合わせる。

## 8. 文と制御構文

- 文末の semicolon は書かず、改行で区切る。
- block は `{}` で囲む。
- `if` の条件を丸括弧で囲む必要はない。括弧を使った場合は、通常の式のグループ化として扱う。
- `if`、`else`、`for` は必ず block を伴う。
- `if` と条件付き `for` の条件は `bool` でなければならない。
- `for` は Go と同じ無条件、条件付き、3 clause の形を持つ。
- `break` と `continue` は最内の `for` を対象とする。
- `return` は現在の関数を終了する。
- `trap "reason"`は回復不能なCPU trapで現在の実行を終了する。理由はruntimeの文字列値ではなくdebug metadataであり、文はreturnしない。

### 8.1 `defer`

`defer` は、文へ到達した関数 invocation の終了処理を block で登録する。

```ond
func process() -> i32 {
    var first = alloc(32)
    defer {
        free(first)
    }

    if needExtra() {
        var extra = alloc(64)
        defer {
            free(extra)
        }
    }

    return 0
}
```

- 登録済みの defer block は、関数本体末尾への到達または `return` による正常終了時に、登録と逆の順序で各 1 回実行する。
- `return` の戻り値とその副作用は defer block より先に評価する。defer block は確定済みの戻り値を変更しない。
- defer block 内の式は登録時ではなく実行時に評価し、参照する local variable のその時点の値を使う。local storage は defer block の実行完了まで有効とする。
- 到達しなかった `defer` 文は登録されず、その block は実行しない。
- defer block は宣言、代入、call、`if`、およびネストした `for` を含む通常の block とする。ただし外側の関数を終了する `return` と、別の `defer` は含められない。
- defer block 内の `break` と `continue` は、その block 内にネストした最内の `for` だけを対象にできる。
- `for` の body 内では `defer` を登録できない。これにより同じdeferサイトへ複数回到達する実行を作らない。
- defer block が正常に完了する限り、残りの登録済みdefer blockを引き続き実行する。faultまたは処理の非終了が起きた場合、それ以降の実行は保証しない。
- defer block 内で呼んだ関数の戻り値を無視またはblock内で処理できるが、外側の関数へerrorを伝播する制御構文は持たない。失敗を呼び出し元へ返す必要がある終了処理は、通常の処理として明示的に実行する。

## 9. 関数

関数はトップレベルで宣言する。

```ond
func split(ptr: *u8, len: i32) -> (*u8, i32) {
    return ptr, len
}
```

- 戻り値は固定個数で、0 個、1 個、または複数個を指定できる。
- 複数の戻り値は関数の return と呼び出し結果にだけ存在し、tuple 型は作らない。
- `return` の値の個数と型は関数宣言に一致しなければならない。
- 複数の戻り値は同数の左辺へ宣言または代入できる。
- 代入は右辺をすべて評価してから左辺へ書き込む。
- compound assignment は左辺の格納先と現在値を 1 回評価してから右辺を評価し、演算結果を同じ格納先へ書き込む。`++` と `--` も同じ一回評価規則に従う。

function pointer 型は parameter 型と戻り値型を含む。

```ond
var callback: func(i32) -> i32 = update
var namedCallback: func(value: i32) -> i32 = update
```

- トップレベル関数名は、その関数と同じ signature の function pointer 値として使える。
- 関数宣言では本体で使用する parameter 名が必要だが、関数型では各 parameter 名を省略しても記載してもよい。関数型内の名前は説明用であり、binding を作らない。
- 関数型の同一性は parameter 型の列と戻り値型の列で決まり、parameter 名の有無や綴りには依存しない。上記の `callback` と `namedCallback` は同じ型である。
- 戻り値がない型は arrow を省略し、複数戻り値は `-> (R1, R2)` と書く。
- assignment と引数渡しでは signature が完全に一致しなければならない。
- function pointer は通常の call 式で間接呼び出しできる。
- `nil` function pointer の呼び出しは null address への間接 call となり CPU fault する。

Kagura target の関数呼び出し規約は [Ond ABI for Kagura v1](../targets/kagura-v1/abi.md) に従う。

### 9.1 User-defined operators

- top-level の `operator` 宣言だけが overload を持つ。通常の関数、method、interface method は overload しない。
- 対象は unary `+` / `-`、binary `+` / `-` / `*` / `/` / `%`、`[]`、`[]=`、`len` とする。比較・論理・bit operator は対象外とする。
- 解決は operand 型の完全一致だけで行い、戻り値型は候補選択に使わない。operator 自体を function pointer 値として取得できない。
- 宣言できる package は、左から見て最初の user-defined operand 型を所有する package とする。名前付きinterface型もuser-defined operandとして使用でき、そのinterfaceを所有するpackageでoperatorを宣言する。
- operator候補はoperandの静的な型だけで選ぶ。具象型がinterfaceを満たしていてもinterface型へ暗黙変換してoperator候補を追加しない。
- `[]` は値を 1 個返す。pointer を返すこともでき、その後の `.field` は通常の pointer field access と同様に暗黙 dereference する。`&collection[index]` は getter の戻り値が addressable とは限らないため禁止する。
- `[]=` は `(collection, index, value)` を受け取り、値を返さない。
- compound assignment は対応する binary operator と代入から導出し、左辺の base と index は一度だけ評価する。
- 同じ operand 型列を持つ operator 宣言は重複できない。operator は import alias を介した明示参照なしに解決候補となる。
- generic operatorは `operator[T] +(...)` のように宣言する。型argumentを使用箇所へ明記する構文はなく、operandの具体型と宣言parameter型を構造的に照合してすべての型parameterを推論する。
- generic interface向けoperatorも通常のtop-level operatorとして本体を持つ。専用alias構文やvtable entryは追加せず、`operator[T] [](values: List[T], index: u32) -> T { return values.Get(index) }`のように公開interface methodへ委譲できる。
- 各型parameterは少なくとも1個のoperand型に現れなければならない。宣言packageが所有するgeneric型を適用したoperandを少なくとも1個必要とし、具体化後も左から最初のuser-defined operandをそのpackageが所有しなければならない。裸の型parameterだけからなる汎用operatorは宣言できない。
- 推論後のparameter型列に対して通常operatorと同じ完全一致規則を適用し、型argument列ごとに静的関数へ具体化する。generic operatorと非generic operator、または複数のgeneric operatorが同じoperand型列に一致した場合、優先順位を設けず曖昧としてcompile errorにする。

## 10. メモリ管理

- address を取得した local variable の storage はその関数 invocation の stack frame、package-level variable の storage は startup segment に置く。compiler は escaping local variable を暗黙に heap へ移さない。
- heap allocation は組み込みの `new(T)` または `alloc[T](count)` でだけ明示的に行う。
- `new(T)` は `T` 1 個分を `alignof(T)` に揃えて確保し、zero value で初期化した `*T` を返す。
- `alloc[T](count)` は `T` を `count: u32` 個格納できる連続領域を `alignof(T)` に揃えて確保し、`*T` を返す。領域の初期値は規定しない。
- `new` と `alloc` は allocation failure 時に `nil` を返す。`alloc[T](0)` は `nil` を返す。zero-size 型を正の個数確保した成功結果は non-nil とする。
- `free(pointer)` は `new` または `alloc` が返した未解放の先頭 pointer、または `nil` を受け取る。`free(nil)` は何もしない。
- heap allocation の所有権は pointer を保持するプログラム側にあり、不要になった領域を `free` する。
- pointer の寿命、解放忘れ、二重解放、範囲外アクセスは言語が検査しない。

bounds checkの失敗は追加のstack frameやheap allocationに依存しない。具体的なfault命令とdebug metadataはtarget仕様に従う。

## 11. 初期化とエントリ

- package-level `var` は initializer を省略した zero value、または compile-time constant initializer だけを持てる。runtime 呼び出しを伴う initializer は compile error とする。
- sourceから宣言する `func init()` hookは持たない。compilerはglobal initializerを適用する内部関数を生成してよいが、user codeから参照できず、runtime初期化の入口にもならない。
- `init` は予約語でも特別名でもなく、通常の関数名として宣言・明示呼び出しできる。
- `main` package は `func main()` を 1 個持たなければならない。
- `main` の return は正常終了 code `0` とする。

Kagura target の runtime entry と正常終了通知は [Ond for Kagura Machine Profile v1](../targets/kagura-v1/machine.md) に従う。

## 12. Build Output

- compilerはsourceをMIR/Objectへ翻訳し、linkerはLinkedImageを生成する。配布形式とROM生成は機種SDKが定義する。
- link 工程は compiler 実装の [Ond Linker](../../../crates/compiler/docs/ond-linker.md) に従う。
- assembly は中間生成物または診断用出力として生成できる。
- 全packageとruntimeは静的にlinkし、機種SDKがその結果を配布imageへ配置する。
- asset discovery、resource ID の割り当て、device protocol は Ond の言語機能ではない。
- compilerはproject内の非source fileを暗黙に同梱しない。アセット管理と同梱は機種SDKが担当する。
