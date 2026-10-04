# Ondのマシン構築体験と中期機能設計案

状態: 検討用の非規範設計案。作成日: 2026年10月4日。

Ondを、Kagura上の仮想コンピュータを組み立て、その内部を観測しながらプログラムを作り、複数のOSへ配布できる開発環境へ育てる。ボード上のチップ、メモリマップ、動作中の波形やVRAMを同じマシンの別の見方として提供する。Enbuは、この仕組みで構築できる代表的なマシンとする。

その基盤は厳密な仕様書駆動とする。deviceは実装から独立した規範仕様で定義し、元の実装や配布サービスが失われても、保存された仕様から実行環境を再実装できることを目指す。保存されたアプリケーションのbinaryとassetsを、その新しい実装上で再び動かせることを長期的な価値とする。

deviceは仕様書と少なくとも一つのhost向け実装を持つ製品として共有できるようにする。通常の作品配布にも必要な仕様本文を必須同梱し、作品が広まるほどコンピュータの設計資料も分散して残る構成にする。規範仕様に加えて設計理由や来歴を保存できるようにし、民俗学的な資料としての価値も持たせる。

本書は、開発者が中期の完成条件と機能の境界を判断するための全体案である。以下の名称、コマンド、ファイル構成、契約は提案であり、既存の言語仕様、ABI、公開API、保存形式の変更を確定するものではない。

利用者の具体的な操作と必要機能は[マシン構築とアプリ開発フロー](machine-workflow.md)、版ごとの配分とリリース条件は[バージョン計画](machine-roadmap.md)で扱う。

製品の最小構成、共有形式、通常成果物への仕様同梱、分散資料からの復元は[デバイス製品のパッケージ](device-package.md)で具体化する。

製品を一台のmachineへ組み立てる責務と、device実装を呼び出す接続profileは[デバイスの組立境界](device-assembly.md)で扱う。

## 目指す完成体験

ユーザーはテンプレートからボードを開き、部品棚からCommonGraphicやCommonAudioを載せる。各チップの接続先と設定を選び、メモリマップで競合がないことを確認する。短いOndプログラムを実行すると、画面に絵が出て音が鳴り、チップを選ぶとVRAMや波形が見える。

配布先をWindowsからmacOSやLinuxへ切り替えても、ボードやプログラムは変えない。ツールが同じデバイス契約を実装する配布先向けの部品を選ぶ。受け取った人は、開発ツールやRustの導入、部品の手動解決をせずに、そのOS向けの実行パッケージを起動できる。

ユーザーが普段選ぶのは「どんなコンピュータを作るか」と「どこで動かすか」である。OSやCPUアーキテクチャに応じた実装の選択はツールが担い、必要な場合だけ選択結果を表示する。

## 中期の完成条件

| ID | 実際にできること | 合格条件の案 |
| --- | --- | --- |
| G1 | ボードを組む | GUIでdeviceを追加し、MMIO範囲を設定し、保存して開き直せる。CLIで同じ定義を検証できる |
| G2 | 接続を理解する | チップ、bus接続、address範囲が対応する。衝突した範囲と相手deviceが表示され、不正な構成で起動しない |
| G3 | 内部を覗く | GPUのguest VRAMと出力画面、audioの生成サンプルと波形を観測できる。停止して同じ仮想時刻の状態を読める |
| G4 | OSをまたいで動かす | 同じ解決済みmachine定義と同じguest imageをWindows、macOS、Linuxの明示した検証環境で実行できる |
| G5 | 作品と設計資料を渡す | 配布先別runner、device実装、guest image、assets、必要な仕様本文をまとめる。開発ツールなしに起動でき、ネットワークなしで仕様を読める |
| G6 | Enbuを構築する | Enbu固有deviceや包装処理を、共通の拡張点で登録できる。Ond compilerにEnbu名での分岐を加えずに構成できる |
| G7 | 仕様から実行環境を復元する | 最小deviceを、元の実装を参照せず保存された規範仕様から独立に実装する。共通適合試験を通し、同じmachine定義と未変更のguest imageで規定の出力を再現する |
| G8 | アプリと実行環境を別々に作る | host用device binaryなしでアプリをbuildし、アプリのsourceやimageなしでruntimeを生成できる。後から両者を指定して起動し、適合する別runtimeにも同じアプリ成果物を渡せる |

