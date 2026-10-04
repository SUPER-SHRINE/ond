# 言語ツアー

この章では、Ond v1の主要機能を短い例で紹介する。境界条件や評価順序を含む厳密な定義は[Ond v1言語仕様](../../specs/ond/v1/language.md)を参照する。

## ソースファイル

Ondのソースファイルは`.ond`拡張子を使い、先頭で所属packageを宣言する。文末のsemicolonは通常書かず、改行で文を区切る。

```ond
package main

// 行コメント
/* block comment */

func main() {
}
```

同じdirectoryにある`.ond`ファイルは同じpackageに属する。project rootでは`package main`を宣言し、引数と戻り値を持たない`func main()`を1個定義する。

## 変数と定数

再代入できる値は`var`、コンパイル時定数は`const`で宣言する。関数内では`:=`も使える。

```ond
const Width: u32 = 320

var enabled: bool = true
var count = 0       // i32と推論される

func example() {
    value := 10
    value += 2
    value++
}
```

初期値のない`var`には型が必要で、その型のzero valueで初期化される。`_`へ代入した値は捨てられる。

## 基本型と変換

数値型は`u8`、`u16`、`u32`、`i8`、`i16`、`i32`、`f32`である。論理型は`bool`である。このほかにpointer、固定長array、struct、function pointer、名前付きinterfaceがある。

Ondは異なる数値型を暗黙変換しない。変換は`as`で明示する。

```ond
var small: u8 = 42
var wide: u32 = small as u32
var fraction: f32 = wide as f32
```

`type`はaliasではなく、新しいdefined typeを作る。underlying typeが同じでも別の型として扱われる。

```ond
type UserId u32

var raw: u32 = 7
var id: UserId = raw as UserId
```

## 関数と複数の戻り値

戻り値は`->`の後に書く。値を返さない関数では`->`自体を書かない。

```ond
func divide(value: i32, divisor: i32) -> (i32, i32) {
    return value / divisor, value % divisor
}

func useResult() {
    quotient, remainder := divide(17, 5)
    _, _ = quotient, remainder
}
```

複数の戻り値は分割代入にだけ使われ、tuple型にはならない。トップレベル関数はsignatureが一致するfunction pointerとして保持できる。

```ond
func twice(value: i32) -> i32 {
    return value * 2
}

var callback: func(i32) -> i32 = twice
```

## 条件分岐とloop

条件は`bool`でなければならない。`for`は無条件、条件だけ、3 clauseの3形式を持つ。

```ond
func find(limit: i32) -> i32 {
    for index := 0; index < limit; index++ {
        if index == 3 {
            return index
        }
    }
    return -1
}
```

`break`と`continue`は最内の`for`に作用する。`&&`と`||`は短絡評価する。

## 配列

Ondのarrayは固定長で、長さも型の一部である。代入と引数渡しでは配列全体を値copyする。

```ond
var values: [4]u32 = [4]u32{10, 20, 30, 40}
var sparse: [8]u32 = [8]u32{2: 100, 5: 200}
var inferred = [...]u32{10, 20, 30}
var count: u32 = len(values)
```

実行時に決まるarray indexはbounds checkされる。raw pointerのindexはcheckされない。

## structとmethod

struct literalはfield名を指定する。省略したfieldはzero valueになる。

```ond
type Counter struct {
    value: i32
}

func (counter: Counter) Value() -> i32 {
    return counter.value
}

func (counter: *Counter) Add(amount: i32) {
    counter.value += amount
}

func useCounter() {
    var counter = Counter{value: 1}
    var pointer: *Counter = &counter
    pointer.Add(2)
    _ = counter.Value()
}
```

receiver型は完全一致が必要で、呼び出し時に暗黙のaddress取得や参照外しは行わない。value receiverは値をcopyし、pointer receiverはpointerを渡す。

## pointerとheap

`&value`でaddressを取得し、`*pointer`で参照外しする。`nil`はpointer、function pointer、interfaceのzero valueとして使える。

```ond
func allocate() -> *u32 {
    var pointer: *u32 = new(u32)
    if pointer != nil {
        *pointer = 42 as u32
    }
    return pointer
}
```

`new(T)`はzero初期化した`T`を1個確保する。`alloc[T](count)`は未初期化の連続領域を確保する。いずれも失敗時は`nil`を返し、不要になった領域は`free`で明示的に解放する。

Ondはgarbage collectionを行わない。また、local variableへのpointerが関数外へescapeしても、compilerは自動的にheapへ移さない。

## 文字列リテラル

文字列リテラルは専用の文字列型ではなく、UTF-8 byte列を保持する`[N]u8`値である。論理的な配列値に終端の`0`は含まれない。

```ond
var ascii: [5]u8 = "hello"
var japanese: [6]u8 = "音頭"
```

可変長の文字列やsliceが必要なら、pointerと長さを持つstructなどをlibrary側で定義する。

読み取り専用のliteral storageには、raw pointerとの受け渡し用に論理値の直後へhidden NUL byteが置かれる。このbyteは型や`len`には含まれず、変数へcopyした配列の直後にNULが続く保証もない。

