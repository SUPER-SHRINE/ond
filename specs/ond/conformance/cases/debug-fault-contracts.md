# Debug metadataとfaultの契約case
<!-- ond-spec-id: conformance/cases/debug-fault-contracts -->

適用版はOnd v1、Kagura v1、artifact Contract revision 7とし、[debug schemaと操作](../../debug-metadata/v1.md)、[target fault表](../../targets/kagura-v1/machine.md#faultと観測結果)から期待を導く。\
各操作は独立した入力とstateで行い、検査対象を下記のschema、区間、lookup、復元、CPU faultの各操作で指定する。

## Debug共有入力
<!-- ond-spec-id: conformance/debug-fault-contracts/input -->

共有Object Oはname="diagnostic"、Text sectionがalignment4、memory_size64、dataは64個のzero byte、relocation列は空とする。\
symbolはF（Exported Function）とG（Local Function）の2個で、共にText、offset0、size16とする。\
Oに所属するDebugObject Dの全JSON値は次のとおりである。

```json
{"package":"sample","functions":[{"symbol":"F","section":"Text","offset":0,"size":16,"frame_size":8,"return_address_offset":4,"source":null,"unwind":[{"offset":0,"cfa_sp_offset":0,"return_address":"LinkRegister"},{"offset":4,"cfa_sp_offset":8,"return_address":{"StackOffset":4}},{"offset":16,"cfa_sp_offset":0,"return_address":"LinkRegister"}]}],"ranges":[{"section":"Text","start":0,"end":16,"operation":"Source","message":null,"source":null,"function":"G"},{"section":"Text","start":4,"end":8,"operation":"Call","message":null,"source":null,"function":"G"},{"section":"Text","start":4,"end":8,"operation":"ExplicitTrap","message":"reason","source":null,"function":"G"}]}
```

配置入力の論理値はread_only=null、memory_base4040、memory_size65592、stack_size8192、entry_symbol="F"、heap_base_symbol="heap"、stack_bottom_symbol="bottom"とし、Textの最終addressを4096に指定する。\
link前の操作はschema・Object所属・symbol区間・数値条件の検査であり、CPU実行は行わない。\
link後操作には標準実行バイナリIを使い、ヘッダはmagic=ONDX、version=target=abi=1、image_size=120、entry_address=4096、initial_sp=69632、section_count=1とする。\
唯一のrecordはkind=Text、file_offset=56、file_size=memory_size=64、memory_address=4096、alignment=4とし、payloadはOの64個のzero byteである。\
これらを[wire規則](../../artifacts.md#標準実行バイナリ)でencodeした120 byteをIの全bytesとする。\
DebugInfoはDの位置へ4096を加算した値、Iの全bytesから求めたbuild_id、stack_bottom=61440、stack_top=69632で構成し、postlinkのschema・区間とlookupを検査する。

| Case ID | 要件 | 操作・入力 | 期待 |
| --- | --- | --- | --- |
| DBG-01.relative-link | DBG-01 | OとDを上のplanでlink | F.address=4096、unwind.address列=[4096,4100,4112]、ranges=[4096,4112),[4100,4104),[4100,4104) |
| DBG-01.range-priority | DBG-01 | link後、PC4100のrange lookup | 幅4のCallを選ぶ。同幅ExplicitTrapより先のrecord |
| DBG-01.local-alias | DBG-01 | 上のDとIでpostlink検査 | 受理。GのFunctionDebug recordの存在を追加要求しない |
| DBG-01.end-row | DBG-01 | Dのunwind末尾offset16を含めlink前検査 | 受理。link後PC4112のfunction lookupは該当functionなし |
| DBG-01.duplicate-row | DBG-01 | D.functions[0].unwind[1]を同じoffsetで1件追加して検査 | 位置重複のlink error |
| DBG-01.range-symbol-outside | DBG-01 | D.ranges[0].endだけ20へ変更して検査 | Gのsymbol範囲外のlink error |
| DBG-01.nontrap-message | DBG-01 | D.ranges[0].messageだけxへ変更して検査 | Source operationのmessage条件違反 |
| DBG-01.image-mismatch | DBG-01 | link後sidecarのstack_topだけ4増加してVM検査 | sidecar全体の使用拒否 |
| DBG-01.stack-unwind | DBG-01 | link後PC4100、SP69624、RAM[69628,69632)のu32=6000でunwind | caller SP69632、caller PC6000。次段はfunction欠落で打切り |
| DBG-01.empty-unwind | DBG-01 | link後F.unwind=[]としてPC4100のtrace | Fからのrow復元をせず打切り |
| DBG-01.stack-boundary | DBG-01 | link後PC4100、SP61436でtrace | stack範囲外により読取り前に打切り |

## Target faultの観測
<!-- ond-spec-id: conformance/debug-fault-contracts/faults -->

要件は`FAULT-01`とする。\
整数・変換のfault入力は[整数実行](./kagura-integers.md)と[cast実行](./kagura-casts.md)、bounds/nilは[評価・fault](./kagura-evaluation-combinations.md)の同じIDと入力を使い、CPU faultの名称を[target対応表](../../targets/kagura-v1/machine.md#faultと観測結果)で比較する。

`FAULT-01.explicit-trap`のmain.ondは次の全文とし、[Kagura共通初期state](./kagura-integers.md)でcompile/link後に実行する。\
期待はINVALID_INSTRUCTION、EXITとOUTPUTのSENTINEL保持、対応debug rangeのoperation=ExplicitTrapとmessage="reason"である。

```ond
package main
func main() { trap "reason" }
```

`FAULT-01.nil-unmapped`は既存`NIL-02.call`と同じunmapped address0を前提にBUS_FAULTを比較する。\
`FAULT-01.raw-unaligned`は`package main`と`func main(){var p=536870913 as u32 as *u32;_=*p}`の2行（LF区切り・末尾LFあり）を同じ初期stateで実行し、UNALIGNED_ACCESSとfaulting_pcが当該load命令のPCであることを比較する。
