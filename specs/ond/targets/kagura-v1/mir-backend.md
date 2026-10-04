# MIR → Kagura backend の境界設計

作成: 2026-09-20。状態: 境界設計と§8〜12の実装記録。全仕様への適合完了を意味しない。
検証済み範囲・不足・追加順序は[適合チェックリスト](./conformance-checklist.md)を参照。
既存の [Machine Profile](./machine.md) と [ABI](./abi.md) を規範とし、本書はそのMIRへの適用を整理する。
未決定事項は§7に隔離する。新しい言語機能、Enbuのカートリッジ規格、最適化器は今回の対象外。

## 1. 入出力と責務

入力は検証済みの `ond_compiler_core::mir::Project`。型は同projectのTypeTableで解決する。
AST/HIRから型や定数を再解釈しない。未対応MIRをAST backendへ暗黙fallbackさせない。
初期実装ではunsupported診断とsource spanを返し、部分的なobjectを成功結果として返さない。
MIR validator違反と、targetのサイズ制限・未対応命令は診断上区別する。

| 層 | 所有するもの | 所有しないもの |
| --- | --- | --- |
| core/MIR | 型同一性、値・評価順、CFG、snapshot、check、symbolic global/init順 | byte layout、register、ABI、section、ROM bank |
| Kagura backend | DataLayout、ABI lowering、frame、命令選択、runtime依存、object生成 | sourceの再型検査、最終address/カートリッジ配置 |
| linker/startup | symbol解決、relocation、section配置、RAM容量検査、entry生成 | 言語式の意味解析 |
| cartridge builder / machine profile | 配布形式、load、MMIO・起動環境 | MIR型・演算の再解釈 |

まず既存compiler crate内の通常のRust moduleとして分割する案とする。
`layout` / `abi` / `frame` / `lower` / `select` / `emit` / `runtime` を責務単位とし、独立crate化は後で可能にする。
概念上のAPIは `layout(TypeId)`、`classify(FunctionType)`、`lower(&mir::Project, &TargetOptions)`。
出力は当初、既存 [Object](../../../../crates/compiler/src/object.rs) を再利用する。
SourceDbは診断表示に利用してよいが、LoadedProject/ASTを意味情報の入力にしない。

SSA変換・最適化は着手条件にしない。O0ではscalar ValueIdを4-byte slotへspillする実装でよい。
block parameterは実際に短絡式で使用されるため初期対応に含める。edge argumentはparallel copyとして扱い、循環copyはtemporaryで解く。
Branchのedge copyは選択されたedgeだけで実行する。

## 2. DataLayout

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
型がu32サイズ内でもframe/object/startupが容量を超える場合は該当工程で拒否する。
raw pointerの実行時offsetだけは仕様どおりmod 2^32であり、静的layoutのoverflow拒否とは別。
zeroは全byte 0。paddingの値は意味を持たないが、Ordered accessをpaddingへの追加accessに拡張しない。

scalarのlocal slotはABIに従い4 byteを予約してよいが、addressableなi8等の値の読み書き幅は1/2 byteを維持する。
予約サイズと型のsizeofを混同しない。文字列literalは通常のbyte配列でありstr型・descriptor・終端NULは存在しない。

## 3. ABI lowering

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
backend内部も「下位N bitが値、上位0」を標準表現とする案を採る。
signed比較・除算・算術右shift時だけN bitから32 bitへsign-extendし、結果を型幅へ戻す。
MIRのInteger.bitsはu64なので、保存形式の上位bitをそのままguestへ持ち込まない。
boolは0/1、f32はbitsを渡す。数値変換とbit transportを区別する。

callee/引数はMIRの評価済みValueを使い、register割当時にsource式を再評価しない。
aggregate ValueIdはimmutable snapshot。Load時点で独立storageへcopyする保守的実装を基本とし、後続Storeやcallで値が変わらないようにする。
Indirect引数copy・戻り値area・live snapshot・outgoing引数領域は寿命が重なる限り共有しない。
破棄されたcall結果もABIに従い受け取る（必要ならreturn areaを確保する）。