## generic型と関数

型argumentはすべて明示する。型argumentの推論やconstraintはなく、generic型・関数は使用された型argument列ごとに具体化される。

```ond
type Box[T] struct {
    value: T
}

func Identity[T](value: T) -> T {
    return value
}

func (box: *Box[Element]) Set(value: Element) {
    box.value = value
}

func useGeneric() {
    var box = Box[u32]{value: Identity[u32](7 as u32)}
    (&box).Set(9)
}
```

generic型のmethodでは、receiverの型argument位置がmethod内の型parameter宣言になる。上の`Element`は`Box`宣言側の`T`と同じ名前でなくてよい。呼び出し時の型argumentはreceiverから推論される。特定の`Box[u32]`だけにmethodを追加したり、method名の後ろへ別の型parameterを宣言したりはできない。

generic関数、generic method、generic operatorの本体は、型argumentに依存しない値名を宣言時に検査する。そのため、未使用のgeneric宣言内でも未定義名はエラーになる。演算可否など型argumentに依存する検査は具体化時に行う。

## interface

interfaceは名前付きのnominal typeである。concrete typeは、必要なsignatureを持つpointer receiver methodによってinterfaceを満たす。`impl`宣言はない。

```ond
type Runner interface {
    Run() -> i32
}

type Task struct {
    result: i32
}

func (task: *Task) Run() -> i32 {
    return task.result
}

func run(runner: Runner) -> i32 {
    return runner.Run()
}

func exampleInterface() -> i32 {
    var task = Task{result: 7}
    var runner: Runner = (&task) as Runner
    return run(runner)
}
```

interfaceへの変換は`pointer as InterfaceType`と明示する。boxing、type assertion、reflection、interface間の変換は提供しない。

interface methodは常にASCII大文字で始める。interface名が非公開でもprivate methodを要求するinterfaceは宣言できない。

interfaceもgeneric型として宣言できる。interface method自身に追加の型parameterは宣言せず、interfaceの型parameterをsignature内で使う。generic型のpointer receiver methodも、具体化後のsignatureが一致すればinterfaceを満たす。

次の例は、非公開の具象型を`List[T]`として公開し、index operatorを公開methodへ委譲する。

```ond
type List[T] interface {
    Get(index: u32) -> T
    Set(index: u32, value: T)
    Destroy()
}

type pairList[T] struct {
    values: [2]T
}

func (values: *pairList[T]) Get(index: u32) -> T {
    return values.values[index]
}

func (values: *pairList[T]) Set(index: u32, value: T) {
    values.values[index] = value
}

func (values: *pairList[T]) Destroy() {
    free(values)
}

func NewPairList[T](first: T, second: T) -> List[T] {
    values := new(pairList[T])
    if values == nil {
        return nil
    }
    values.values[0] = first
    values.values[1] = second
    return values as List[T]
}

operator[T] [](values: List[T], index: u32) -> T {
    return values.Get(index)
}

operator[T] []=(values: List[T], index: u32, value: T) {
    values.Set(index, value)
}

func useList() {
    values := NewPairList[u32](3, 5)
    if values == nil {
        return
    }
    defer {
        values.Destroy()
    }
    values[1] = 13
    _ = values[1]
}
```

`pairList`の実装はpackage外から直接参照できないが、`NewPairList[u32]`が返す`List[u32]`の公開methodとoperatorは利用できる。変換は`values as List[T]`のように明示する必要があり、operator選択も式の静的な型に対して行う。`List[u32]`を満たす具象pointerへ上記operatorを直接適用する暗黙interface変換はない。

interface値は参照先の所有権や寿命を管理しない。この例の`Destroy`のように、確保と解放の契約はlibraryのAPIとして明示する。

## trapとdebug情報

`trap "reason"`は回復不能なCPU trapで実行を停止する文であり、関数呼び出しではない。理由はruntimeの文字列値にはならず、link時に`.onddebug` sidecarへ記録される。

```ond
func requireIndex(index: u32, length: u32) {
    if index >= length {
        trap "index is out of range"
    }
}
```

対応するVMはsidecarを使ってtrap理由、source位置、fault分類、stack traceを表示できる。このdebug表示はKagura VMの任意機能であり、sidecarがなくても実行imageはtrapで停止できる。

## defer

`defer` blockは関数が正常終了するとき、登録と逆の順序で実行される。確保した領域の解放などに使える。

```ond
func work() -> i32 {
    var buffer: *u8 = alloc[u8](256 as u32)
    defer {
        free(buffer)
    }

    if buffer == nil {
        return -1
    }
    return 0
}
```

`defer`はloop bodyでは登録できず、fault時の実行も保証しない。戻り値はdefer blockより先に確定する。

## 公開範囲

top-level宣言、struct field、methodの名前がASCII大文字で始まると、他packageから参照できる。小文字で始まる名前はpackage内だけで使える。

```ond
package math

const Version = 1 // 公開
const scale = 2   // package内だけ
```

importと複数packageの例は[プロジェクトとパッケージ](projects-and-packages.md)で説明する。
