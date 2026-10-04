# デバイスの組立境界とランタイム接続の設計案

状態: 検討用の非規範設計案。作成日: 2026年10月4日。

[デバイス製品](device-package.md)を配れるだけでは、任意のbinaryを一台のmachineへ接続できない。本書は、device作者が提供するもの、machine作者が配線するもの、runtimeが管理するものを分け、組立から起動までの責務を定める案である。新しいABI、公開API、Kagura仕様や依存関係の変更は、この文書だけでは承認・実装しない。

利用者が部品を組むためにnative compilerやlinkerを導入しなくてよいことを前提に、既成runnerと共通の接続profileに対応したdevice moduleを組み合わせる方式を第一候補とする。device作者はそのprofile向けに実装をbuildして配布する。Ondのアプリcompile、device実装のnative build、machine runtimeの組立は別の工程である。

## 三つの契約

| 契約 | 定めるもの | 定めないもの |
| --- | --- | --- |
| デバイス動作契約 | register、window、状態、reset、時間、入出力、fault、観測の意味 | DLL等の読込方法、ボード上の絶対address、特定runnerの内部型 |
| マシン構成契約 | CPU、RAM、device instance、windowの絶対address、接続、時計、起動・終了 | host実装の内部構造、特定アプリのsource |
| ホスト接続契約 | factory、instance handle、呼出し、buffer所有権、エラー、寿命、対応するbinary形式 | guest命令やデバイス動作の別定義 |

deviceの動作契約に適合していても、runnerが対応しない接続profileのbinaryは直接使えない。逆に、同じ接続profileでloadできても、必要な動作仕様へ適合しているとは限らない。仕様・構成への適合とbinary接続の適合を別に検査する。将来の再実装は古いnative ABIまで再現する必要はなく、同じ動作契約に適合する新しい接続手段を選べる。

動作契約は人間が再実装できる本文を正本とし、組立に必要なwindowや設定制約は対応する宣言的schemaでも提供する。仕様の自由文をツールが推測して配線する方式にしない。schemaの各項目を規範要件へ対応付け、本文との不一致は仕様の問題として解消する。

## 誰が何を提供するか

| 担当 | 提供・管理するもの | 他へ任せるもの |
| --- | --- | --- |
| デバイス作者 | 相対offsetを持つwindow、設定の制約、内部状態、論理的な画像や音声、実装module、適合根拠 | machine全体のmap、CPUの実行loop、エディタUI |
| マシン作者 | deviceの選択とinstance名、絶対addressへの割当て、論理port接続、時間・起動profile | device内部のbuffer配置、OS APIの選択 |
| ランタイム作者 | CPU実行、bus配送、instanceの寿命、仮想時間、host入出力service、診断、観測の仲介 | 個々の作品のプログラム、deviceの規範動作の変更 |
| アプリ作者 | 生成されたguest APIを使うOndプログラムとassets | native moduleのload、hostのpointerやwindow handle |

CPUとRAMもボード上ではチップとして表示できる。ただし最初の接続profileでCPU自体を任意pluginへ交換できるようにはしない。CPU・bus・loader・標準RAMはrunnerが提供する基盤、graphics等は接続するdeviceとして始める。この実装上の区別をUIの見た目と混同しない。

## 組立から実行まで

```text
仕様一式 + machine構成
    → 論理構成の検証と固定
    ├─ guest SDK + link設定 → Ond compile/link → アプリ成果物
    └─ host選択 + 接続profile + 信頼方針
         → 実装の解決 → host別組立計画 → runtime成果物

runtime起動
    → 許可されたmoduleをload → instance生成 → windowとserviceを接続
    → machine初期化 → アプリをload → CPU実行とdevice進行
```

組立計画は、論理machineの識別、runnerの版と内容hash、選択した実装と接続profile、instanceとfactoryの対応、windowの配送先、host serviceへの割当てを持つ。deviceの内部へのpointerや実行時handleは保存しない。時計やevent順序をhost別計画で勝手に変えず、論理machineの規範契約から取得する。

`build-runtime`はmanifest等の宣言だけから対応するrunner・module・依存資源を選び、固定した組立計画と仕様本文を包装する。原則としてこの工程でmoduleをloadしたり、利用者の環境でnative linkを行ったりしない。別OS向けの包装と、そのOS上でのload・動作検証は別の結果として記録する。

起動時はまず宣言、hash、対応host、接続profile、必要service、信頼方針を検査する。binaryのload自体からコードが動く前提で扱い、自己申告の情報を問い合わせるためだけに未承認binaryをloadしない。許可されたものをloadした後も、返されたinterfaceの版・長さ・能力と宣言の一致を確認する。

