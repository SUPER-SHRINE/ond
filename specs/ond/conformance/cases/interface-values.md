# 公開interfaceと型付きwire値
<!-- ond-spec-id: conformance/cases/interface-values -->

要件は`ART-01`とし、Contract revision 7の[公開interface](../../artifacts.md#公開interface)、[wire形式](../../artifacts.md#json-wire形式)、[具体化identity](../../build/cache.md#意味型具体化定数の正規化)を定義元とする。

## 型付きValueの読出しと生成
<!-- ond-spec-id: conformance/interface-values/typed-values -->

共有Interface Iの全JSONは次のとおりである。\
各行は独立にIと表のValueをwire読出しし、そのValueのtyをIのtypesで解決して成立条件を検査する。\
受理行のwriter比較はValueを生成したときの正規化結果を表し、型番号はIのまま比較する。\
追加source、Object生成、machine実行はこの操作へ含めない。

```json
{"exports":{"T0":{"Global":0},"T1":{"Global":1},"T10":{"Global":10},"T11":{"Global":11},"T2":{"Global":2},"T3":{"Global":3},"T4":{"Global":4},"T5":{"Global":5},"T6":{"Global":6},"T7":{"Global":7},"T8":{"Global":8},"T9":{"Global":9}},"package":"p","types":[{"kind":"I8","methods":[],"name":null},{"kind":"U8","methods":[],"name":null},{"kind":"I16","methods":[],"name":null},{"kind":"I32","methods":[],"name":null},{"kind":"U32","methods":[],"name":null},{"kind":"F32","methods":[],"name":null},{"kind":"Bool","methods":[],"name":null},{"kind":{"Pointer":1},"methods":[],"name":null},{"kind":{"Function":{"parameters":[],"returns":[]}},"methods":[],"name":null},{"kind":{"Array":{"element":1,"length":2}},"methods":[],"name":null},{"kind":{"Struct":[{"name":"Value","private_owner":null,"ty":1}]},"methods":[],"name":null},{"kind":{"Defined":1},"methods":[],"name":"p.Byte"}]}
```

| Case ID | Valueの全JSON | 読出し・writer期待 |
| --- | --- | --- |
| ART-01.value-i8-min | <code>{&quot;kind&quot;:{&quot;Integer&quot;:18446744073709551488},&quot;ty&quot;:0}</code> | 受理、writerは同じ値 |
| ART-01.value-i8-unextended | <code>{&quot;kind&quot;:{&quot;Integer&quot;:128},&quot;ty&quot;:0}</code> | 符号拡張不一致で拒否 |
| ART-01.value-u8-max | <code>{&quot;kind&quot;:{&quot;Integer&quot;:255},&quot;ty&quot;:1}</code> | 受理、writerは同じ値 |
| ART-01.value-u8-overflow | <code>{&quot;kind&quot;:{&quot;Integer&quot;:256},&quot;ty&quot;:1}</code> | unsigned範囲外で拒否 |
| ART-01.value-i32-minus-one | <code>{&quot;kind&quot;:{&quot;Integer&quot;:18446744073709551615},&quot;ty&quot;:3}</code> | 受理、writerは同じ値 |
| ART-01.value-i32-unextended | <code>{&quot;kind&quot;:{&quot;Integer&quot;:4294967295},&quot;ty&quot;:3}</code> | 符号拡張不一致で拒否 |
| ART-01.value-wrong-kind | <code>{&quot;kind&quot;:{&quot;Bool&quot;:true},&quot;ty&quot;:1}</code> | 整数型へのBool kindで拒否 |
| ART-01.value-nil-pointer | <code>{&quot;kind&quot;:&quot;Nil&quot;,&quot;ty&quot;:7}</code> | 受理、writerのkind=Zero |
| ART-01.value-nil-function | <code>{&quot;kind&quot;:&quot;Nil&quot;,&quot;ty&quot;:8}</code> | 受理、writerのkind=Zero |
| ART-01.value-nil-integer | <code>{&quot;kind&quot;:&quot;Nil&quot;,&quot;ty&quot;:4}</code> | 整数型へのNil kindで拒否 |
| ART-01.value-false | <code>{&quot;kind&quot;:{&quot;Bool&quot;:false},&quot;ty&quot;:6}</code> | 受理、writerのkind=Zero |
| ART-01.value-positive-zero | <code>{&quot;kind&quot;:{&quot;Float32&quot;:0},&quot;ty&quot;:5}</code> | 受理、writerのkind=Zero |
| ART-01.value-negative-zero | <code>{&quot;kind&quot;:{&quot;Float32&quot;:2147483648},&quot;ty&quot;:5}</code> | 受理、writerは同じbits |
| ART-01.value-array-element | <code>{&quot;kind&quot;:{&quot;Composite&quot;:[[1,{&quot;kind&quot;:{&quot;Integer&quot;:255},&quot;ty&quot;:1}]]},&quot;ty&quot;:9}</code> | 受理、writerは同じ値 |
| ART-01.value-array-range | <code>{&quot;kind&quot;:{&quot;Composite&quot;:[[2,{&quot;kind&quot;:&quot;Zero&quot;,&quot;ty&quot;:1}]]},&quot;ty&quot;:9}</code> | array index範囲外で拒否 |
| ART-01.value-array-child-type | <code>{&quot;kind&quot;:{&quot;Composite&quot;:[[0,{&quot;kind&quot;:{&quot;Integer&quot;:1},&quot;ty&quot;:0}]]},&quot;ty&quot;:9}</code> | 子型I8と要素型U8の不一致で拒否 |
| ART-01.value-array-duplicate | <code>{&quot;kind&quot;:{&quot;Composite&quot;:[[0,{&quot;kind&quot;:&quot;Zero&quot;,&quot;ty&quot;:1}],[0,{&quot;kind&quot;:&quot;Zero&quot;,&quot;ty&quot;:1}]]},&quot;ty&quot;:9}</code> | index重複で拒否 |
| ART-01.value-struct-range | <code>{&quot;kind&quot;:{&quot;Composite&quot;:[[1,{&quot;kind&quot;:&quot;Zero&quot;,&quot;ty&quot;:1}]]},&quot;ty&quot;:10}</code> | field index範囲外で拒否 |
| ART-01.value-aggregate-zero | <code>{&quot;kind&quot;:{&quot;Composite&quot;:[[1,{&quot;kind&quot;:{&quot;Integer&quot;:0},&quot;ty&quot;:1}]]},&quot;ty&quot;:9}</code> | 受理、writerのkind=Zero |
| ART-01.value-defined-underlying | <code>{&quot;kind&quot;:{&quot;Integer&quot;:255},&quot;ty&quot;:11}</code> | 受理、writerは同じ値 |

`ART-01.value-defined-cycle`はI.types[11].kindだけを`{"Defined":11}`へ変更し、Value={ty:11,kind:Zero}を検査して直接再帰型のerrorを期待する。\
`ART-01.value-pointer-cycle`はI.types[11].kindを`{"Defined":7}`、I.types[7].kindを`{"Pointer":11}`へ変更し、同じValueを受理する。

`ART-01.value-equivalent-child`はI.typesへ`{name:null,kind:U8,methods:[]}`をindex12として追加し、Value={ty:9,kind:Composite[[0,{ty:12,kind:Integer1}]]}を受理する。\
子ty12とelement参照1は同じU8型である。\
`ART-01.value-distinct-nominal`はI.typesへ`{name:p.OtherByte,kind:Defined(1),methods:[]}`をindex12として追加し、I.types[9].kind.Array.elementを11へ変更する。\
Value={ty:9,kind:Composite[[0,{ty:12,kind:Integer1}]]}は異なるnominal型の子として拒否する。

## Generic sourceと宣言owner
<!-- ond-spec-id: conformance/interface-values/generic-owner -->

Ond v1とKagura v1を適用し、manifestの全bytesは`[project]`と`name = "p"`の2行（LF区切り・末尾LFあり）とする。\
compileを新しいproject状態で行い、依存projectは持たない。\
入力は次のfile全文とし、UTF-8、LF、各末尾LFを保持する。

`main.ond`:
```ond
package main
import "p/lib"
func observe()->u32{var b:lib.Box[u8];var f:func(u8)->u8=lib.Identity[u8];return (lib.Identity[u8](1) as u32)+(f(2) as u32)+lib.Identity[u32](4)+(b.Get() as u32)+((b+3) as u32)}
func main(){}
```

`lib/lib.ond`:
```ond
package lib
type Box[T] struct{Value:T}
type Holder struct{Small:Box[u8];Nested:Box[Box[u8]]}
func Identity[T](x:T)->T{return x}
func Unused[T](x:T)->T{return x}
func (b:Box[T]) Get()->T{return b.Value}
operator (b:Box[T]) +(rhs:T)->T{return b.Value+rhs}
```

compileを成功させ、p/libのInterfaceと宣言ownerごとのGenerationおよびObjectを比較する。\
具体化identityはGeneration.instancesの集合で照合し、Objectに収録される具体化codeと残るpackage間参照は宣言ownerへの所属と解決で比較する。\
値の観測は共通case契約のMemoryVmでfresh状態のmain.observe([])を1回呼ぶ。

| Case ID | 観測 | 期待 |
| --- | --- | --- |
| ART-01.generic-exports | p/lib interface.exports | HolderのType bindingだけ。NormalizedInterface.exportsも同じbinding集合 |
| ART-01.generic-types | Holderから到達するtypes | Box[U8]とBox[Defined(Box[U8])]の具体型を含み、nameは下記canonical名 |
| ART-01.generic-owner | Identityの具体化codeと残る参照 | 収録先はp/lib Objectであり、package間参照は宣言ownerの定義へ解決 |
| ART-01.generic-value | main.observe([]) | 返値[10] |
| HASH-01.function-arguments | Identity InstanceId | declaration=Function(package p/lib,name Identity)、receiver_arguments=[]、type_argumentsは[U8]と[U32]の2件 |
| HASH-01.method-arguments | Get InstanceId | declaration=Method(package p/lib,receiver Box,name Get,pointer_receiver false)、receiver_arguments=[U8]、type_arguments=[] |
| HASH-01.operator-arguments | + InstanceId | declaration=Operator(package p/lib,receiver Box,token +,operand_count 2,pointer_receiver false)、receiver_arguments=[U8]、type_arguments=[] |

p/libのGeneration.instancesは、表のIdentity U8/U32、Get U8、operator+ U8の4identityからなる集合と完全一致する。

Box[U8]のnameは`p/lib.Box["U8"]`、Box[Box[U8]]のnameは次のstringと完全一致する。\
この表記内のargumentはTypeExprのcanonical JSONである。

```text
p/lib.Box[{"Defined":{"arguments":["U8"],"name":"Box","package":"p/lib"}}]
```

## Generic具体化の経路
<!-- ond-spec-id: conformance/interface-values/generic-expansion-paths -->

要件は`TYPE-05`とし、[型parameterの成立条件](../../language/v1/types.md#型parameter)から期待を導く。\
各行は独立projectの`main.ond`全文であり、共通manifestを用いてCoreで検査する。\
受理行はcompile成功、拒否行は該当する型適用または使用箇所とgeneric宣言に対応する[関連位置](./README.md#入力と期待結果の読み方)を比較する。

| Case ID | main.ond全文 | Core期待 |
| --- | --- | --- |
| TYPE-05.direct-function | <code>package main<br>func F[T](){F[T]()}<br>func main(){F[u8]()}<br></code> | 受理。F[u8]の再要求は同じ型argument列 |
| TYPE-05.independent-functions | <code>package main<br>func F[T](){F[T]()}<br>func main(){F[u8]();F[u16]()}<br></code> | 受理。独立したF[u8]とF[u16]の2具体化 |
| TYPE-05.finite-function-chain | <code>package main<br>func F[T](){G[*T]()}<br>func G[T](){}<br>func main(){F[u8]()}<br></code> | 受理。F[u8]からG[*u8]へ到達 |
| TYPE-05.growing-function | <code>package main<br>func Grow[T](){Grow[*T]()}<br>func main(){Grow[u8]()}<br></code> | compile error。Grow[u8]からGrow[*u8]を要求 |
| TYPE-05.mutual-function | <code>package main<br>func F[T](){G[*T]()}<br>func G[T](){F[T]()}<br>func main(){F[u8]()}<br></code> | compile error。F[u8]からG[*u8]を経てF[*u8]を要求 |
| TYPE-05.flipped-function | <code>package main<br>func Flip[A,B](){Flip[B,A]()}<br>func main(){Flip[u8,u16]()}<br></code> | compile error。Flip[u8,u16]からFlip[u16,u8]を要求 |
| TYPE-05.finite-nominal | <code>package main<br>type Box[T] struct{value:T}<br>var B:Box[Box[u8]]<br>func main(){}<br></code> | 受理。Bの型はBox[Box[u8]] |
| TYPE-05.pointer-nominal | <code>package main<br>type Node[T] struct{next:*Node[T]}<br>var N:Node[u8]<br>func main(){}<br></code> | 受理。Node[u8]の再要求は同じ型argument列 |
| TYPE-05.growing-nominal | <code>package main<br>type Grow[T] struct{next:*Grow[*T]}<br>var G:Grow[u8]<br>func main(){}<br></code> | compile error。Grow[u8]からGrow[*u8]を要求 |
| TYPE-05.growing-method | <code>package main<br>type Box[T] struct{}<br>func (b:Box[T]) Grow(){var next:Box[*T];next.Grow()}<br>func main(){var b:Box[u8];b.Grow()}<br></code> | compile error。Growのreceiver型argumentがu8から*u8へ変わる |
| TYPE-05.growing-operator | <code>package main<br>type Box[T] struct{}<br>operator (b:Box[T]) +(rhs:Box[T])-&gt;u8{var next:Box[*T];return next+next}<br>func main(){var b:Box[u8];_=b+b}<br></code> | compile error。operator +のreceiver型argumentがu8から*u8へ変わる |
| TYPE-05.nested-growing-nominal | <code>package main<br>type Relay[U] U<br>type Grow[T] struct{next:*Relay[Grow[*T]]}<br>var G:Grow[u8]<br>func main(){}<br></code> | compile error。Grow定義内のRelay型argumentに明記されたGrow[*u8]を要求 |

## 実行rootの論理package所属
<!-- ond-spec-id: conformance/interface-values/executable-root-package -->

要件は`PKG-01`、`ART-01`、`HASH-01`とし、[実行rootの論理path](../../language/v1/names-and-scopes.md#packageとimport)、[成果物schema](../../artifacts.md#packageartifactとcompilationmanifest)、[具体化identity](../../build/cache.md#意味型具体化定数の正規化)を適用する。\
独立projectのmanifest全文は次の内容とし、LF区切り・末尾LFありとする。

```toml
[project]
name = "p"
[link]
startup = "external"
stack-size = 0x100000
[link.ram]
base = 0x10002000
size = 0x03ffe000
[link.exit]
mode = "jump"
address = 0x10000000
```

依存project、既存cacheは持たず、次のfileをUTF-8、LF、各末尾LFありで置く。\
project rootをcwdとして`ond compile .`を実行する。

`main.ond`:

```ond
package main
type Root struct{secret:u8}
var Global:u8
func Id[T](x:T)->T{return x}
func main(){Global=Id[u8](1)}
```

| Case ID | 操作と比較対象 | 期待 |
| --- | --- | --- |
| PKG-01.root-logical-normal | compileし、rootのPackageArtifact、Interface、DebugObject、CompilationManifest.ArtifactRef、PackageBuildInput、Generation.interfaceのpackage、NominalIdと非公開fieldのprivate_owner、Id[u8]のDeclarationId.package、Objectの通常非generic function/global symbol、ProgramMetadata.entry_symbolを比較 | 論理package fieldはすべてp、nominal名はp.Root、Root.secretのprivate_ownerはp、IdのDeclarationId.packageはp、通常symbolはmain.mainとmain.Global、entry_symbolはmain.main |
