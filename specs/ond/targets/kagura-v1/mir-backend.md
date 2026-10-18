# MIR → Kagura backend の境界設計
<!-- ond-spec-id: targets/kagura-v1/mir-backend -->

本書はMIRをKagura Objectへ変換するbackendの契約を定義する。
実装検証の証拠は[Compiler適合記録](../../../../docs/compiler/kagura-v1-conformance-record.md)を参照。
[Machine Profile](./machine.md) と [ABI](./abi.md) を規範とし、本書はそのMIRへの適用を整理する。
本書の対象はMIR loweringとKagura backendの契約であり、新しい言語機能とEnbuカートリッジ規格は含めない。

## 1. 入出力と責務
<!-- ond-spec-id: targets/kagura-v1/mir-backend-io-and-responsibilities -->

入力は検証済みの `ond_compiler_core::mir::Project`。型は同projectのTypeTableで解決する。
AST/HIRから型や定数を再解釈しない。未対応MIRをAST backendへ暗黙fallbackさせない。
未対応MIRにはunsupported診断とsource spanを返し、部分的なobjectを成功結果として返さない。
MIR validator違反と、targetのサイズ制限・未対応命令は診断上区別する。

| 層 | 所有するもの | 所有しないもの |
| --- | --- | --- |
| core/MIR | 型同一性、値・評価順、CFG、snapshot、check、symbolic global/init順 | byte layout、register、ABI、section、ROM bank |
| Kagura backend | DataLayout、ABI lowering、frame、命令選択、runtime依存、object生成 | sourceの再型検査、最終address/カートリッジ配置 |
| linker/startup | symbol解決、relocation、section配置、RAM容量検査、entry生成 | 言語式の意味解析 |
| cartridge builder / machine profile | 配布形式、load、MMIO・起動環境 | MIR型・演算の再解釈 |

SourceDbは診断表示に利用してよいが、LoadedProject/ASTを意味情報の入力にしない。

O0ではscalar ValueIdを4-byte slotへspillできる。SSA変換や最適化の有無はこの契約の対象外とする。
block parameterとedge argumentを扱う。edge argumentはparallel copyとし、循環copyはtemporaryで解く。
Branchのedge copyは選択されたedgeだけで実行する。

## 2. DataLayout
<!-- ond-spec-id: targets/kagura-v1/mir-data-layout -->

| MIR型 | size / alignment | memory表現 |
| --- | --- | --- |
| bool / u8 / i8 | 1 / 1 | bool storeは0/1、loadは非0をtrueに正規化 |
| u16 / i16 | 2 / 2 | little-endian |
| u32 / i32 / f32 | 4 / 4 | f32はbinary32 bits |
| Pointer / Function | 4 / 4 | bus address / entry address。nilは0 |
| Array(N,T) | N×size(T) / align(T) | headerなし、stride=size(T) |
| Struct | 下記 | 宣言順field、自然alignment、末尾padding |
| Defined | underlyingと同じ | 元のTypeIdは失わない |

structは各fieldで `offset = align_up(offset, align(field))`、次にsizeを加える。
最大field alignmentで最終sizeを切り上げる。例: `{a:u8,b:u16,c:u8}` はoffset 0/2/4、size 6、alignment 2。
arrayのstrideはstruct末尾paddingを含む。この例の配列2要素はsize 12。
関数型はABI規定のfunction pointerとして扱い、parameter/result型のlayoutを再帰的に埋め込まない。
pointer経由の再帰でlayout計算を続けない。値としての循環はvalidatorで拒否する。