G4の初期検証候補はWindows x64、macOS arm64、Linux x64の一つの明示した配布環境とする。対応OS最小version、Linuxの実行環境、配布先CPUはdeviceの配布契約を設計するときに確定する。「各OSのすべての環境」を保証する意味ではない。

同じ入力列と仮想時刻に対し、guestが観測する状態と規定した画素・音声サンプルが一致することを互換性試験の中心とする。ウィンドウの描画時刻や物理スピーカーの遅延まで同一とはしない。異なるmachine構成間で任意のプログラムを無変更で動かすことは、中期の必須条件に含めない。

## 仕様書を正本とする原則

Device Contractの正本は、特定の実装から独立して読める規範仕様書とする。実装の言語、内部構造、使用するgraphicsやaudioのlibraryを固定せず、仕様が定義する状態遷移、外部から観測できる値、時間と入出力の関係への適合を要求する。ソフトウェア、別のVM、将来のhardware上の実装であっても、同じ契約を満たしmachineへの接続条件を備えれば交換できる。

規範仕様、適合試験、参照実装の役割を次のように分ける。

| 資料 | 役割 | 食い違いが見つかった場合 |
| --- | --- | --- |
| 規範仕様 | 必須動作、許される動作、適用範囲、互換性を定義する正本 | 曖昧さや矛盾は仕様の課題として解消し、実装の偶然の挙動で補わない |
| 適合試験仕様とtest vector | 規範要件にIDで対応付け、異なる実装を同じ方法で検査する | 正本の要件に照らして修正する。試験に通るだけで未検査の全要件への適合が証明されたとはしない |
| 参照実装とSDK | 仕様の理解と実装を助ける非規範の成果物 | 正本に適合するよう修正する。ソースコードやSDK関数だけを動作説明の代わりにしない |

規範仕様に表、schema、疑似コード、数式、固定データを含めてよいが、何が規範であるかと文書間の優先関係を明示する。参照する資料は正確なrevisionを固定し、必要な内容を保存できる形式にする。読むために特定の現行アプリやオンラインサービスを必要としない。

仕様versionと実装versionを分離する。公開した仕様の本文は識別可能なrevisionとして保存し、訂正や互換性への影響を履歴に記す。同じ識別子の意味を後から黙って変えない。machineは必要な仕様revisionと設定を固定し、互換性のある別仕様への更新にも根拠を要求する。

## デバイス仕様に必要な内容

registerの名前と配置に加え、その値がいつどのように変わるかまで定義する。例えばCommonGraphicの描画方式を交換可能にするには、VRAM形式、色、描画順、範囲外の扱い、blendや丸めの結果が読者の推測に依存しない必要がある。

| 項目 | 規定する内容 |
| --- | --- |
| 識別と構成 | 仕様IDとrevision、設定値、既定値、許容範囲、必須機能、拡張機能とその識別 |
| 状態と初期化 | 状態変数、電源投入時とreset時の値、memory初期値、起動と終了の条件 |
| busへの接続 | windowの大きさ、offset、access幅、endianness、alignment、readとwriteの副作用 |
| 状態遷移とfault | 操作ごとの事前条件、更新順、成功結果、拒否される操作、fault時に保持する状態 |
| 時間と外部入力 | 仮想時刻、完了時点、同時イベントの優先順、入力の取り込み、待機、bufferのoverflowとunderflow |
| 画像と音声の意味 | pixelとsampleの形式、変換、整数幅、丸め、clipping、channel順、規定した出力の比較方法 |
| 観測 | 公開する論理状態と形式、snapshot時点、副作用がない条件。実装内部の構造は要求しない |
| 適合性 | 要件ID、正常系、境界値、不正access、時間依存動作の試験と期待結果、適用するprofile |

アプリケーションの規定動作が依存する部分は、未定義のまま実装に委ねない。予約registerや未対応操作の扱いも決める。差を許容する部分は許容範囲と比較方法を明記し、複数の結果が合法なら単一の出力再現は約束しない。厳密な再生が必要な機能には、出力を一意に決めるprofileを用意する。乱数、時刻、外部入力が影響する場合は、それらも再現条件として扱う。

device単体の仕様だけではmachineの動作は決まらない。Kagura ISAとbus、device間の接続、memory map、仮想時間、起動、imageとassetsの読込形式も、versionを固定した規範契約で説明できる必要がある。各deviceへの適合と、組み合わせたmachineへの適合を別に確認する。