frameは8-byte倍数。r15/callee-save保存、locals/spills/snapshot、child return area、最大outgoing領域を含める。
全てのhelper callもcallとして扱い、r15とlive caller-save値を保護する。
SP減算underflowとSTACK_BOTTOMを、frameへ書く前に検査する。0-byte frameは省略可能。

## 4. MIR命令の対応

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
現在有効な整数Checked演算は主にdiv/rem。将来のchecked Add等を単なるwrapへlowerせず、対応できない組み合わせは診断する。
Kaguraの16-bit immediate、word-relative分岐、alignment faultに注意する。
soft-floatの既存 [runtime_support.ond](../../../../crates/compiler/src/runtime_support.ond) は再利用候補であり、新経路での正しさは別途検証する。
helperの生成が同じhelperを要求する循環を避け、整数primitive等を基底として依存閉包を検査する。

## 5. Intrinsic / global / startup

- New(T): layoutのsize/alignmentで確保し、成功時のみzero化。failureはnil。ゼロサイズは§7。
- Alloc / Free: machine profileのallocator。alloc(0)/failureはnil、free(nil)はno-op。
- Load32 / Store32: addressを32-bitとしてLDW/STWを1回実行。alignment/MMIO faultはbus/CPUへ委ねる。
- debug metadata: function/range/source/unwindをObjectと並行して生成する。linkerが最終addressへ解決し、VMは任意でfault理由とstack traceの表示に利用できる。metadataの有無は実行時意味論を変えない。
- Global: 当初はzero storageをBssへ出す。初期値のsource再評価やData化による順序変更はしない。
- initializer: MIRの内部関数を通常関数としてemitし、Project.initialization_orderを明示的なstartup計画へ渡す。
- startup: 全globalのzeroが完了した後、指定順のinitializer、最後にmain.main。正常return後の終了0はmachine固有entryが担当する。
- 既存Object.importsから順序を再構成して一致を仮定しない。MIRの到達範囲・順序との照合または明示planの受渡しを接続工程で実装する。
- symbol/relocation/section形式を再利用しても、EnbuのbankやMMIOをcoreへ追加しない。現machine profileは既存Ond-Kagura用であり、Enbu対応済みとは扱わない。

## 6. 既存実装との差分と最初の実装単位

public入口 [pipeline/api.rs](../../../../crates/compiler/src/pipeline/api.rs) はcoreで生成・検証したMIRをKagura backendへ渡す。旧AST backendと独立した型/literal解釈は撤去済み。byte array文字列literalも標準経路で扱う。

| 段階 | 成果物 / 合格条件 |
| --- | --- |
| 1 | 独立DataLayout + ABI plan。size/align/offset、6/7引数、4/5戻り値、hidden pointer、mixed aggregateの固定期待値テスト |
| 2 | O0 scalar/CFG/edge copy/直接call/frame/object。整数return、短絡、再帰・stack引数をKagura実行で確認 |
| 3 | Place/aggregate/Indirect/global/init/intrinsic。snapshot、alias、評価順trace、byte配列literal、startup順を実行確認 |
| 4 | 整数div/rem、soft-float/cast、fault/容量/遠距離。coreの数値vectorsを新backend実行へ接続 |

backendテストと標準CLI/machineテストはいずれもMIR経由で実行する。
MIR dumpの一致・簡易VM・独立oracleの成功をKagura実行の代用にしない。
coreの意味/型/表現不足だけを再現テスト付きでcoreへ戻す。target layout/ABI/命令数制限はbackendの責務。

## 7. 決定済みの境界と残る決定事項

以下はユーザー合意を反映した設計方針。実装済みを意味しない。

### 7.1 ゼロサイズ型