すべてのinstanceと必須接続を作り終えるまでguestを実行しない。途中で失敗したら、作成済みinstanceとserviceを逆順に解放し、不完全なmachineを起動しない。作成段階ではguestへのeventや画面・音声の出力を開始しない。

初期化はmachine契約で定めた順にCPU・RAM・deviceをresetし、ROM等を用意し、アプリのsectionをloadし、entryとSPを設定して実行へ移る。アプリのentryはload時の入力であり、runtimeをbuildする時点では不要である。再起動でも古いsessionの入力や出力を引き継がない。

## CommonGraphicを一個載せる例

以下は[制作フロー](machine-workflow.md)と同じ仮例であり、正式なCommonGraphic仕様ではない。

| 接点 | deviceが提供するもの | machineやruntimeが決めるもの |
| --- | --- | --- |
| `registers` | 256 byteのwindow、許可access幅、相対register offset | 先頭を`0x10000000`へ割り当てる |
| `vram` | 128×128 RGBA8に対応する64 KiBのwindow | 先頭を`0x20000000`へ割り当てる |
| `frame` | 仕様に従った画素とframeの確定時点 | 表示serviceまたはheadlessの出力先へ接続する |
| `inspect` | 副作用のないregisterとVRAMのsnapshot | CLIやGUIが取得し、共通viewerで表示する |

二つのwindowは同じ`gpu0`の状態を参照する。各windowごとにGPUを一個ずつ生成してはいけない。もう一個載せた`gpu1`は別instanceとして独立した状態を持つ。windowのsizeが設定で変わる場合は、load前に宣言的な規則から求め、instance生成後も一致を検査する。

例えばCPUの`write32(0x10000004, value)`をruntimeが`gpu0`の`registers`、offset `4`、幅`4`の書込みへ配送する。deviceはボード全体の絶対addressを知らなくてよい。accessがwindow全体に収まり、alignmentと許可幅が正しいかを配送前に検査する。未対応の32-bit accessを4回の8-bit accessへ黙って分解しない。

一回のread/writeがfaultを返す場合は、その操作の状態変更や出力を残さない。deviceは受理条件を先に検査し、成功する操作だけをcommitする。runtimeが任意moduleの内部状態を後から巻き戻せるとは仮定しない。既に成功した別命令の副作用は保持する。faultした命令内のfetchを含む副作用については、後述のCPU契約も満たす必要がある。

初期案ではVRAMをdevice instanceが所有し、bus経由のaccessとsnapshotを同じ状態へ結び付ける。別のRAMコピーをGUI専用に正本として持たせない。system RAMはruntimeが所有する。deviceへRAM全体のhost pointerやCPUの可変参照を渡さない。

`Present`の処理等で確定した画素は、型と長さを持つ出力としてruntimeへ渡す。物理windowを開くのはhost側の表示serviceであり、GPUチップ自身がエディタのwindowやevent loopを所有しない。最初はコピーするbufferで寿命を明確にし、共有bufferやGPU資源handleは必要性を測ってから別の能力として設計する。

headlessでも同じ論理frameを得る。表示が遅いことを理由にguestのVRAMやframe確定順を変えない。音声も同様にdeviceが生成したPCMとhostの再生を分ける。hostの能力を利用する将来の高速実装は、必要serviceと能力を宣言し、guestからの観測結果を動作契約に一致させる。

## 接続profileが持つ操作

以下は意味上の操作であり、関数名やABIの確定案ではない。

| 操作 | 契約で決めること |
| --- | --- |
| interface確認 | profileの版、必須機能、関数表の長さ、宣言との対応。load前の信頼検査とは別 |
| instance作成と破棄 | 設定、資源制限、service割当て、独立したhandle、失敗時の部分解放 |
| reset | 初期状態、queueの破棄、sessionの更新、永続状態を持つ場合の保持条件 |
| windowのreadとwrite | window ID、相対offset、幅、値、仮想時刻、成功とdevice fault |
| 時間の進行 | 指定した仮想時刻までの状態遷移、eventの順序、処理量の上限 |
| 入力の受渡し | timestamp付き論理入力。入力deviceを加える段階で有効化 |
| 出力の回収 | frame、PCM等の型付き出力、長さ、時刻、bufferの寿命、queue上限 |
| 観測 | 副作用のないsnapshot、範囲、形式、sessionとepoch。通常のMMIO readと分離 |