## 体験から逆算した機能

| 体験 | 必要な機能 | 支える共通モデル |
| --- | --- | --- |
| チップを選んで載せる | 部品棚、互換version、設定項目、対応先の表示 | Device CatalogとDevice Contract |
| ボードを組み替える | 配置、選択、追加と削除、接続編集、undoとredo | Machine DefinitionとBoard Layout |
| メモリを割り当てる | 空き範囲の提案、手動割り当て、競合とalignment検証 | Device Instance、bus mapping、共有validator |
| 書いて動かす | machine用Ond API、compile、link、起動と終了、診断 | SDK生成とMachine Runtime |
| 中身を調べる | register、メモリ、画像、波形、状態、停止と再開 | Inspection Protocolと型付きview |
| 別のOSへ渡す | 実装自動解決、依存の固定、包装、起動時互換性検査 | Host Target、Host Implementation、Package |

ボードは動作モデルを操作するGUIとして扱う。チップの座標、色、形、配線の引き回しは表示設定であり、移動だけでaddressや実行結果を変えない。接続変更は明示的な操作として行う。GUIの操作履歴はmachine定義の変更をundoでき、CLIやテキスト編集による変更も読み込めるようにする。

標準接続は、各deviceの公開するbus windowをKaguraのaddress空間へmapするものとする。チップを動かすたびに自動でaddressを変更する方式は採らない。仮想画面や音声出力など、hostへの接続もMMIO接続と区別して見せる。任意の電気配線、信号伝搬、電圧のシミュレーションは後の別機能とする。

## 画面の構成

主画面は中央のボード、選択したチップの詳細、メモリマップを連動させる。部品を選ぶと、そのチップ、接続線、address範囲が同時に強調される。詳細には「設定」「接続」「内部状態」を用意し、内部状態から波形やVRAMを開く。複数deviceの観測パネルを固定できるようにし、描画と音声を同時に比較できるようにする。

プログラムの出力画面とdevice内部のプレビューは区別する。GPUの表示画面は最終出力であり、VRAM viewはguest側のメモリや画像surfaceを解釈したものになる。host GPU固有の未公開メモリを覗くことは、共通機能の条件に含めない。

| 対象 | 最初に提供する観測 | 次の拡張候補 |
| --- | --- | --- |
| CPU | PC、register、停止理由、対応するsource位置 | breakpoint、watchpoint、命令trace |
| RAMとROM | address付きbyte表示、領域名、更新箇所 | 比較、検索、構造体表示 |
| Graphics | 出力surface、guest VRAM、register、更新領域 | tile、palette、sprite、command queue |
| Audio | deviceが生成したPCMの波形、channel、buffer状態 | spectrum、channel solo、イベントtrace |
| Input | buttonやaxisの状態、入力イベント | 入力記録と再生 |

起動中の観測は購読した範囲だけ更新する。構成変更はまず停止状態に限定し、変更後は再検証と必要な再buildを行って再起動する。停止時のmemory書き換えも、単に読む観測とは別の明示操作とする。部品がない、配布先実装がない、古いimageが選ばれた等の失敗は、該当チップに結び付けて説明する。

## マシンと実行先の分離

Kagura target、仮想machine、host targetの三つを区別する。Kagura targetはISAとOnd ABI、仮想machineはdevice構成と起動環境、host targetはそのmachineを動かすOSやCPUの条件である。

```text
Ond source + 解決済みMachine Definition
    → device用Ond packageとlink設定
    → compilerとlinker
    → アプリ成果物（guest image + assets + machine定義 + 必要な規範仕様本文）

解決済みMachine Definition + 配布先のHost Target
    → 対応するrunnerと各deviceのHost Implementationを解決
    → runtime成果物（CPU、bus、device、loader、対応machine定義と規範仕様本文）

アプリ成果物 + runtime成果物
    → 互換性検査 → loadして実行
    → 任意で配布先別の一括パッケージへ包装

Machine Runtime → 観測snapshot → GUIのボードとdevice view
```

同じmachineなら、WindowsからmacOSへ変えてもKagura guest imageの再compileは原則不要とする。address mapやdevice契約を変えた場合はmachineが変わるため、guest API生成やlinkを再実行する。ボードの見た目だけの変更はguest成果物を無効化しない。

GUIとCLIは、同じmachine読み込み、依存解決、検証、build、実行サービスを呼ぶ。GUI専用の隠れた設定に依存しない。GUIのwidget状態を正式なmachine定義の代わりに保存しない。