サイズ・offset・乗算・alignment切上げはcheckedな広い整数で計算し、32-bit表現へ切り詰めない。
固定長配列は [Data Layout](./machine.md#5-data-layout) に従い、`N * sizeof(T) > 2^32-1` なら frontend で compile error とする。\
型がu32サイズ内でもframe/object/startupが容量を超える場合は該当工程で拒否する。
raw pointerの実行時offsetだけは仕様どおりmod 2^32であり、静的layoutのoverflow拒否とは別。
zeroは全byte 0。paddingの値は意味を持たないが、Ordered accessをpaddingへの追加accessに拡張しない。

scalarのlocal slotはABIに従い4 byteを予約してよいが、addressableなi8等の値の読み書き幅は1/2 byteを維持する。
予約サイズと型のsizeofを混同しない。文字列literalは通常のbyte配列でありstr型・descriptor・終端NULは存在しない。

## 3. ABI lowering
<!-- ond-spec-id: targets/kagura-v1/mir-abi-lowering -->

既存ABIを変更せず使用する。

- r1..r6: 引数、r1..r4: register戻り値。r7..r11: callee-save、r12..r13/r15: caller-save、r14: SP。
- call境界はSPを8-byte alignmentへ揃える。ABI wordは4 byte。
- integer/bool/f32/pointer/functionはDirect-1。array/struct/interfaceはsizeに関係なくIndirect。
- Indirect引数はcaller-owned copyのpointerを1 wordとして渡す。calleeは元のaggregateをaliasしない。
- 全戻り値がDirectかつ4 word以下ならregister。aggregateを1つでも含むか5 word以上なら全戻り値をreturn areaへ置く。
- memory returnのhidden pointerを先頭wordに追加し、その後にuser引数を並べる。先頭6 wordがregister、残りがCFA+0,+4,...。
- return areaはDirect値に4-byte slot、aggregateに通常layoutを用いる。各fieldを最大4-byteの自然alignmentに揃える。

境界例:

| signatureの形 | 配置 |
| --- | --- |
| 7個のDirect引数、戻り値なし | r1..r6、7個目はCFA+0。stack引数領域は8 byte |
| 6個のDirect引数、aggregate戻り値 | r1=hidden return pointer、user1..5=r2..r6、user6=CFA+0 |
| 戻り値(i8,u16,bool,f32) | r1..r4 |
| 戻り値(u8,[3]u8,u16) | return area offset 0/4/8、通常のrecord末尾は12 byte |

狭い符号付き整数もABI wordの上位bitは0にする（i8 -1なら0x000000ff）。
backend内部では「下位N bitが値、上位0」を標準表現とする。
signed比較・除算・算術右shift時だけN bitから32 bitへsign-extendし、結果を型幅へ戻す。
MIRのInteger.bitsはu64なので、保存形式の上位bitをそのままguestへ持ち込まない。
boolは0/1、f32はbitsを渡す。数値変換とbit transportを区別する。

callee/引数はMIRの評価済みValueを使い、register割当時にsource式を再評価しない。
aggregate ValueIdはimmutable snapshotとする。O0 backendでは独立storageへcopyし、後続Storeやcallで値が変わらないようにする。
Indirect引数copy・戻り値area・live snapshot・outgoing引数領域は寿命が重なる限り共有しない。
破棄されたcall結果もABIに従い受け取る（必要ならreturn areaを確保する）。

frameは8-byte倍数。r15/callee-save保存、locals/spills/snapshot、child return area、最大outgoing領域を含める。
全てのhelper callもcallとして扱い、r15とlive caller-save値を保護する。
SP減算underflowとSTACK_BOTTOMを、frameへ書く前に検査する。0-byte frameは省略可能。

## 4. MIR命令の対応
<!-- ond-spec-id: targets/kagura-v1/mir-instruction-lowering -->

Kagura ISAは12命令で、SUB/DIV/浮動小数点/CALL/RETは独立opcodeではない。
以下のCALL/RET等は説明用の疑似操作であり、最終的にはJZ等へ展開する。

| MIR | Kaguraへのlowering / 注意 |
| --- | --- |
| Constant Integer/Bool/Nil/Float32 | 32-bit即値構築。sext16即値に収まらない値は複数命令。Float32はbits |
| Constant Zero | scalarは0、aggregateは型付きstorageのzero初期化。巨大arrayを要素数分のIRへ展開しない |
| Constant Function | entry symbolへのaddress relocation。link前の最終addressを仮定しない |
| Negate / BitNot / LogicalNot | integerはNAND+ADD / NAND / CMP。f32 Negateは符号反転とNaN契約を満たす処理 |
| Add / Subtract / Multiply | ADD / 二の補数+ADD / MUL。狭い型は結果をmask |
| BitAnd / BitOr / BitXor / BitAndNot | NANDの合成、結果の型幅を維持 |
| ShiftLeft / ShiftRightLogical / ShiftRightArithmetic | SHIFT。countをOndの型幅Nでmod化。i8/i16の算術shiftはsign-extendが必要 |
| Equal/NotEqual、signed/unsignedの大小・以下・以上 | CMPのeq/lt、operand交換・反転。signed操作の前に正規化 |
| Divide / Remainder | ソフトウェア列またはhelper。NonZeroを先行させる。signed min/-1除算はfault、remainderは0 |
| FloatAdd/Subtract/Multiply/Divide | soft-float helper。binary32丸め、canonical NaN、signed zero。0除算を整数faultにしない |
| FloatEqual/NotEqual/Less/LessEqual/Greater/GreaterEqual | NaNを含むfloat比較helper。整数CMPのbits比較で代用しない |
| Cast Infallible / Checked | integerはsource符号に応じた拡張後にdestination幅へwrap。integer↔pointerも32-bit target表現。int↔f32は数値変換helper、f32→intはdestination幅の範囲を検査 |
| AddressOf | 型付きPlaceからaddressを計算。追加のnil/lifetime/bounds検査を入れない |
| Load / Store | scalarはLDB/LDH/LDW、STB/STH/STW。型幅1回のbus access。bool正規化。aggregateはsnapshot/型付きcopy |
| AggregateCopy | field/element単位のcopy。Orderedではaccess幅・順序を保持、paddingを含むmemcpyで代用しない。alias時もsource snapshotを維持 |
| Check Bounds | signed indexの負数判定とunsigned index<length。失敗はbounds trap |
| Check NonZero / SignedDivisionOverflow | 型幅に応じた比較とfault経路。validatorで要求されるcheckを消さない |
| Call Direct / Indirect | ABI lowering後、directはJZ r15,r0,r0,rel、indirectはJZ r15,r0,target,0。targetにはr0以外を使用 |
| Call Intrinsic | §5。runtime helperもABIのcaller-save規則に従う |
| Jump / Branch | edge parallel copy後にJZ。boolは0/1。遠距離分岐は§7 |
| Return | ABI戻り値格納、frame/register復元、JZ r0,r0,r15,0 |
| Trap Bounds/DivisionByZero/SignedDivisionOverflow/InvalidConversion/StackOverflow/Explicit | stackを追加使用しないnon-return fault path。Explicitのmessageは命令列へ埋め込まずdebug metadataだけへ出す。helper名は内部実装事項 |
| Unreachable | 防御的に予約opcode faultをemit。次のblockへfallthroughさせない |

Ordered Load/Store/Copyとfaultする命令・callの順序を保持する。未使用Valueでもaccess/check/callを落とさない。
backendが対応しないChecked演算はwrapへlowerせず、診断する。
Kaguraの16-bit immediate、word-relative分岐、alignment faultに注意する。
soft-floatの既存 [runtime_support.ond](../../../../crates/compiler/src/runtime_support.ond) は再利用し、各helperは本書の演算契約を満たす。
helperの生成が同じhelperを要求する循環を避け、整数primitive等を基底として依存閉包を検査する。

## 5. Intrinsic / global / startup
<!-- ond-spec-id: targets/kagura-v1/mir-intrinsic-global-startup -->

- New(T): layoutのsize/alignmentで確保し、成功時のみzero化。failureはnil。ゼロサイズは§7。
- Alloc / Free: machine profileのallocator。alloc(0)/failureはnil、free(nil)はno-op。
- debug metadata: function/range/source/unwindをObjectと並行して生成する。linkerが最終addressへ解決し、VMは任意でfault理由とstack traceの表示に利用できる。metadataの有無は実行時意味論を変えない。
- Global: zero storageをBssへ出す。初期値のsource再評価やData化による順序変更はしない。
- initializer: MIRの内部関数を通常関数としてemitし、Project.initialization_orderを明示的なstartup計画へ渡す。
- startup: 全globalのzeroが完了した後、指定順のinitializer、最後にmain.main。正常return後の終了0はmachine固有entryが担当する。
- 既存Object.importsから順序を再構成して一致を仮定しない。MIRの到達範囲・順序との照合または明示planを受け渡す。
- symbol/relocation/section形式を再利用しても、EnbuのbankやMMIOをcoreへ追加しない。

## 6. Backendの構成要件
<!-- ond-spec-id: targets/kagura-v1/mir-backend-architecture -->

Backendの入口はcoreで生成・検証したMIRを受け取る。AST/HIRから型・literalの意味を再解釈せず、Kaguraのlayout・ABI・命令制約をbackendで適用する。

Backend適合の根拠となる実行証拠は[Compiler適合記録](../../../../docs/compiler/kagura-v1-conformance-record.md)に記録する。

## 7. Backendの追加契約
<!-- ond-spec-id: targets/kagura-v1/mir-boundary-decisions -->

以下の項は本書の規範契約である。

### 7.1 ゼロサイズ型
<!-- ond-spec-id: targets/kagura-v1/mir-zero-sized-types -->

空structはsize=0/align=1、[0]Tはsize=0/align(T)とする。
address取得用slot・Indirect引数temporary等のゼロサイズstorageは最低1 byteを予約し、nilと区別する。型のsizeofは0のままとし、異なるゼロサイズ値のアドレス同一性に新しい言語保証は追加しない。frame内の予約はalignment等により1 byteより大きくてもよい。
new(T)はsize=0でも最低payloadを確保し、成功時は非nil、失敗時はnilを返す。alloc(0)はnilを返す。

### 7.2 Ordered aggregate
<!-- ond-spec-id: targets/kagura-v1/mir-ordered-aggregate -->

宣言/index昇順で再帰的にscalar accessし、paddingへ触れず、source snapshotを完成後にdestinationへ書く契約とする。
全体のatomicityは保証しない。途中faultで完了済みの書き込みやMMIOの副作用を巻き戻さない。
結果が同じという理由でもOrdered accessの順序変更は許可しない。machine仕様にも同じ契約を記載する。

### 7.3 遠距離call/分岐
<!-- ond-spec-id: targets/kagura-v1/mir-long-distance-control-flow -->

**関数内はbackendの分岐拡張、関数間はリンカの中継コード（thunk/trampoline）**で対応する契約とする。

- backendは同一関数内のblock配置と距離を確認し、届かない分岐をaddress構築＋間接jumpへ拡張する。条件分岐では長距離経路を選んだ場合だけそのjumpを実行する。
- リンカは関数間の最終距離を確認し、直接call/jumpで届かない場合、呼出元から到達できる位置へ中継コードを配置する。
- 中継コードは最終entry addressを一時registerへ構築して間接jumpする。全32-bit addressを指定できるので、通常は1回の中継で足り、短距離jumpを何段も連鎖させる設計にはしない。
- callの中継は元のcallが設定したr15を保持し、再callしない。calleeは元の呼出元へ直接returnする。引数register・stack引数・SPも維持する。
- scratch registerのclobberをbackend/linker共通の契約にする。caller-saveであっても関数内jumpではliveな値を壊してよいわけではない。具体的なregister予約・relocation表現は§11で固定する。
- 同じ目的地へ向かう呼出元は、全てが到達できる場合に中継コードを共有できる。共有を正しさの条件にはしない。
- 中継コード挿入によって他の距離も変わる方式では、配置・距離判定を再計算し、全relocationが有効になるまで調整する。§11の固定長予約方式を用いて配置変化を防ぐ。未解決・容量超過・配置不能は明示エラーとする。
- 中継コードは通常実行がfallthroughする位置へ無防備に挿入しない。全てのentryを4-byte alignedに保ち、関数内分岐の距離保証を壊さない配置単位・挿入点を設計する。

アドレス構築の命令列生成はKagura用の低レベル部品としてbackendとリンカから共有する。
関数内の長距離分岐はbackendが、中継コードの命令列はリンカが生成し、最終addressはrelocation/layoutに基づきリンカが確定する。
ObjectのAbs32/Call16/Jump16に加え、literalを含む中継領域を予約したCallSlot/JumpSlotを用いる（§11）。

Call16/Jump16は中継領域もscratch契約も持たないため、距離超過をlinkで拒否する。MIR生成とstartupはCallSlotを使用し、無言の切り詰めやAST fallbackを行わない。
適合試験では到達範囲の境界・正負両方向・複数siteの配置不変・引数/r15保持・条件分岐の非選択経路・容量超過を検証する。中継共有と可変長再配置はこの契約で要求しない。



## 8. MIR backend契約
<!-- ond-spec-id: targets/kagura-v1/mir-backend-object-contract -->

`ond-compiler::mir_backend::codegen_objects(&mir::Project)`は検証済みMIRを入力とし、Objectを生成する。Linkerの`link_objects`はObject列からLinkedImageを生成する。未対応MIRはsource span付き診断とし、部分Objectや別backendへのfallbackを成功結果として返してはならない。MIR validator違反とtarget制約違反を診断上区別する。起動環境とROM化は機種SDKが所有する。

Backendは、Kagura v1で定義されたscalar、pointer、function pointer、array、struct、defined typeと、MIRの値・memory・control flow・call・trap操作をABIに従ってlowerする。aggregate値はsnapshotとして扱い、copy時はsource読み取りを完了してからdestinationへ宣言/index順で書く。paddingへaccessしてはならない。

全globalは自然alignmentでBSSへ置き、静的aggregate定数と文字列literalは自然alignmentでRodataへ置く。ゼロサイズstorageにはidentity用の最低1 byteを予約する。global initializerはMIR initializerとして実行し、全BSS zero化の後、`Project.initialization_order`で一度ずつ呼び、最後に`main.main`を呼ぶ。Object順やimport名順から初期化順を再構成しない。rootから到達しないpackageのinitializerを呼ばない。

Integer Checked演算をwrapへ置き換えてはならず、表現できない演算は診断する。ABIおよびoffset規則は§3、§10、§11、§12に従う。

## 9. Runtime helperの供給
<!-- ond-spec-id: targets/kagura-v1/runtime-helper-supply -->

`mir_backend::runtime`はcompiler所有のOnd helperを、compiler-core → MIR backend → Objectの経路でビルドする。Ondの言語仕様やuser importにruntime専用機能を追加しない。

- 各`Helper`はentryとscalar ABIを定義し、`symbol()`で予約済みlinker symbol、`signature()`で型を提供する。helper集合はsoft-float、整数除算/剰余、allocatorを含む。
- backendがhelper symbolへのcall/address relocationを生成すると、`codegen_objects`末尾の供給処理が要求を収集する。entryのABIを検証し、direct callとFunction定数から依存関数をたどって一度だけemitする。要求がなければruntimeソースのコンパイルも追加Objectも発生しない。
- userとruntimeは別々のTypeTableでコンパイルし、Objectと明示ABIで接続する。内部関数も制御文字を含む予約namespaceへ改名し、user定義の衝突・未知の要求を診断する。
- runtime sourceはメモリ上で処理でき、物理ファイルの存在に依存しない。合成したfrontend用entryはuser startupへ含めない。
- 通常のOnd関数の再帰を含む依存関係は有限の依存閉包として扱う。runtime生成は再帰的な再供給を行わず、未対応primitiveまたはloweringで生じた閉包外の依存を診断する。runtime helperはglobal storage/startup副作用を持ってはならない。allocatorは状態のaddressを引数で受け取るため、この制限内で動作する。
- `runtime_support.ond`を使用する。compiler所有の`__sf_invalid`だけはMIRのTrap primitiveに置換し、変換エラーの停止を整数除算helperへ依存させない。

## 10. 数値演算・allocator
<!-- ond-spec-id: targets/kagura-v1/numeric-and-allocator-lowering -->

- `numeric`モジュールで整数除算/剰余、float演算/比較/negate、整数↔f32 castをhelperへ選択する。f32はstorage・引数・戻り値では32-bit patternをそのまま運ぶ。signed狭整数はhelper前にsign-extendし、結果を型幅へ正規化する。暗黙のhelper callも既存frameのr15保存・Value spillを使用する。helper引数は最大4 wordなので追加のoutgoing stack areaは不要。
- 除算はinteger.ondの32段のunsigned long divisionを基底とし、最上位carryを落とさず比較する。signed処理は絶対値のunsigned bit patternと符号の復元で実装する。NonZero/SignedDivisionOverflowはMIRの位置でfaultを生成する。signed min/-1の除算はfault、剰余は0。helper自身は/や%を使わない。
- f32 castは0方向に丸めた結果の範囲を検査する。unsignedへの-0.5は0、signed狭型では切り詰め前のfractionではなく整数結果を範囲検査する。NaN/Infや範囲外はTrap primitiveで停止する。
- heap.ondのalloc/free/newへ、linkerの`__ond_heap_base`、`__ond_stack_bottom`、startup BSSの`__ond_heap_cursor`のaddressを渡す。cursor=0なら初回allocでheap_baseへ設定する。package初期化中のnewも同じ状態を使用する。
- first-fit、余りがheader+最低payload以上のときの分割、free時の前後結合、末尾回収を実装する。size計算overflow・容量不足ではnilを返す。newは成功時だけ型size分をzero化し、size=0でも最低payloadを確保する。alloc(0)=nil、free(nil)=no-op。有効なallocation pointerをfreeする契約であり、不正freeやheap header破壊に対する安全性は保証しない。

長距離call/branchは§11、frameと引数offsetは§12に従う。

## 11. 長距離分岐とObjectの中継slot
<!-- ond-spec-id: targets/kagura-v1/long-branch-relocation-slots -->

各siteに**固定長の専用中継領域を予約する**。予約分を含めてblock label、symbol size、relocation offset、RAM/heap配置を決定し、link時には命令を書き換えるだけで挿入・削除しない。これにより、他の分岐距離やAbs32を再配置する必要がない。近距離も予約領域を保持するためコードサイズが増える。中継共有・不要領域の圧縮・可変長relaxationはこの契約で要求しない。

### 11.1 関数内branch
<!-- ond-spec-id: targets/kagura-v1/function-local-branches -->

- `Emitter.branch`は7 word（28 byte）を予約する。word 0が条件付き/無条件JZ、word 1がword 7へのskip、word 2〜6が中継領域。
- `(target - (site + 4))/4`がsigned 16-bitに収まる場合、word 0を直接patchする。それ以外ではword 0をword 2へ向ける。
- 中継はPCをr12へ取得し、literalからr13へ相対byte差分を読み、r12+r13へ間接jumpする。literalはword 3のaddressからtargetへのmodulo-2^32差分。text全体の32-bitサイズ検査を行った上で生成し、Object配置に依存しない。
- taken経路のr12/r13のみclobber可能とする。O0ではlive MIR値・edge copyをstackへ保存済みであり、内部数値checkもこの契約に従う。非選択経路はword 1で中継命令とliteralをskipする。
- prologueのfault skip等、命令列内で距離が固定された短いJZは直接encodeする。

### 11.2 CallSlot / JumpSlot relocation
<!-- ond-spec-id: targets/kagura-v1/call-jump-slot-relocation -->

CallSlot/JumpSlot relocationはText内の4-byte alignedな**6 word（24 byte）**を所有する。offsetはword 0を指し、入力Objectのdata/memory_size内に全slotを含める。

| word | link前 | 遠距離patch後 |
| --- | --- | --- |
| 0 | `JZ r15,r0,r0,0`（CallSlot）または`JZ r0,condition,r0,0`（JumpSlot） | 同じ条件/link registerでword 2へjump |
| 1 | `JZ r0,r0,r0,4` | 変更なし。word 6へskip |
| 2 | 0 | `JZ r12,r0,r0,0`（word 3のPC取得） |
| 3 | 0 | `LDW r12,r12,r0,8`（word 5のliteral読出し） |
| 4 | 0 | `JZ r0,r0,r12,0`（最終targetへjump） |
| 5 | 0 | `symbol address + addend`の32-bit literal |

近距離ならword 0のsigned-16-bit immediateだけをpatchする。call時のr15はword 1を指し、calleeのreturn後にword 6へskipする。thunkは再callせず、引数r1〜r6、stack引数、SP、callee-save、r15を保持し、r12だけを変更する。JumpSlotの非選択経路もword 1から継続し、registerを変更しない。

linkerはslotの入力範囲・template・alignment、解決先symbol、addend後の32-bit範囲とalignment、最終image容量を検査する。他のsymbol/relocationを予約領域内へ置いたり、内部へ通常分岐してはならない。Call16/Jump16は自動拡張せず、距離超過をlink時に拒否する。

低レベルencode・PC取得/literal読出し・中継命令列を`kagura_encoding`へ集約し、MIR emitterと`linker/thunks`で共有する。startupのmain/initializer callにもCallSlotを使う。

### 11.3 適合条件
<!-- ond-spec-id: targets/kagura-v1/relocation-verification -->

- signed word距離32767/32768/-32768/-32769で、近距離/中継選択後の到達先が正しいことを確認する。
- 関数内相対中継は複数load addressで有効にし、条件不成立時にscratchを保持する。
- linker call/jumpはregister・r15を保持し、return後にliteralをskipする。addendと複数site/Abs32の配置を変化させない。
- 128 KiBを超える関数内の長距離loop/条件分岐、startup/main/initializer call、7引数call、function pointer callを実行でき、SP/callee-saveを保持する。
- 不正template・範囲/alignment・未解決symbol・addend overflow・RAM不足・旧relocationの距離超過は診断する。

## 12. Stack frame容量
<!-- ond-spec-id: targets/kagura-v1/large-stack-frame -->

frameおよびincoming引数offsetにはsigned-16-bitを超える値を許す。小さなoffsetでは即値命令を使用し、32767を超えるoffsetでは32-bit即値とSPの加算で実アドレスを生成する。local、spill、edge snapshot、argument copy、return area、保存r15、incoming/outgoing引数に共通の処理を使用する。

- stack load/storeの大offset展開はr13をscratchとして使用する。データregisterがr13の場合だけr12を使う。r12に保持したstack引数をstoreする際も値を壊さない。これらのscratchにアクセスを越えるlive値を置かない。
- stack address生成は出力registerだけを変更する。SP自体は変更せず、r1〜r6への引数address構築時も既に設定した引数を維持する。
- prologueは32-bit frameサイズを生成し、減算underflowとSTACK_BOTTOMを検査する。検査成功前にSPを更新したりframeへ書き込んだりしない。epilogueも大きなサイズをregisterへ生成してSPを復元する。戻り値r1〜r4と復元したr15を維持する。
- slot予約・alignment・frameとincoming領域の合計は32-bit checked演算で計算し、overflowは診断する。設定されたstack容量を自動拡張するものではなく、実行時のstack不足はfaultになる。

Aggregate loweringは要素ごとの命令展開を用いる。大きな配列のコード量、実メモリ容量、32-bitアドレス範囲は独立した制約とする。
## 13. 標準経路と責務の分離
<!-- ond-spec-id: targets/kagura-v1/standard-path-responsibilities -->

`compile_project` / `compile_loaded_project` / overrides APIはMIR codegenでObjectを生成する。SDKがLinkPlan・初期化順序・終了処理を指定してlinkし、ROMへ配置する。package初期化順序はMIRのinitialization_orderだけを用いる。

Ond製soft-float runtimeはMIRからcompileする。LSPはcompiler-coreを用い、targetのcodegenやframe制限を診断に含めない。
