# 公開interfaceと型付きwire値
<!-- ond-spec-id: conformance/cases/interface-values -->

要件は`ART-01`とし、Contract revision 6の[公開interface](../../artifacts.md#公開interface)、[wire形式](../../artifacts.md#json-wire形式)、[具体化identity](../../build/cache.md#意味型具体化定数の正規化)を定義元とする。

## 型付きValueの読出しと生成
<!-- ond-spec-id: conformance/interface-values/typed-values -->

共有Interface Iの全JSONは次のとおりである。各行は独立にIと表のValueをwire読出しし、そのValueのtyをIのtypesで解決して成立条件を検査する。受理行のwriter比較はValueを生成したときの正規化結果を表し、型番号はIのまま比較する。追加source、Object生成、machine実行はこの操作へ含めない。

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

`ART-01.value-defined-cycle`はI.types[11].kindだけを`{"Defined":11}`へ変更し、Value={ty:11,kind:Zero}を検査して直接再帰型のerrorを期待する。`ART-01.value-pointer-cycle`はI.types[11].kindを`{"Defined":7}`、I.types[7].kindを`{"Pointer":11}`へ変更し、同じValueを受理する。

`ART-01.value-equivalent-child`はI.typesへ`{name:null,kind:U8,methods:[]}`をindex12として追加し、Value={ty:9,kind:Composite[[0,{ty:12,kind:Integer1}]]}を受理する。子ty12とelement参照1は同じU8型である。\
`ART-01.value-distinct-nominal`はI.typesへ`{name:p.OtherByte,kind:Defined(1),methods:[]}`をindex12として追加し、I.types[9].kind.Array.elementを11へ変更する。Value={ty:9,kind:Composite[[0,{ty:12,kind:Integer1}]]}は異なるnominal型の子として拒否する。

## Generic sourceと宣言owner
<!-- ond-spec-id: conformance/interface-values/generic-owner -->

Ond v1とKagura v1を適用し、manifestの全bytesは`[project]`と`name = "p"`の2行（LF区切り・末尾LFあり）とする。通常compileを新しいproject状態で行い、追加library/依存projectは持たない。入力は次の2file全文とし、UTF-8、LF、各末尾LFを保持する。

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

compileを成功させ、p/libのInterfaceと宣言ownerごとのGenerationおよびObjectを比較する。具体化identityはGeneration.instancesの集合で照合し、Objectに収録される具体化codeと残るpackage間参照は宣言ownerへの所属と解決で比較する。値の観測は共通case契約のMemoryVmでfresh状態のmain.observe([])を1回呼ぶ。

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

Box[U8]のnameは`p/lib.Box["U8"]`、Box[Box[U8]]のnameは次のstringと完全一致する。この表記内のargumentはTypeExprのcanonical JSONである。

```text
p/lib.Box[{"Defined":{"arguments":["U8"],"name":"Box","package":"p/lib"}}]
```