## アプリとランタイムの独立生成

Ond実行バイナリは、Kagura上で動くguestプログラムとして扱う。そのimage、assets、必要なmachine契約を持つアプリ成果物と、host上でKagura CPU・bus・deviceを動かすruntime成果物を、それぞれ独立して出力する。同じmachine向けの複数アプリでruntimeを再利用でき、同じアプリを適合する別のruntimeへ渡せるようにする。

アプリのbuildにはmachine仕様、device用のguest API、link設定を使い、host用device binaryの入手や実行を要求しない。guest APIを生成するためだけにhost用device binaryをloadする設計も避ける。runtime生成はmachine定義とhost向け実装から行い、個別アプリのsource、image、entry addressを要求しない。runtime生成には既成の配布物の解決と組み立ても含み、Ond compilerがnative device実装までcompileすることは前提にしない。

両者を結ぶ正本は、解決済みの論理machine定義と、それが参照する規範仕様である。アプリ成果物は必要なISA・ABI、device仕様revisionと設定、address map、起動・終了契約、load形式を識別できるようにし、runtimeはそれらへの対応を示す。初期は同じ論理machine契約への対応を要求し、将来の互換profile間の受入れは明示した規則による。host実装の名前やbinary hashでアプリの実行互換性を判定しない。

起動時に対応を検査し、loaderがアプリ成果物に記録されたsection配置とentryを読み取って初期化する。実際の配置がmachineの領域制約を満たすことも確認し、不適合ならguest実行前に拒否する。アプリ固有のentryやassetの内容をruntimeへ固定して、アプリ更新のたびにruntimeの再生成を必要にすることは避ける。

ここで分離するruntimeはhost側の実行環境である。Ond compilerが生成・linkするheap管理や演算helper、guest側device driverなどは、Kagura上で動くguest codeとしてアプリimageへ含められる。machine固有のboot ROMを別途持つ場合は、アプリに依存しない内容をmachine資源として保存し、起動契約を固定する。

device一個ごとの配布artifactと、それらを組み合わせたmachine runtimeも区別する。deviceをruntimeへ静的に組み込む方式でも、アプリとruntimeの分離は成立する。device単位の動的load方式は、別途決める実装上の選択である。

device製品が仕様とhost実装の両方を持っていても、アプリ成果物へ製品全体を混ぜる必要はない。アプリには必要な仕様本文を、runtimeには必要な仕様本文と選択した実装を収録する。各成果物を別々に渡しても仕様を読めるようにし、外部cacheだけに本文を残す方式にしない。

## データと責務の境界

| 概念 | 保持する内容 | versionと識別の考え方 |
| --- | --- | --- |
| Device Contract | 規範仕様、guestに見える動作、MMIO、設定、reset、時間、観測契約。Ond APIはこの仕様に対応するbinding | 表示名ではなく名前空間付きIDと仕様revisionで識別 |
| Device Package | 仕様一式と一つ以上のhost実装、製品情報、目録、配布条件を持つ共有可能な製品 | 製品ID、release、内容digestを仕様や実装の版と分離。形式上の成立と適合検証済みを区別 |
| Host Implementation | 対応するDevice Contract、host条件、runtime接続version、実装binaryと必要な資源 | 契約versionとは別に実装versionと内容hashを持つ |
| Host Connection Profile | instance生成・破棄、window access、時間、入出力、観測、buffer所有権、binary接続 | guest動作仕様とは別のID・revisionで管理 |
| Device Instance | `gpu0`等の安定したID、選択契約、設定、各windowのbusへの割り当て | 同じdeviceを複数載せられる |
| Machine Definition | CPU profile、memory、device instances、接続、仮想時間、起動と終了、guest SDKの結合 | 規範仕様と構成から互換性を識別し、host実装名やbinary hashを含めない |
| Board Layout | チップ座標、向き、表示名、配線表示、固定した観測パネル | machineの実行互換性hashに含めない |
| Resolution Lock | 解決した契約、SDK、実装version、host別artifact、hash | 当時のbuildと配布を再現する記録。machineの実行互換性とは区別 |
| Assembly Plan | runnerと実装の対応、instance生成、window配送先、host serviceの割当て | host別の組立結果。論理machineを識別するがアプリのentryは保持しない |
| Inspection Schema | register表、メモリ領域、画像surface、sample streamとその解釈 | 実行契約から識別可能にし、対応しないviewは利用不可と表示 |

