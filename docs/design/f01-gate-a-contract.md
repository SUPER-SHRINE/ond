# F01 ゲートAの最小device・machine契約と適合試験案

状態: 検討用の非規範設計案。作成日: 2026年10月5日。

本書は[0.2.0のゲートA](machine-roadmap.md#020の範囲と終了条件)に向け、最小graphics、window、load、起動・終了、仮想時間、snapshotの動作と試験を提案する。既存の言語仕様、ABI、MIR、Object／LinkedImage、公開APIを変更せず、新しい規範仕様を採用済みとは扱わない。文中の「要求する」「拒否する」は、採用する場合の契約案を表す。

この設計PRの成果は、契約の採用候補、要件ID、試験の入力と期待結果、第二実装へ渡す資料の充足条件である。実装、試験の実行、第二実装の完成、ゲートAの通過は本書の追加だけでは成立しない。

## 根拠と担当する境界

| 根拠 | 既存の規定または計画 | 本書への反映 |
| --- | --- | --- |
| [Ond target runtime契約](../../specs/ond/targets/kagura-v1/machine.md) §1〜4、§6 | 32-bit little-endianのflat bus、機種SDKによるmap・load・SP・main後の処理、型幅と同じscalar access、aggregateは全体でatomicではない | CPUとOnd ABIを変えず、machine側を具体化する |
| [Ond ABI](../../specs/ond/targets/kagura-v1/abi.md) §1〜2 | SPは`r14`、call境界で8-byte alignment | 起動時のSPとguest startupを区別する |
| [全体設計](machine-experience.md#仕様書を正本とする原則) | 実装ではなく本文が正本、要件IDと試験を対応付ける | 正常・境界・fault・resetの期待結果を本文から作る |
| [全体設計の時間と観測](machine-experience.md#実行時間と観測の契約) | ISAは共通cycle数を定めない、命令確定境界でdeviceを進める初期案、副作用のない同一epoch snapshot | 命令数に基づく最小時間案と停止時snapshotを提案する |
| [制作フロー](machine-workflow.md) §2〜5 | 固定画像、複数windowと同一instance、host binaryなしのSDK・アプリbuild | 直接VRAMを書き、論理frameを確定する最小構成に絞る |
| [組立境界](device-assembly.md) | 動作・machine・host接続の三契約、fault時の無副作用、命令fetchの副作用問題 | 相対offsetと絶対mapを分け、MMIO fetchを呼出し前に拒否する |
| [device製品](device-package.md) | 規範依存本文の閉包、仕様と実装の別識別、形式成立と適合の別表示 | 第二実装へ仕様本文と試験資料を渡す |

組立境界が参照する[Kagura CPU](../../../kagura/specs/kagura/v1/cpu.md) §8〜9と[Kagura Bus](../../../kagura/specs/kagura/v1/bus.md)は、一回のfetch/accessとfault時の無副作用を説明する調査参照である。Ondが固定するKagura `0.1.0`と隣接checkoutの本文が同じかは、このリンクだけでは証明できない。採用前に固定版の規範本文を照合し、正確なrevisionとdigestを依存記録へ残す。依存側の変更が必要なら`blocked`とし、Ondの作業から隣接repositoryを変更しない。

製品manifest、archive、digestのserialize規則、アプリwrapperの保存形式はF19/F07、native ABI、bufferの所有権とbinaryのload方式はF20で決める。本書は、それらが保持・配送する論理値と動作を定める案である。hostの関数名、Rust型、OS window handleは動作契約に含めない。

## 最小構成の提案

CPU・bus・RAM・loaderはmachine基盤とし、graphicsは一種類を一個以上載せられる構成を提案する。各graphics instanceは制御windowとVRAM windowを持つ。固定画像に必要な機能は、VRAMのread/writeと、その時点の画素をframeとして確定する一操作だけとする。描画command、blend、palette、DMA、IRQ、入力、audio、周期更新はこの最小profileに含めない。

`Clear`、`SetPixel`、`Present`は既存文書の仮API名であり、採用済みのdevice操作名ではない。領域を埋める処理や座標からoffsetへの変換はguest SDKで実装できる。128×128、64 KiB、例示された絶対addressも既定にしない。

以下の要件はすべて未採用候補である。`F01-`で始まるIDは本案の追跡用で、Ond/Kagura既存規範のIDを改番しない。採用前の変更でも、同じIDの意味を変えた箇所は判断記録と試験へ反映する。

### Graphicsの状態と操作

| 要件ID | 契約案 |
| --- | --- |
| F01-GR-01 | 設定は正整数`width`、`height`を明示し、既定値を持たせない。VRAM長は`4 × width × height` byte。対応上限と検査する整数範囲は判断D01で固定し、乗算・address計算をwrapさせない。欠落、0、範囲外、計算overflowはinstance生成前に拒否する |
| F01-GR-02 | 一画素は4 byteの`R, G, B, A`で各成分は0〜255。座標原点は左上、右へx、下へyが増え、offsetは`4 × (y × width + x)`。行paddingはない。論理frameはVRAMと同じbyte列で、色変換、alpha合成、拡大や補間を行わない。`A`も値のまま保存・比較する。host表示の変換結果を適合判定に用いない |
| F01-GR-03 | 制御windowは4 byte。offset 0への整列した32-bit writeで値`1`だけを受理し、VRAM全体のコピーを一つのframeとして確定する。read、他の値、8/16-bit writeはfault。VRAMのread/writeは8/16/32-bitを受理し、little-endianと各幅のalignmentに従う。未定義の操作を成功扱いにしない |
| F01-GR-04 | reset後はVRAM全byteが0、確定frameは存在せず、frame番号は0。確定成功ごとに番号を1増やし、最初のframeは1。VRAMの後の書換えは確定済みframeを変更しない。同じVRAMを続けて確定しても別番号のframeを二つ生成する |
| F01-GR-05 | 確定frameはinstance ID、session、frame番号、仮想時刻、width、height、形式識別、画素byte列を持つ。論理frameの欠落・重複・並替えは許容しない。headlessと表示ありでこの列とguest状態は一致する。物理表示の遅さでVRAMや確定順を変更しない |

RGBA8と制御値`1`は本案が比較可能性を確かめるための採用候補である。製品名CommonGraphicの正式仕様や、既存Enbuへの適合を意味しない。

### Windowとmachine map

| 要件ID | 契約案 |
| --- | --- |
| F01-WIN-01 | deviceはwindow ID、byte長、許可read/write幅、alignmentを相対offsetで宣言する。machineは`(instance ID, window ID)`ごとに絶対baseを一つ割り当てる。制御とVRAMは同じinstanceの状態へ配送し、二個目のinstanceは独立した状態を持つ |
| F01-WIN-02 | access全体が一つのwindowに収まり、幅とalignmentが許可される場合だけdeviceへ一回配送する。検査は`offset ≤ length - width`等のoverflowしない方法で行い、windowを跨ぐaccessは隣のwindowへ分割しない。不正accessをdeviceへ呼び出さない |
| F01-WIN-03 | deviceがfaultを返す操作は、VRAM、制御状態、frame番号、出力列を変更しない。成功した他の命令の効果は保持する。Ond aggregateを構成する複数scalar access全体のrollbackは要求しない |
| F01-MAP-01 | mapは32-bit address空間内の半開区間`[base, base + length)`として解釈する。終端の数学上の値`2^32`は表現可能な検査用の値とし、bus addressへwrapさせない。RAM、ROM、MMIO、終了windowの重複、0長、overflow、必須window欠落、未知window、必要alignment違反をguest実行前に拒否する。初期profileにalias/bankは設けない |
| F01-MAP-02 | 命令fetch可能な範囲をmachineに明示する。fetchの全4 byteが副作用のないRAM/ROMの許可範囲に収まる場合だけ読む。MMIOや未許可領域のfetchはdevice readを呼ぶ前に`BUS_FAULT`として停止する。unaligned PCはCPU規範の`UNALIGNED_PC`を保持する。通常のdata accessからこの制限を推測しない |

RAMの`.text`・`.rodata`へのdata writeを新たに禁止する案ではない。現行Ond target契約のRAM write protectionを行わない意味は維持し、fetch許可範囲とは別に扱う。

### Load、起動、終了、reset

| 要件ID | 契約案 |
| --- | --- |
| F01-LOAD-01 | アプリが要求するISA/ABI、device仕様ID・revision・digest・設定、map、時間、起動・終了、load profileと、runtimeが対応する論理machine契約をguest実行前に照合する。初期は同一の論理契約を要求し、暗黙の互換拡張は行わない。host実装IDやbinary hashの変更だけでは拒否しない |
| F01-LOAD-02 | sectionの配置address、payload長、zero化領域、alignment、entry、stack予約を入力としてすべて検査してからloadする。書込先は許可されたRAMに限定し、MMIOへimageをloadしない。section同士の重複、範囲overflow、RAM外、stack侵食、entryのunalignment/実行範囲外を拒否する。失敗時にguest命令やdevice accessを実行せず、成功前の部分imageを実行可能にしない。具体的な保存fieldはwrapper側で固定する |
| F01-BOOT-01 | 起動順は、構成とアプリの事前検査、全instanceと必須接続の準備、session開始、CPU/RAM/deviceのreset、machine資源の配置、section payloadのloadと指定zero化、SP/entry設定、guest実行とする。CPU resetは既存規範に従う。RAMはresetで全byteを0にする候補とし、ROM等の固定資源はその後に配置する。失敗時はguestを始めない |
| F01-BOOT-02 | `r14 = STACK_TOP`、`PC = app entry`とし、STACK_TOPを8-byte alignedに検査する。その他のregisterはCPU reset値を保つ。アプリのentryはpackage initializerを規定順に呼んでmainへ進むlink済みguest startupであり、loaderがOndのmainを直接呼ぶ契約ではない。heap/stack配置は現行Ond target契約を保持する |
| F01-END-01 | machineが専用の終了windowを明示する案とする。長さ4 byte、offset 0の整列32-bit writeだけを受理し、任意の`u32`値を終了codeとして記録する。read/他幅はfault。guest startupがmainの正常return後にcode `0`を書き、書込命令の成功境界で正常終了する。次の命令は実行しない。終了codeはCPU faultとは別の結果である |
| F01-END-02 | 正常終了、CPU fault、利用者の停止は異なる停止理由にする。停止後はCPU/device/仮想時間を進めず、最後に確定した論理frameとsnapshotをresetまたは破棄まで保持する。最後のVRAM書込みだけでframeを自動確定しない。物理windowをいつ閉じるかはhostの操作方針として別に決める |
| F01-FAULT-01 | CPU faultはCPU規範の種類・faulting PCを保ち、bus faultならaddress・幅・fetch/load/storeも記録する。該当instance/windowと相対offsetを配送診断へ付ける。fault命令はregister、PC、device、frameに副作用を残さない。host出力障害や接続違反をguestの不正accessと混同しない |
| F01-RESET-01 | resetは実行中の命令境界で更新を止め、新sessionへ移り、CPU/RAM/device・仮想時間・epoch・frame番号・終了理由を初期化する。古いsessionの入力・出力は以後のguest状態へ適用しない。最小profileに永続状態はない。再実行では同じ検査とload手順を行う。reset前のsnapshotは履歴として残せても現在sessionとして表示しない |

終了windowの導入とguest startupからの終了操作は未採用の起動profile案である。現行backendが固定終了deviceを要求する仕様へ変更しない。既存のcompile/link経路やEnbu SDKにこの動作を暗黙に適用しない。

### 仮想時間と順序

`F01-TIME-01`の候補は、秒やCPU cycleではなく成功命令数を単位とする`u64`の仮想時刻`T`である。起動時は`T = 0`、命令一個の成功で`T + 1`、fault命令は進めない。命令種別ごとの比率は設けない。採用判断はD04に残す。

| 要件ID | 契約案 |
| --- | --- |
| F01-TIME-01 | host経過時間、描画頻度、snapshot回数からguest時間を作らない。pause・停止中とload/resetの処理では時間を進めない。`T = 2^64 - 1`では次の命令を始める前にmachineの時間上限として停止し、wrapやguest CPU faultへの置換をしない |
| F01-TIME-02 | 各成功境界の順は下記1〜5とする。複数instanceを進める順はmachineが保存するinstance順。表示名・ボード座標で順を変えない。初期graphicsに周期eventはないが、時間配送の順を試験traceへ記録する |
| F01-TIME-03 | 一つのinstanceのframeは確定操作の順、複数instanceのframeは成功境界とinstance順で並べる。guest状態を変更する処理をruntimeが直列化し、snapshotやhost非同期処理が途中に割り込まない |

1. 現在の`T`で停止要求を取り込み、続行するなら時刻上限とfetch範囲を検査する。
2. CPU命令を一個実行する。その命令中のdevice data accessへは、成功時の候補時刻`T + 1`を与える。graphicsの確定操作は、このaccess時点のVRAMをコピーする。
3. CPUが成功したらregister/PCの確定後、machineの`T`とepochを1増やす。faultなら停止し、時間・epochを保持する。
4. 保存されたinstance順にdeviceを新しい`T`まで進める。周期eventを持たない最小graphicsのVRAMは変わらない。
5. 成功操作のframeを時刻`T`で公開し、その後にsnapshot可能な境界へ戻る。終了writeを行った命令もこの境界まで完了して停止する。

device単体試験では明示timestamp付き操作を直列に渡し、受理成功した操作のframeに同じtimestampを付ける。machine試験ではこの命令境界規則を通す。候補時刻を渡しただけでfault時に時間を進めないことも検査する。deviceの呼出し順だけでCPU命令全体の無副作用を保証できるとは限らず、特にfetch制限の成立を固定Kagura版で検証する。

### Snapshot

| 要件ID | 契約案 |
| --- | --- |
| F01-SNAP-01 | 最初のsnapshotはpauseまたは停止済みの命令境界から取得する。CPU register/PC、停止理由、指定RAM範囲、graphics VRAM、最後の確定frameを同じsession・epoch・仮想時刻に結び付ける。実行中の要求は境界でpauseして取得するか、明示的に拒否し、途中の状態を混ぜない |
| F01-SNAP-02 | snapshotは通常MMIO readを呼ばず、VRAM、frame、時刻、epoch、停止理由を変えない。連続取得・サイズ確認・不正な範囲/容量による失敗も消費操作にしない。失敗した取得を完全なsnapshotとして返さない |
| F01-SNAP-03 | schemaは論理値の型、byte長、address/offset、画素形式、instance IDを持つ。sessionは実行とresetを区別できる識別子、epochはそのsession内の成功命令境界数で初期0とする。異なるruntime間でsession文字列そのものの一致は要求せず、新旧を区別できることを検査する。schemaのserializeとbuffer寿命は別契約で固定する |

snapshotは保存状態への復元機能を意味しない。live購読、履歴replay、memory編集は初期の適合条件に含めない。

## 共通適合試験案

試験は本文の要件を正本とし、adapterが期待値を参照実装から採取して埋めない。各caseは要件ID、分類、明示した構成・初期状態、操作列と時刻、成功/fault/停止理由、状態byte列、frame列、必要な配送traceを持つ。実装固有のpointer、内部buffer配置、OSの画面captureを比較しない。

下記の2×1/1×1等は小さなtest vectorであり製品既定ではない。D01採用時にこれらを許可範囲へ含めるか、同じ境界を満たすvectorへ置換する。`L`は採用後のwindow長、`w`は検査するaccess幅、`B`は明示されたbaseを表す。

| Case ID / 分類 | 要件 | 入力と受入条件 |
| --- | --- | --- |
| A-GR-01 正常 | GR-02/03/05 | 2×1のVRAMへ`[0x12,0x34,0x56,0x78,0x9a,0xbc,0xde,0xf0]`を8-bitで書き、read32 offset 0が`0x78563412`。制御write32=1で、同じ8 byte、2×1、frame番号1、指定時刻のframeを一つ得る |
| A-GR-02 正常・順序 | GR-04/05、TIME-03 | A-GR-01の後、offset 0を`0xff`に変更。snapshotのVRAM先頭は`0xff`、前frame先頭は`0x12`。二回目の確定だけが番号2で`0xff`を含む。同じ内容を再確定すると番号3でframe列長3になる |
| A-GR-03 境界 | GR-01、WIN-02 | 1×1と許容上限の構成を検査。各許可幅で最後に収まる整列offsetと`L-w`へのaccessが成功する条件を確認し、そこから1 byte外れたaccessを拒否する。整列違反と範囲違反が重なる入力は成功として扱わない |
| A-GR-04 fault | GR-01 | width/heightの欠落、0、上限+1、積やbyte長のoverflowを一つずつ与え、生成前に原因と設定項目を診断。instance作成・guest実行は0回 |
| A-GR-05 fault | GR-03、WIN-02/03 | 制御read、8/16-bit write、write32=0/2/最大u32、VRAM unaligned access、不許可幅、window跨ぎを個別実行。fault前後で全論理状態とframe列が同一。配送前の違反はdevice呼出し0回 |
| A-WIN-01 正常・分離 | WIN-01、GR-04 | 同じ実装から二instanceを作る。一方のVRAM更新と制御操作でそのinstanceのframeだけが変わり、他方のVRAM/番号/出力は変わらない。どちらも制御とVRAMが同じinstanceを参照する |
| A-MAP-01 境界・fault | MAP-01 | 隣接する非重複領域は受理。1 byte重複、必須window欠落/重複指定、未知window、alignment違反、0長、終端>`2^32`を拒否。終端=`2^32`の整列済み合法範囲はoverflowと誤判定しない |
| A-WIN-02 fault | WIN-02/03 | 隣接windowの境界を跨ぐread/writeで、両側のstateと呼出しtraceが変わらない。32-bit accessを8-bit四回へ分解して受理するadapterは不合格 |
| A-LOAD-01 正常 | LOAD-01/02、BOOT-01/02 | 明示したRAM・stackと二種類の合法imageで、payload byteが一致、指定zero化領域は全0、entry/SPは各入力どおり。同じruntime構成を二imageで再利用し、第一命令前のsnapshotを比較する |
| A-LOAD-02 fault | LOAD-01/02 | 論理契約の不一致と、section重複/overflow/RAM外/MMIO先/stack侵食、不正entryを一つずつ与え、guest開始0回・device access0回。古い成功imageへ暗黙にfallbackしない |
| A-BOOT-01 reset | BOOT-01/02、RESET-01 | RAM/VRAM/register/frame/停止理由を変えた後reset・同一image再load。新session、T=epoch=0、device初期値、load後RAM、entry/SPが初回起動と一致。古いframeの遅延配送を新sessionの出力として受理しない |
| A-END-01 正常・境界 | END-01/02、TIME-02 | frame確定後、終了windowへ0と最大u32を別runで書く。対応codeで停止し、その命令の成功分だけT/epochが増える。後続のRAM sentinel書換えは起きず、最後のframeを保持する |
| A-END-02 fault | END-01、FAULT-01 | 終了windowのread/不許可幅/unalignmentを個別実行。正常終了codeを新規記録せずCPU faultで止まり、後続命令なし。RAM/graphicsはfault前と同一 |
| A-FAULT-01 fault | FAULT-01、WIN-03 | 成功したstoreの後にdevice拒否accessまたは不正命令。成功storeの効果を保持し、fault命令のregister/PC/device状態は開始時と同一。fault種類とPC、bus faultのaddress/幅/operationを照合する |
| A-FETCH-01 fault | MAP-02、FAULT-01 | 許可範囲端の全4 byteが収まるfetchと跨ぐfetch、未map先、graphics MMIO先、unaligned PCを個別試験。readでsentinelを消費する試験用MMIO先へのfetchでもread呼出し0回・sentinel保持。BUS_FAULTとUNALIGNED_PCを区別する |
| A-TIME-01 正常・順序 | TIME-01/02/03 | 算術、VRAM store、frame確定、終了を含む短い成功命令列でT/epochとtraceを境界ごとに照合。store→確定は更新後byte、確定→storeは更新前byteのframeとなり、frame時刻はその確定命令の成功境界。二instanceの進行traceは保存順 |
| A-TIME-02 fault・境界 | TIME-01/02 | fault前後、pause中、snapshot前後、host待機前後でT/epoch不変。時刻を指定できる共通の試験adapterでT=`2^64-2`から成功一命令を通し、次は命令開始前に時間上限停止。wrap、device access、CPU faultへの置換なし |
| A-SNAP-01 正常 | SNAP-01/02/03 | 更新後にpauseし、CPU/RAM/二graphicsを同一session・epoch・Tで取得。二回取得して論理値が同一、通常MMIO read traceなし。未確定frameは「なし」として扱い、zero画素のframeと混同しない |
| A-SNAP-02 fault・reset | SNAP-02/03、RESET-01 | 範囲外/overflow範囲、不足容量、実行途中の取得要求を試験。明示拒否または境界pauseだけを許容し、部分値を完全snapshotにしない。失敗前後でstateと時刻不変。reset後の取得は新sessionで、旧値を現在として返さない |
| A-OBS-01 正常 | GR-05、SNAP-02、TIME-01 | 同じimageを、snapshotなし/各境界snapshotあり、headless/表示ありで実行。全guest状態、frame byte列・番号・仮想時刻、停止理由が一致。hostの実時間や画面更新回数は比較対象にしない |

表の要件欄では共通接頭辞`F01-`を省略した。resetのdevice初期値はA-BOOT-01に加え、device単体でもGR-04の全byte、frameなし、番号0を確認する。試験用read-clear MMIOはfetch防御とsnapshot無副作用の検査道具であり、0.2.0製品へ別deviceを追加する要件ではない。

共通adapterは、論理instanceの生成/reset、明示時刻付きaccess、frame列の取得、停止境界snapshot、machineのstepとtraceを試験に提供する。これは試験上の操作で、native ABIの確定案ではない。時刻上限を注入するfixtureは検証専用であり、通常の実行やアプリ成果物へ時刻書換えを許可しない。adapterだけのmock試験と、固定CPUを通すmachine試験の結果を別に記録する。

## 判断待ちと規範化の停止条件

次の案を採用するか変更するかは設計判断として残す。値を省略したまま実装の都合で埋めたり、試作の結果を自動で正式仕様としたりしない。

| 判断ID | 採用候補と必要な決定 | 決まるまで止めるもの |
| --- | --- | --- |
| D01 画面構成 | width/heightの明示、無既定、RGBA8。対応寸法・総byte数の上限、設定の整数範囲、形式識別を固定する。128×128を既定にする根拠は現計画にない | 設定schemaと上限vectorの確定、graphics適合判定 |
| D02 window | 本書の制御4 byte、write32=1、read fault、VRAM三幅の採否。window ID、size/alignment制約、未対応幅の診断を本文と宣言schemaで一致させる | guest bindingとdevice規範本文の確定 |
| D03 load・起動・終了 | RAM全zero、直接entry/SP、終了window案、停止後の最終論理frame保持の採否。RAM/stack/map値、資源配置順、許可section規則、load入力の解釈をwrapperの契約と対応付ける | boot profile、image fixtureと正常終了試験の確定 |
| D04 時間 | 成功一命令=一単位、faultで不変、u64上限、候補時刻と境界順、instance順の採否。別比率を選ぶ場合は時刻vectorも変更する | 時刻に依存する試験と第二実装のmachine適合判定 |
| D05 観測・出力 | session/epochの識別、snapshot schema、frame形式の識別、出力の保持量と回収規則。容量不足を隠して論理frameを落とさない。host接続側のbuffer規則と対応を取る | snapshot/frame交換と資源制限試験の確定 |
| D06 規範依存 | 固定Kagura版のCPU/bus本文、Ond ABI、load形式等の正確なrevision・digestと優先関係。fetch拒否を既存CPUの外側で無副作用のまま成立させられるか検証する | 依存閉包の確定、CPU統合の通過判断。Kagura変更が必要ならblocked |

resource上限、frame回収が間に合わない場合、device時間進行失敗、host出力障害からの停止はhost接続契約との接点である。少なくともguest faultと別診断にし、停止境界と保存済み論理出力の扱いを双方に同じ意味で記述してから統合試験を確定する。設定上限やqueue容量を未指定のまま、無限の確保を要求する仕様へ進めない。

## 第二実装へ渡す資料とゲートAの判定

第二実装には、採用後の動作本文・宣言schema・machine profile・観測schema、固定した規範依存本文、共通試験仕様とvector、要件対応表を渡す。元device実装のsource、そこから作った内部設計図、元実装が生成した期待値を前提にしない。同じcoreの別buildや、元実装をwrapしたものを第二実装として数えない。試験adapterのtransport部分を共用しても、deviceの状態遷移を共用しない。

引渡し前に次をすべて確認する。一つでも未充足なら「第二実装へ渡せる完全な規範契約」と表示しない。

1. D01〜D06の採否と具体値を判断記録に残し、本文/schema/vectorに未決値・仮既定・矛盾がない。規範本文と参考資料の優先関係を明示する。
2. device状態、全windowの許可操作、初期/reset値、予約領域、fault、frameの意味と時刻を本文だけで決められる。
3. machineの全map、fetch領域、load入力、entry/SP、起動と終了、時間単位・上限・順序、観測の境界を固定し、個別アプリに依存する入力とmachine資源を分ける。
4. 本文・schema・固定データ・依存本文がオフラインで揃い、revision/digest、要件IDとcase IDの対応、試験の比較規則を保存する。
5. 全caseの期待値を本文から説明でき、受理と拒否の境界を試せる。fixtureにだけ含める値と実行profileの設定値を区別する。

ゲートAの通過は、上記資料から作った独立実装と第一実装が、正常・境界・fault・resetの共通device試験と固定CPUを通すmachine試験を通した後に判断する。記録には対象契約・試験・実装のdigest、host環境、case別結果、独立実装が参照した資料、仕様不足とその修正を残す。単に二実装の出力が互いに一致するだけでは、共通の誤りを検出できないため不十分である。

F20の同じrunner binaryへ二実装を接続する試作、F19の製品・必要本文検査、ゲートB/Cのbuild/runはそれぞれ別の受入条件として維持する。本案の動作試験が通ってもnative接続や製品包装を通過したとは扱わない。