型layoutは空structがsize=0/align=1、[0]Tがsize=0/align(T)で確定し、machine仕様へ反映した。
address取得用slot・Indirect引数temporary等のゼロサイズstorageは最低1 byteを予約し、nilと区別する方針で確定した。型のsizeofは0のままとし、異なるゼロサイズ値のアドレス同一性に新しい言語保証は追加しない。frame内の予約はalignment等により1 byteより大きくてもよい。
new(T)はsize=0でも最低payloadを確保し、成功時は非nil、失敗時はnilを返す。alloc(0)はnilを返す。この契約は§10の実行テストで確認している。

### 7.2 Ordered aggregate

宣言/index昇順で再帰的にscalar accessし、paddingへ触れず、source snapshotを完成後にdestinationへ書く方針を確定した。
全体のatomicityは保証しない。途中faultで完了済みの書き込みやMMIOの副作用を巻き戻さない。
結果が同じという理由でもOrdered accessの順序変更は許可しない。machine仕様にも同じ契約を記載する。

### 7.3 遠距離call/分岐

**関数内はbackendの分岐拡張、関数間はリンカの中継コード（thunk/trampoline）**で対応する方針を確定した。

- backendは同一関数内のblock配置と距離を確認し、届かない分岐をaddress構築＋間接jumpへ拡張する。条件分岐では長距離経路を選んだ場合だけそのjumpを実行する。
- リンカは関数間の最終距離を確認し、直接call/jumpで届かない場合、呼出元から到達できる位置へ中継コードを配置する。
- 中継コードは最終entry addressを一時registerへ構築して間接jumpする。全32-bit addressを指定できるので、通常は1回の中継で足り、短距離jumpを何段も連鎖させる設計にはしない。
- callの中継は元のcallが設定したr15を保持し、再callしない。calleeは元の呼出元へ直接returnする。引数register・stack引数・SPも維持する。
- scratch registerのclobberをbackend/linker共通の契約にする。caller-saveであっても関数内jumpではliveな値を壊してよいわけではない。具体的なregister予約・relocation表現は§11で固定する。
- 同じ目的地へ向かう呼出元は、全てが到達できる場合に中継コードを共有できる。共有を正しさの条件にはしない。
- 中継コード挿入によって他の距離も変わる方式では、配置・距離判定を再計算し、全relocationが有効になるまで調整する。初期実装は§11の固定長予約方式で配置変化を避ける。未解決・容量超過・配置不能は明示エラーとする。
- 中継コードは通常実行がfallthroughする位置へ無防備に挿入しない。全てのentryを4-byte alignedに保ち、関数内分岐の距離保証を壊さない配置単位・挿入点を設計する。

アドレス構築の命令列生成はKagura用の低レベル部品としてbackendとリンカから共有する。
関数内の長距離分岐はbackendが、中継コードの命令列はリンカが生成し、最終addressはrelocation/layoutに基づきリンカが確定する。
既存ObjectのAbs32/Call16/Jump16に加え、literalを含む中継領域を予約したCallSlot/JumpSlotを導入する（§11）。

旧Call16/Jump16は中継領域もscratch契約も持たないため、引き続き距離超過をlinkで拒否する。MIR生成とstartupはCallSlotを使用し、無言の切り詰めやAST fallbackは行わない。
到達範囲の境界・正負両方向・複数siteの配置不変・引数/r15保持・条件分岐の非選択経路・容量超過をテストする。中継共有と可変長再配置は未導入。

新backend経路の対応済み範囲は、これらの契約を満たすテストとともに段階的に更新する。

## 8. MIR backendの実装状況

`ond-compiler::mir_backend` にASTを参照しないO0 backendを追加した。CLIでは `enbu build <project-dir> <cartridge.bin>` が唯一の標準経路である。`--mir`は廃止した。未対応機能で旧backendへfallbackしない。

公開APIは`mir_backend::codegen_objects(&mir::Project)`と`linker::link_objects`。入力MIRのvalidatorを実行し、Object/LinkedImageを生成する。起動処理の方針とROM化は機種SDKが所有する。helperはMIR経路で供給する。