`machine.toml`、`machine.lock`、`board.layout.json`は保存ファイル名の候補とする。現行の`ond.toml`と`ond.lock`をmachine設定で拡張するかどうかは別途判断する。内容と責務を先に定義し、拡張子やschemaは次段階で確定する。

machine全体のbus mapを検証した後、code配置に必要な部分だけを既存`LinkPlan`へ渡す。`LinkPlan`だけではMMIO一覧、device、起動方法、観測契約を表現しきれない。RAM、ROM、MMIOの重なり、範囲のoverflow、access幅、device固有のalignment、必要windowの欠落、予約領域を共通validatorで検査する。bankやaliasは明示された契約に限って扱い、暗黙の重複mapにはしない。

## CommonGraphicを選んだときの動作

CommonGraphicは共通の仮想graphics deviceの表示名とする。以下のIDとversionは説明用の仮例であり、実在する配布物を意味しない。

```text
論理device: example/common-graphic @ contract 1
    ├─ 共通: guest register、VRAM形式、描画の意味、Ond API、観測schema
    ├─ Windows x64用実装
    ├─ macOS arm64用実装
    └─ Linux x64用実装
```

host実装は同じguest契約を満たす必要がある。OSによりMMIO意味やpixel formatが変わるものを、同じ論理deviceとして隠さない。共通動作を担うcoreとOSの画面・音声出力adapterを共有できれば実装を共通化する。CPU描画かGPU描画かは、契約への適合と性能要件を満たす範囲で実装側が選べる。

動作仕様への適合と、あるrunnerへbinaryを接続するための条件は区別する。共通の動作仕様を満たす実装は、runnerが扱えるadapterや読込形式を通じて利用する。特定のplugin ABIへの依存を、論理deviceの同一性や将来の再実装の前提にしない。実装固有の拡張にアプリケーションが依存する場合は、その拡張も明示した規範契約としてmachineに記録する。

解決器は、契約IDとversion、OS、CPU architecture、必要なruntime接続version、最小OSやsystem library等の要件に合う候補を選ぶ。lockがある場合は固定された候補を使う。候補がなければ、足りないdeviceと対応先を一覧で返す。未知の実装への置き換えやソースからの自動buildを暗黙に行わない。

将来の復元では、元のhost binaryが入手できなくても、同じ仕様revisionとmachine条件を満たす別実装を明示的に登録して再解決できるようにする。元のlockを保存し、新しい実装と適合根拠を別の解決記録へ残す。host実装の交換だけではmachineの実行互換性を変えず、元のbinary hashの一致を起動条件にしない。当時の配布物の完全再現と、別実装によるアプリケーションの再実行を区別する。

通常の実行では現在のhostを使い、exportでは明示された配布先で解決する。あるOSから別OS向けの既存binaryを集めて包装できても、署名、installer作成、起動検証までそのOS上で完結できるとは限らない。中期の約束は、各対応先で検証済みの実行パッケージを作れることとする。

device binaryの読込形式は、配布・性能・隔離の試作で判断する。[組立境界の案](device-assembly.md)では、組む人にnative compilerを要求しないため、既成runnerと版付き接続profileを持つnative moduleを第一候補にする。静的方式は収録済みrunnerの選択、または発行者側のlink工程として区別する。最初は同梱した信頼済みの実装集合で一種類のprofileを評価し、一般の第三者binaryはその導入・信頼モデルを定めてから扱う。同一processのnative moduleに対し、宣言した権限だけで隔離を保証しない。

## 実行時間と観測の契約

観測で通常のMMIO readを呼ばない。readでFIFOを消費したりflagがclearされたりするdeviceもあり、GUIを開くだけで実行結果が変わるためである。runtimeが副作用のないsnapshotやstreamの複製を作り、GUIはそれを読む。

snapshotにはdevice instance、実行session、仮想時刻またはepochを付ける。実行中は頻度や容量を制限した近似的なlive表示を提供し、各表示の時刻を明示する。停止時には同じepochのCPU、memory、device状態を読む。履歴のaudio streamと現在のregister snapshotを同時刻の値と誤認させない。