native module案では、固定幅の値、opaqueなinstance handle、明示した長さのbuffer、版付きinterfaceを境界に用いる。Rustの`dyn Device`、`Any`、`Vec`等を配布ABIとして直接渡す設計にはしない。C形式の境界を候補とする場合も、対象hostの呼出規約、型配置、所有権、エラー値をprofile本文で固定する。Cという名前だけで全host共通のbinaryになるとは扱わない。

bufferを確保した側と解放する側、呼出中だけ借りる領域、呼出後も保持できるhandleを区別する。初期案は呼出元が確保した出力領域へのコピーとし、長さ不足を診断する。サイズ確認を含めsnapshotが消費操作にならないようにする。OS handle、関数pointer、host addressをguestへ公開しない。

## 時間と接続の所有者

初期案ではruntimeがguest状態を変更する呼出しを直列化し、deviceが独自threadからguest状態を更新する方式は採らない。CPU命令境界、deviceの進行、入力適用、出力回収の順序をmachineの時間profileで定める。命令の成功・fault時に仮想時刻を進めるかも規範化し、単にhostの経過時間から決めない。

deviceは他deviceを名前で探したり、その関数を直接呼び出したりしない。必要な論理portを宣言し、machineが接続を定義し、runtimeが配送する。初期のgraphics例ではMMIOとframe出力で成立させ、一般的なdevice間配線をすべて実装する必要はない。host serviceの要求は論理portと分けて記録する。

DMA、共有memory、IRQを必要とするdeviceは、それらを追加の明示した契約として要求する。初期profileで未対応なら組立を拒否し、CPUへの割込み方法等を推測で補わない。DMAを導入する場合も、許可領域と幅、実行時点、fault、既に完了した書込みの扱いを定めたruntime経由のaccessとし、raw pointerによる無制限accessを標準にしない。

ホスト側の非同期処理結果はqueueへ入れ、runtimeが規定した時点で適用する。device呼出中にCPU実行や他deviceへの再入呼出しを許さない。無限処理、同時刻eventの循環、queue枯渇への扱いはprofileに規定し、能力不足を隠してeventを落とさない。guest動作を定めるqueueと、遅れてよいGUI観測のqueueを区別する。

## エラーと安全性

不正なguest accessによるdevice fault、profile不適合や不正buffer等の接続違反、hostの表示・音声出力失敗を別の診断にする。device内の例外等を境界の外へ伝播させない実装契約を定めるが、任意native codeのcrashやmemory破壊を同一process内で回復できるとは約束しない。

serviceだけを渡す設計は責務の整理であって、native codeを技術的に隔離する仕組みではない。最初は同梱の信頼済み実装だけを対象とし、一般の第三者binaryを自動loadしない。隔離を保証するには別processやsandbox方式と、それに対応する通信・資源制限・故障処理が必要な別設計になる。

pauseではguest更新を止めてから同一epochのsnapshotを取得する。resetでは古い入力とframe・PCMを無効化する。破棄では新しい呼出しを止め、進行中処理と借用bufferを回収してinstanceを解放し、最後にmoduleをunloadする。hot-plugや実行中module交換は初期範囲に入れない。

## Binaryの組み込み方

| 方式 | 組む人が行う処理 | 制約と位置付け |
| --- | --- | --- |
| 共通runnerとnative module | 接続profileに対応する既成binaryを選び、配置する | 第一候補。profileごとのloaderとhost検証が必要。同一processなら隔離は保証しない |
| device収録済みrunner | 収録一覧からinstanceを作り、mapと設定を選ぶ | 初期の限定実装に使える。未収録deviceは後付けできず、不足として診断する |
| static libraryからrunnerを生成 | native linkと必要に応じたbuildを行う | runner発行者向け工程。利用者の標準組立の暗黙fallbackにはしない |
| 別processまたはWasm等 | 対応するservice実行基盤へmoduleを接続する | 後続の隔離候補。通信、権限、転送量、障害処理の検証が別途必要 |

native moduleを第一候補とするのは、個別配布された部品の追加をrunnerの再compileから切り離すためである。0.2.0の試作で一種類のhost接続profileを評価して採否を決める。静的収録だけの試作に縮小する場合は、外部製品の自由な組立を実証済みと扱わず、公開計画の制限へ明記する。

製品descriptorにはbinaryの種類、接続profileのID・revision・digest、factory入口、対応する動作契約と能力、必要service、host依存、信頼判断に必要な情報を追加する。runnerも対応profile、提供service、収録済みdeviceがあればその実装ID・hashを宣言する。静的実装のbinaryが製品に入っているだけでは、既成runnerへの接続可否は決まらない。

## 現行コードからの接続