- 対応型: 6整数型・f32・bool・pointer・function pointer、およびこれらを要素/fieldとするarray・structとdefined type。自然size/alignmentと物理slotの予約量を区別する。
- 対応命令: scalar定数・nil・関数address・aggregate zero、AddressOf、Placeのdereference/field/index/raw offset、型幅に応じたload/store、AggregateCopy、Bounds Check。加減乗算・bit演算・shift・比較・単項演算・整数/pointer cast、CFG分岐、loop、block parameterの並列copyも対応。
- aggregate: ValueIdは独立したsnapshot。copyはsource読み取りを完了してからdestinationへ書き、再帰的に宣言/index順のscalar accessのみを発行する。paddingを読み書きしない。文字列literalも通常のbyte配列として実行できる。
- 呼出し: 直接/間接call、Direct/Indirect引数、複数戻り値。先頭6 ABI wordはr1〜r6、残りはstack。aggregate引数はcaller-owned temporaryへコピーする。aggregateを含む戻り値または5個以上のDirect戻り値はhidden pointer付きreturn area、それ以外はr1〜r4。再帰対応。r7〜r11を変更せず、r15をframeへ保存する。
- frame: 8-byte alignment、outgoing引数領域、local/value snapshot、edge copy一時領域、引数copy、callごとのreturn area、return address。各用途を分離し、stack下限検査後にSPを更新する。frameとincoming引数offsetは32-bit範囲で検査し、大きなoffsetは§12のアドレス生成を使用する。
- intrinsic: load32/store32を単一LDW/STWへ変換する。alloc/free/newは§10のOnd製runtime helperへ接続する。
- global/startup: 全globalを自然alignmentでBSSへ配置し、文字列リテラルと配列・struct定数の静的実体を自然alignmentでRodataへ配置する。Static PlaceはAbs32 relocationで解決する。ゼロサイズglobalと静的実体にも物理identity用の最低1 byteを予約する。通常のglobal初期値は定数で書ける場合もDataへ先取りせず、MIR initializer内の処理として残す。machine loaderが全BSSをゼロ化した後、`Project.initialization_order`に従ってinitializerを一度ずつ呼び、最後にmain.mainを呼ぶ。Object順やimport名のソートから順序を再構成しない。mainから到達しないpackageのinitializerは呼ばない（object/storage自体の除去は未実装）。
- 検証: 生成したLinkedImageをテストbusへloadしてKagura CPUで実行し、結果・短絡非評価・再帰・stack引数・SP復元・callee-save保持・stack overflowを確認する。pointer alias、自然layout/array stride、bounds、ゼロサイズ値、aggregate引数の非alias、評価途中のsnapshot、mixed return area、間接callも検査する。bus traceではアクセス幅・順序・padding非アクセス・重複copy・source/destination途中faultを確認する。layout/ABIの固定期待値、layout overflow、分岐距離の正負境界、frame上限、未対応診断も検査する。

globalの検証では、後方参照時のzero、初期化中に書かれた後続globalの非再zero化、aggregate snapshot・pointer・function pointer、複数戻り値initializer、diamond importの一度だけの実行、ファイル/宣言順、未到達package、途中fault、global layout/容量/合計section overflowをKagura実行または固定期待値診断で確認する。global初期化の実行はLinkedImage直接loadとEnbuのROM起動で検証する。

整数除算/剰余とそのCheck、f32、allocatorの接続は§10に記載する。関数内の長距離分岐とリンカ中継コードは§11、frame/incoming引数のsigned-16-bit制限の撤去は§12で実装済み。現在の言語から生成されないChecked加減乗算などはwrapへ置き換えず未対応診断とする。

責務は `layout`（型配置/ABI計画）、`frame`（物理slot）、`memory`（Place解決/型幅access）、`calls`（引数/戻り値）、`function`（CFG/命令統合）、`globals`（BSS配置）、`ops`、`emit` に分割している。startup計画はcartridge builderでMIRから取得する。coreの変更は不要だった。