VRAMは関心領域や更新差分、audioは時刻付きsample buffer、bus activityは集計値を使う。表示が追い付かない場合は観測データを間引き、欠落と古さを表示する。UI更新のためにguest時間を進めたり、音声sampleを作り直したりしない。観測の有無でguest状態遷移は変えず、観測の負荷がhost上の実行速度に影響し得ることは区別する。

KaguraのISAは共通のcycle数やdevice clockを規定していない。仮想時間の単位、命令実行からdevice tickへの進行、入力の取り込み順、描画とaudioの更新順をMachine Runtime側の契約として定義する。初期案は命令の確定境界でdeviceを進める単純なモデルとし、具体的な比率は試作で確定する。host時刻での画面表示と音声再生は出力adapterが担う。

同じ入力記録を与える検証modeを用意し、観測ON/OFFと各OS実装間で、規定時刻のguest状態、画素、生成音声を比較する。nativeな画面やaudio出力が使えないheadless実行でも、規定した出力bufferを検証できるようにする。

## コマンドと成果物

機能の入り口はユーザー案の`ond kagura-device`と`ond kagura-machine`を仮置きする。具体的な引数は未確定であり、以下は操作の責務を示す例である。

```console
ond kagura-device list
ond kagura-device inspect CommonGraphic
ond kagura-machine new hobby-board
ond kagura-machine edit hobby-board
ond kagura-machine check hobby-board
ond kagura-machine build hobby-board --project ./demo -o ./out/demo-app
ond kagura-machine build-runtime hobby-board --host windows-x64 -o ./out/runtime-windows
ond kagura-machine run --app ./out/demo-app --runtime ./out/runtime-windows
ond kagura-machine export --app ./out/demo-app --runtime ./out/runtime-windows -o ./out/demo-windows
```

`edit`はGUIを開く想定、`build`はmachine向けアプリ成果物生成、`build-runtime`は配布先別の実行環境生成、`run`は指定した二成果物の互換性を検査して実行、`export`は二成果物を一括配布用に包装する操作とする。出力先は説明用のdirectory例であり、正式な保存形式や拡張子は未確定である。アプリのbuildは内部で既存の`ond compile`と`ond link`を利用する。

日常操作には、projectとmachineを指定するだけで必要なbuildと現在のhost向けruntime解決を行う`run`の短縮形も用意できる。この場合も内部では二成果物を分けて保存する。`export`は元のアプリ成果物を再compileせずに同梱し、machineが異なるimageとruntimeの組み合わせを拒否する。

| 成果物 | 内容 | OS依存 |
| --- | --- | --- |
| マシン設計 | device構成、map、契約の固定情報、ボード表示 | 論理定義はOS非依存 |
| デバイス製品 | 規範仕様とその依存本文、一つ以上のhost実装、製品manifest、目録と配布条件 | 一つのhost版だけでも成立する |
| アプリ成果物 | Kagura image、assets、解決済みmachine定義、必要仕様本文と索引、任意のdebug情報 | 同じmachineなら原則OS非依存。host実装を含まない |
| runtime成果物 | 対応先のCPU、bus、device、loader、必要なmachine資源と定義、必要仕様本文と索引 | 配布先ごとに異なる。個別アプリを含まない |
| 一括配布パッケージ | 独立生成したアプリ成果物とruntime成果物、起動用設定、仕様の閲覧入口 | 配布先ごとに異なる。個別配布も可能 |

任意のdebug情報やボードeditorは、通常の実行パッケージに必須としない。一括配布で一つのarchiveや実行ファイルにまとめる場合も、元のアプリ成果物をそのまま取り出せるようにし、別のruntimeで実行する経路を維持する。

現行の`.ondimage`はload addressやentryを含む`LinkedImage`のJSON/Base64表現であり、OSが直接起動するnative executableではない。これはアプリ側の分離に使える既存の土台だが、完全なmachine契約を含むアプリpackageや独立したruntime生成は新たに必要になる。現行の`LinkedImage`を無承認で拡張せず、当初は外側のpackage metadataに機種契約を保持する案とする。

## 将来の復元に必要な保存物

通常の最終成果物にも、その解釈に必要な規範仕様一式を必須同梱する。アプリケーションのguest imageとassets、必要な起動ROMや別途loadするguest library、解決済みmachine定義を、それぞれの成果物の責務に従って保存する。仕様のURLやhashだけでなく、必要な本文、schema、固定データを残す。参照先がさらに参照するKagura、device、時間、起動、image形式等の契約も、依存が閉じるまで収録する。仕様の欠落を黙認して最終成果物を生成しない。