Ondの[テスト用LinkedMachine](../../crates/compiler/src/test_support/machine.rs)は`Cpu`と`DefaultBus`を持ち、RAM、DebugIo、終了deviceをコードで配置し、imageをloadして`Cpu::step`を呼んでいる。[依存定義](../../crates/compiler/Cargo.toml)ではKaguraはdev-dependencyであり、このテスト実装を公開の製品loaderと同一視しない。

調査した隣接checkoutのKagura参照実装では、[Device](../../../kagura/support/reference/rust/kagura/src/device.rs)は幅別のread/writeと`Any`によるdowncast、[DefaultBus](../../../kagura/support/reference/rust/kagura/src/bus.rs)の`map_device`は範囲ごとにdeviceを所有するAPIである。複数windowを共有するinstance管理、時間、reset、型付き観測、外部moduleの接続は、新しいmachine統合層の責務として扱う。参照実装のRust型を配布規格に昇格させない。

候補はruntime側にinstance registryとwindow routerを置き、Kaguraの`Bus`を実装するadapterから幅を保って配送する構造である。一個のinstanceへ複数windowを接続し、観測や時間進行は同じregistryをruntimeが管理する。既存の型別downcastを製品の公開観測APIとして使わない。

現行[OndのKagura runtime契約](../../specs/ond/targets/kagura-v1/machine.md)はscalarを型幅と同じ一回のbus accessとし、aggregate全体のatomicityを保証しない。新adapterでもこの意味を保持し、未対応幅をbyte accessへ分解しない。aggregateを構成する別のscalar命令が既に成功していた場合、その結果まで巻き戻す意味ではない。

[Kagura bus契約](../../../kagura/specs/kagura/v1/bus.md)の一access単位のcommitとfault時の無副作用も維持する。隣接repositoryへのリンクは現状調査の参照であり、将来の配布物に規範本文を同梱する代わりにはならない。

さらに[Kagura CPU契約](../../../kagura/specs/kagura/v1/cpu.md)はfaultした命令全体のdevice副作用なしを要求する。現CPUはfetchも通常の`bus.read32`で行うため、副作用のあるMMIOをfetchし、その後decode等でfaultする場合は、一accessの保証だけでは不十分である。初期machineでは命令fetchを副作用のないRAM・ROM領域へ限定し、対象外ではdeviceのreadを呼ぶ前に停止させる案を検証する。MMIOからのfetchを一般に許す場合は別途transaction等が必要で、初期profileでは保証しない。既存CPUの外側で実行領域制約と正しいfaultを担保できなければ、Kaguraへの依存課題としてblockedを記録する。

この接続案は既存CPUの外側で成立するかを試作して確認する。隣接checkoutとOndが固定するKagura版は同一と仮定せず、実装時に固定版で検証する。Kagura本体の変更が必要と判明した項目はblockedとして記録し、このタスクから隣接repositoryを変更しない。

## 版計画と最初の試験

0.2.0では動作・構成・host接続の三契約を分離し、信頼済みの最小graphicsで一つの接続profileを試す。同じrunner binaryで独立した二実装を差し替え、アプリの再compileもrunnerのnative再linkも不要なことを、module方式の採用条件にする。未承認の第三者binaryは対象にしない。

同じmoduleから`gpu0`と`gpu1`を作り、各々のregisterとVRAMが同じinstanceへ結び付くこと、instance間の状態が混ざらないことを確認する。windowの移動は設定と必要なアプリ再buildだけで対応し、device module自体は再buildしない。

0.3.0では入力、PCM、live観測を同じ寿命・時間管理へ載せる。0.4.0ではnative開発環境のないWindowsで包装済みruntimeを起動する。0.5.0で各hostの接続profile実装を検証し、0.6.0で対応profileを持つ外部製品の登録と信頼判断を伴う実行へ広げる。公開registryや全方式のloaderは必須にしない。

最初の適合試験には、幅・境界・alignment違反、必須windowの欠落、必要service不足、異なるprofile、descriptorと実装の不一致、二個目の作成失敗時の解放、reset後の古い出力、破棄後の呼出し、headlessと表示ありのguest結果一致を含める。read-clear型MMIOを命令fetch先に指定しても、fault前にその副作用が発生しないことを検査する。構成検査と`build-runtime`だけではbinaryを実行しないことも確認する。

次の承認対象は、最小graphicsの規範動作とwindow schema、仮想時間profile、host接続の操作・buffer・寿命、native moduleの具体的ABI、組立計画のschemaである。形式の外枠だけでなく、この小さな実機相当の試作を通してから一般の製品向け規格へ進める。