§6の段階3と段階4の数値runtime・長距離分岐・frame容量制限の緩和まで実装済み。全仕様ケースのbackend適合検証、Enbuのbanked ROM対応の完了を意味しない。

## 9. Runtime helper供給基盤

`mir_backend::runtime`はcompiler所有のOnd helperを、compiler-core → MIR backend → Objectの経路でビルドする。Ondの言語仕様やuser importにruntime専用機能を追加しない。

- `Helper`は登録済みentryとscalar ABIを定義し、`symbol()`で予約済みlinker symbol、`signature()`で受け渡しの型を提供する。soft-floatの15 entry、整数除算/剰余4 entry、allocator3 entryを登録済み。`objects(&[Helper])`で要求したentryのObjectを生成できる。
- backendがhelper symbolへのcall/address relocationを生成すると、`codegen_objects`末尾の供給処理が要求を収集する。entryのABIを検証し、direct callとFunction定数から依存関数をたどって一度だけemitする。要求がなければruntimeソースのコンパイルも追加Objectも発生しない。
- userとruntimeは別々のTypeTableでコンパイルし、Objectと明示ABIで接続する。内部関数も制御文字を含む予約namespaceへ改名し、user定義の衝突・未知の要求を診断する。
- compiler-coreが現状project単位の入口なので、メモリ上の空mainをfrontend用に添える。そのmainは選択前に破棄し、userのstartupへ追加しない。ロード済みソースの検査は物理ファイル存在に依存しない。
- 通常のOnd関数の再帰はvisited集合で有限の依存閉包として扱う。runtime生成は再供給しない専用入口を使い、未対応primitiveまたはloweringで生じた閉包外の依存を診断する。runtime helperにglobal storage/startup副作用を持たせることは現段階では拒否する。allocatorは状態のaddressを引数で受け取るため、この制限内で動作する。
- 既存`runtime_support.ond`を共有する。符号付き定数MinNormalExpは現言語仕様と旧backendの両方で扱える`0 - 126`へ修正した。compiler所有の`__sf_invalid`だけはMIRのTrap primitiveに置換し、変換エラーの停止を整数除算helperへ依存させない。
- テストは全22 entryのMIR/Object生成、依存選択・重複排除・Function定数・再帰、未知要求・衝突・副作用拒否を確認する。Object境界でのhelper単体テストに加え、§10の通常のOndソースからの実行も検証する。hostのfloat MMIO deviceは使わない。

## 10. 数値命令・allocatorの接続

- `numeric`モジュールで整数除算/剰余、float演算/比較/negate、整数↔f32 castをhelperへ選択する。f32はstorage・引数・戻り値では32-bit patternをそのまま運ぶ。signed狭整数はhelper前にsign-extendし、結果を型幅へ正規化する。暗黙のhelper callも既存frameのr15保存・Value spillを使用する。helper引数は最大4 wordなので追加のoutgoing stack areaは不要。
- 除算はinteger.ondの32段のunsigned long divisionを基底とし、最上位carryを落とさず比較する。signed処理は絶対値のunsigned bit patternと符号の復元で実装する。NonZero/SignedDivisionOverflowはMIRの位置でfaultを生成する。signed min/-1の除算はfault、剰余は0。helper自身は/や%を使わない。
- f32 castは0方向に丸めた結果の範囲を検査する。unsignedへの-0.5は0、signed狭型では切り詰め前のfractionではなく整数結果を範囲検査する。旧soft-floatの負の小数→unsignedと大きな指数→i32の境界処理も修正した。NaN/Infや範囲外はTrap primitiveで停止する。
- heap.ondのalloc/free/newへ、linkerの`__ond_heap_base`、`__ond_stack_bottom`、startup BSSの`__ond_heap_cursor`のaddressを渡す。cursor=0なら初回allocでheap_baseへ設定する。package初期化中のnewも同じ状態を使用する。
- first-fit、余りがheader+最低payload以上のときの分割、free時の前後結合、末尾回収を実装する。size計算overflow・容量不足ではnilを返す。newは成功時だけ型size分をzero化し、size=0でも最低payloadを確保する。alloc(0)=nil、free(nil)=no-op。有効なallocation pointerをfreeする契約であり、不正freeやheap header破壊に対する安全性は保証しない。
- 検証は通常のOndソース→MIR→Object→link→Kagura CPUで行う。float四則演算はsubnormal・NaN・Inf・signed zero・丸め境界と固定乱数のbinary32期待値で比較する。float全比較、cast成功/失敗、aggregate/global storage、6整数型のdiv/rem、全符号組合せ、ゼロ除算、min/-1を確認する。
- heapテストはglobal initializer、再利用時のnew zero化、ゼロサイズ、分割・前後結合・末尾回収、overflow/確保失敗、heapがSTACK_BOTTOMちょうどまで満杯のときのnilと解放後の復帰、stack側sentinel非破壊を確認する。Enbu CLIのbuildでも数値演算とnew/freeを組み合わせる。

