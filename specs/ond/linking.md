# Object linking 契約
<!-- ond-spec-id: linking/v1 -->

この章はObject列とcaller指定LinkPlanからLinkedImageを生成する外部契約を定義する。

## Symbol解決と配置
<!-- ond-spec-id: linking/symbol-resolution -->

Objectは[成果物schema](./artifacts.md#object)に従う。\
入力Objectの成立条件は次のとおりであり、違反はlink errorとなる。

| 対象 | 成立条件 |
| --- | --- |
| Section | kindはObject内で一意、alignmentは正の2冪、data長はmemory_size以下 |
| Text/Rodata | data長とmemory_sizeが一致 |
| Bss | dataは空 |
| Text | alignmentは4以上、memory_sizeは4の倍数 |
| Symbol | 名前は空でなくObject内で一意 |
| Imported | section=null、offset=0、size=0 |
| 定義symbol | 対応sectionが存在し、offset+sizeがsection memory_size以内 |
| 定義Function（Local/Exported） | sectionはText、offsetは4-byte alignedでmemory_size未満 |
| Relocation | 対応sectionが存在し、offsetは4-byte aligned。Abs32/Call16/Jump16は4 byte、slotは24 byteをdata内に含む |
| branch relocation | Text内のpatch siteであり、解決targetとaddend後のaddressも4-byte alignedなTextを指す |
| patch所有範囲 | relocationのpatch/template範囲は同section内で互いに分離。symbolのoffsetと参照先はslotの内部に入らず、有効な入口を指す |

明示Imported宣言も解決対象となり、relocationはImported宣言の有無によらずexportを参照できる。\
Local symbolはそのObject内で解決する。\
Exported symbolは共有namespaceを形成する。\
undefined import、重複export、不正symbol range、invalid alignment、overlap patch site、branch slot内部への参照はlink errorを返す。\
LinkPlanのRAM baseとstack_sizeは4-byte aligned、checked計算したRAM_ENDは8-byte alignedで、`0 < stack_size < memory_size`を満たす。\
read_onlyは正のsize、4-byte alignedなbase/endを持ち、RAMと分離する。\
LinkPlanがRAM base/size、stack reservation、optional read-only region、entry/heap/stack境界symbolを指定し、section配置とaddress/offset arithmeticはchecked範囲で行う。\
容量不足は指定RAM/stack容量に対するlink errorとなる。\
Text、Rodata、Data、Bssの順で配置し、各Objectのsectionを入力Object順で`max(alignment, 4)`へ揃える。\
同kindの結合sectionのalignmentは入力の`max(alignment, 4)`の最大値とする。\
read_only領域を指定した場合はText/Rodataをそこへ、Data/BssをRAMへ配置し、両領域は分離する。\
entry_symbol、heap_base_symbol、stack_bottom_symbolは非空で互いに異なる。\
heap/stack境界名と入力ObjectのLocal/Exported定義との衝突はlink errorとなる。\
entryはExported Functionであり、branch relocationの解決先は実行可能なText内である。\
reserved runtime symbolとの衝突、不正なimport定義、checked address overflowもlink errorとなる。\
LinkedImageは最終load address、initialized bytes、section memory size、entry point、heap/stack boundaries、resolved symbolsを記録する。\
loaderはdataをloadし、各sectionのdata末尾からmemory_sizeまでをzero初期化する。

## relocationと中継slot
<!-- ond-spec-id: linking/relocation-slots -->

`Abs32`はsymbol addressにsigned addendを加えたchecked 32-bit値をlittle-endianで書き込む。\
`Call16`と`Jump16`の入力wordは、opcode bits31..28=`1100`のJZ命令であることを要求する。\
rd=bits27..24、condition=bits23..20、base=bits19..16、immediate=bits15..0とし、次の形を検査する。

| 種別 | 入力形 |
| --- | --- |
| Call16 | rd=r15、condition=r0、base=r0、任意のimmediate |
| Jump16 | rd=r0、condition=r0〜r15の任意register、base=r0、任意のimmediate |

siteはTextの4-byte aligned位置、targetはsymbol addressにsigned addendをchecked加算したu32 addressであり、4-byte alignedなTextを指す。\
site+4もchecked u32で求め、数学的整数`(target-(site+4))/4`が整数かつ[-32768,32767]に入ることを要求する。\
負のdの下位16bitはd mod 65536（0〜65535）のtwo's-complement表現とする。\
dをそのdisplacementとし、`(word & 0xffff0000) | (d & 0xffff)`の4byte little-endian値をpatchする。\
入力形違反、overflow、距離範囲外はlink errorとなる。\
この2種は専用slotへ自動拡張しない。

CallSlot/JumpSlotは[固定長slot規範](#固定長slot規範)と[正確なtemplate](#callslotjumpslot-relocation)に従う。

## 呼出側と実行環境の境界
<!-- ond-spec-id: linking/sdk-boundary -->

呼出側と実行環境の役割は[責務の定義](./build/packages.md#責務)に従う。\
ProgramMetadataがinitializer orderとmainをlink入力へ渡す。\
共有startupはruntime ABI glueを生成し、termination instructionsはcallerが指定する。\
呼出側はRAM範囲とstack量をLinkPlanへ指定し、正常終了後の制御経路をtermination instructionsとして指定する。\
実行環境はmemory mapとdevice構成を定め、loaderはsection load、zero初期化、SP設定、entryへの制御移行を行う。\
配布ツールはLinkedImageとassetsを配布形式またはROMへまとめる。

## 固定長slot規範
<!-- ond-spec-id: targets/kagura-v1/long-branch-relocation-slots -->

各siteに固定長の専用slotを予約し、slotを含めてsymbol size、relocation offsetと配置addressを定める。\
link時はこの予約範囲内をpatchし、近距離と遠距離の選択によって他のsiteやAbs32のaddressを変化させない。

### CallSlot/JumpSlot relocation
<!-- ond-spec-id: targets/kagura-v1/call-jump-slot-relocation -->

CallSlot/JumpSlot relocationはText内の4-byte alignedな**6 word（24 byte）**を所有する。\
offsetはword 0を指し、入力Objectのdata/memory_size内に全slotを含める。

| word | link前 | 遠距離patch後 |
| --- | --- | --- |
| 0 | `JZ r15,r0,r0,0`（CallSlot）または`JZ r0,condition,r0,0`（JumpSlot） | 同じ条件/link registerでword 2へjump |
| 1 | `JZ r0,r0,r0,4` | 変更なし。word 6へskip |
| 2 | 0 | `JZ r12,r0,r0,0`（word 3のPC取得） |
| 3 | 0 | `LDW r12,r12,r0,8`（word 5のliteral読出し） |
| 4 | 0 | `JZ r0,r0,r12,0`（最終targetへjump） |
| 5 | 0 | `symbol address + addend`の32-bit literal |

近距離ならword 0のsigned-16-bit immediateだけをpatchする。\
call時のr15はword 1を指し、calleeのreturn後にword 6へskipする。\
thunkは再callせず、引数r1〜r6、stack引数、SP、callee-save、r15を保持し、r12だけを変更する。\
JumpSlotの非選択経路もword 1から継続し、全register値を保持する。

linkerはslotの入力範囲・template・alignment、解決先symbol、addend後の32-bit範囲とalignment、最終image容量を検査する。\
他のsymbol/relocationを予約領域内へ置いたり、内部へ通常分岐してはならない。\
Call16/Jump16は自動拡張せず、距離超過をlink時に拒否する。

startupのmain/initializer callもCallSlot形式に従う。

## 長距離callと分岐
<!-- ond-spec-id: targets/kagura-v1/mir-long-distance-control-flow -->

関数内branchは全32-bit address範囲の到達先へ制御を移せる。\
関数間call/jumpの到達範囲はObject relocationの種別によって定まる。\
条件分岐は条件成立時に到達先へ移り、非成立時は継続経路を実行する。\
call経路は元のreturn addressと引数を保持する。\
scratchは[ABI](./targets/kagura-v1/abi.md#1-registers)と上記slot契約に従う。