拡張保存用のpackageでは、適合試験仕様とtest vector、設計理由、変更履歴、由来等も保存し、将来の別実装の検証と設計の背景理解を助ける。通常配布との違いは仕様本文の有無ではなく、この追加資料の範囲である。元のnative binaryは当時の実装資料として残せるが、その起動を復元の必須条件にしない。特定の実行履歴まで再現する場合は、初期状態、seed、必要な外部入力列や保存データも合わせて保存する。

多数の作品に同じ仕様が残ることを欠落補完に利用する。照合には仕様ID、revision、内容digest、由来を使い、コピー数だけで仕様の正誤を決めない。異版や派生を併存させ、少数の資料を多数派へ上書きしない。

仕様から復元する対象は、保存したアプリケーションを実行する環境とその規定動作である。仕様だけから失われたアプリケーション固有のコードや画像、音楽を復元することはできない。sourceからの再buildも保存目標に含める場合は、source、依存source、対応するOnd言語・ABI・build契約などを追加で保存する。

## ゴールから逆算した実装順序

版ごとの詳細な範囲と終了条件は[バージョン計画](machine-roadmap.md)を参照する。現時点の配分案は次のとおり。

| 版の案 | 完成させる体験 |
| --- | --- |
| 0.2.0 | 仕様と独立実装を先に検証し、CLIから一台のmachineを動かす。製品形式の基礎を定め、必要仕様本文を持つアプリとruntimeを別々に生成する |
| 0.3.0 | GUIで組み替え、入力で図形を動かして音を鳴らし、VRAMと波形を観測する |
| 0.4.0 | 外部assetを取り込み、Windowsの受取側へ開発環境なしで起動できる作品を渡す |
| 0.5.0 | 同じアプリ成果物をWindows、macOS、Linuxの明示した検証環境へ運ぶ |
| 0.6.0 | 外部device製品を受け渡し、Enbuを共通モデルで構築する。来歴等も含む保存物と分散資料から別実装へ復元する |
| 1.0.0候補 | 保証する仕様・成果物・公開境界を明示し、長期互換性と移行の方針を確立する |

0.2.0の固定画像から始め、0.3.0までに「図形を入力で動かし、音を鳴らし、内部状態を見る」という一周を完成させる。MMIOを別の空き範囲へ移し、生成されたdevice APIを使う同じsourceを再buildして動かせることも確認する。

詳細なGPU、多数のdevice、3Dのボード表示、公開registry、任意の第三者plugin、hot-plug、逆実行、全入力の永続replay、遠隔デバッグ、異種machine間のcapability抽象化は、この一周を成立させた後に判断する。各OSの実機検証環境の準備は0.2.0から進め、最後まで未検証のままにしない。

## 現状との接続と次の設計対象

現状のOndはcompiler-core、Kagura backend、Object、linker、LinkedImageとdebug sidecarを持ち、機種固有の起動・包装・device APIをSDK側へ分離している。この上にmachine統合層を置く。Kagura規格と隣接リポジトリの変更は本案の前提にしない。

現行の[パッケージビルド契約](../../specs/ond/package-build.md)は`ond build`と`ond assets`を復活させないと明記している。前の議論にあったトップレベル`ond build`案は、この境界との整合を再検討する必要がある。本案ではmachine操作の配下に仮置きし、既存CLI契約の変更とは分ける。

次の詳細設計では、まずDevice Contract、Machine Definition、Inspection Schemaの三つを、最小graphics/audio deviceの具体例で作る。Device Contractは独立に実装できる規範仕様と適合試験へ具体化する。その例から仮想時間と起動手順を確定し、runtimeへのbinary接続方式を比較する。Enbuの現物との互換性確認は別途必要であり、本書ではEnbuの全要件を調査済みとは扱わない。

参照した現状の根拠:

- [Ondの構成とSDK境界](../../README.md)
- [Kagura targetのruntime契約](../../specs/ond/targets/kagura-v1/machine.md)
- [MIR backendの責務分割](../../specs/ond/targets/kagura-v1/mir-backend.md)
- [LinkPlanとLinkedImageの実装](../../crates/protocol/src/link.rs)
- [debug sidecarの仕様](../../specs/ond/debug-metadata.md)

本書の完成は全体機能案の提示を意味する。machine runtime、GUI、device配布、各OSでの実行は今後実装・検証する対象である。