これらは対応命令の接続と回帰検証であり、binary32全bit組合せや全言語仕様ケースの網羅を主張するものではない。長距離call/branchは§11、frame容量制限の緩和は§12を参照。

## 11. 長距離分岐とObjectの中継slot

初期実装は**固定長の専用中継領域を各siteに予約する**。予約分を含めてblock label、symbol size、relocation offset、RAM/heap配置を決定し、link時には命令を書き換えるだけで挿入・削除しない。これにより、他の分岐距離やAbs32を再配置する必要がない。近距離も予約領域を保持するためコードサイズが増える。中継共有・不要領域の圧縮・可変長relaxationは後続の最適化とする。

### 11.1 関数内branch

- `Emitter.branch`は7 word（28 byte）を予約する。word 0が条件付き/無条件JZ、word 1がword 7へのskip、word 2〜6が中継領域。
- `(target - (site + 4))/4`がsigned 16-bitに収まる場合、word 0を直接patchする。それ以外ではword 0をword 2へ向ける。
- 中継はPCをr12へ取得し、literalからr13へ相対byte差分を読み、r12+r13へ間接jumpする。literalはword 3のaddressからtargetへのmodulo-2^32差分。text全体の32-bitサイズ検査を行った上で生成し、Object配置に依存しない。
- taken経路のr12/r13のみclobber可能とする。O0ではlive MIR値・edge copyをstackへ保存済みであり、内部数値checkもこの契約に従う。非選択経路はword 1で中継命令とliteralをskipする。
- prologueのfault skip等、命令列内で距離が固定された短いJZは従来どおり直接encodeする。

### 11.2 CallSlot / JumpSlot relocation

新relocationはText内の4-byte alignedな**6 word（24 byte）**を所有する。offsetはword 0を指し、入力Objectのdata/memory_size内に全slotを含める。

| word | link前 | 遠距離patch後 |
| --- | --- | --- |
| 0 | `JZ r15,r0,r0,0`（CallSlot）または`JZ r0,condition,r0,0`（JumpSlot） | 同じ条件/link registerでword 2へjump |
| 1 | `JZ r0,r0,r0,4` | 変更なし。word 6へskip |
| 2 | 0 | `JZ r12,r0,r0,0`（word 3のPC取得） |
| 3 | 0 | `LDW r12,r12,r0,8`（word 5のliteral読出し） |
| 4 | 0 | `JZ r0,r0,r12,0`（最終targetへjump） |
| 5 | 0 | `symbol address + addend`の32-bit literal |

近距離ならword 0のsigned-16-bit immediateだけをpatchする。call時のr15はword 1を指し、calleeのreturn後にword 6へskipする。thunkは再callせず、引数r1〜r6、stack引数、SP、callee-save、r15を保持し、r12だけを変更する。JumpSlotの非選択経路もword 1から継続し、registerを変更しない。

linkerはslotの入力範囲・template・alignment、解決先symbol、addend後の32-bit範囲とalignment、最終image容量を検査する。他のsymbol/relocationを予約領域内へ置いたり、内部へ通常分岐してはならない。旧Call16/Jump16は自動拡張しない。これらはObject/linkerの低レベル契約として残すが、撤去済みの旧AST backendへの入口は提供しない。

低レベルencode・PC取得/literal読出し・中継命令列を`kagura_encoding`へ集約し、MIR emitterと`linker/thunks`で共有する。startupのmain/initializer callにもCallSlotを使う。coreと言語仕様の変更は不要。

### 11.3 検証

- signed word距離32767/32768/-32768/-32769で、近距離/中継選択と実CPUの到達先を検証する。
- 関数内相対中継を複数load addressで実行し、条件不成立時のscratch保持も確認する。
- linker call/jumpでregister・r15保持、return後のliteral skip、addend、複数siteとAbs32の配置不変を確認する。
- Ondソースから128 KiB超の関数を生成し、長距離ループ・条件分岐の両経路、前方startup/main/initializer call、後方7引数callと関数pointerをKaguraで実行する。SP/callee-saveも確認する。
- 不正template・範囲/alignment・未解決symbol・addend overflow・RAM不足・旧relocationの距離超過は診断する。

## 12. 大きなstack frame

MIR backendのframe/incoming引数offsetに対するsigned-16-bit制限を撤去した。小さなoffsetでは従来の即値命令を維持し、32767を超えるoffsetでは32-bit即値とSPの加算で実アドレスを生成する。local、spill、edge snapshot、argument copy、return area、保存r15、incoming/outgoing引数に共通の処理を使用する。

- stack load/storeの大offset展開はr13をscratchとして使用する。データregisterがr13の場合だけr12を使う。r12に保持したstack引数をstoreする際も値を壊さない。これらのscratchにアクセスを越えるlive値を置かない。
- stack address生成は出力registerだけを変更する。SP自体は変更せず、r1〜r6への引数address構築時も既に設定した引数を維持する。
- prologueは従来の32-bit frameサイズ生成、減算underflow、STACK_BOTTOM検査を維持し、成功前にSP更新やframeへの書き込みをしない。epilogueも大きなサイズをregisterへ生成してSPを復元する。戻り値r1〜r4と復元したr15を維持する。
- slot予約・alignment・frameとincoming領域の合計は32-bit checked演算のままとし、overflowは診断する。設定されたstack容量を自動拡張するものではなく、実行時のstack不足は引き続きfaultになる。
- 検証は32764/32768/65536/131072のload/store/address、r12/r13/引数/r15保持、40 KiB超frameの7引数・間接call・複数scalar戻り値・aggregate引数/戻り値/snapshot、通常ソースの36 KiBローカル配列、stack不足時のSP/メモリ非破壊、frame計算overflow。実行はKagura CPUを使用する。

aggregateの要素ごとの命令展開やO0のslot消費量は変えていない。大きな配列でコード量が増える点、実メモリ容量、32-bitアドレス範囲は別の制約として残る。core・言語仕様・Object形式の変更は不要。
## 13. 標準経路と責務の分離

`compile_project` / `compile_loaded_project` / overrides APIはMIR codegenでObjectを生成する。SDKがLinkPlan・初期化順序・終了処理を指定してlinkし、ROMへ配置する。package初期化順序はMIRのinitialization_orderだけを用いる。

旧AST codegen、旧startup/runtime供給、単独`.kg`→assembly compiler、未使用host float deviceを撤去した。Ond製soft-float runtimeは引き続きMIRでコンパイルする。LSPはcompiler-coreへ直接依存し、targetのcodegenやframe制限を診断に混ぜない。
