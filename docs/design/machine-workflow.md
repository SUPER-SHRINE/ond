# Ondのマシン構築とアプリ開発フロー案

状態: 検討用の非規範設計案。作成日: 2026年10月4日。

この文書は、利用者が部品を選び、Ondプログラムを書き、アプリとruntimeを別々にbuildして配布するまでの操作から、必要な機能を洗い出す。[全体設計](machine-experience.md)の原則を具体化し、機能IDを[バージョン計画](machine-roadmap.md)へ対応付ける。

部品の共有単位と、通常成果物への仕様本文の必須同梱は[デバイス製品のパッケージ](device-package.md)に従う案とする。保存用exportを選ばなくても、作品の配布とともに設計資料が残る。

device作者・machine作者・runtime作者の責務と、binaryを選んで接続する過程は[デバイスの組立境界](device-assembly.md)で具体化する。

以下の`kagura-device`と`kagura-machine`のコマンド、device名、API、保存ファイル名は未実装の提案例であり、現行CLIの利用手順ではない。現行の`ond compile`と`ond link`は下位の処理として利用する。新しいOnd構文を前提にせず、machine向けの通常のOnd packageでAPIを提供する。

## 一周させる作品

小さな画面に図形を表示するマシンを作る。最初は固定画像を表示し、次に入力で図形を動かし、buttonを押すと音が鳴るようにする。ボードからgraphicsを選ぶとVRAM、audioを選ぶと生成PCMの波形を観測できる。最後に、同じアプリを別OS向けruntimeと組み合わせて配布する。

0.2.0では固定画像を表示するところまで、0.3.0では入力と音声を加えてボード上で観測するところまでを完成させる案とする。以下は後続版まで含めた完成時のフローであり、すべてが0.2.0に入るわけではない。

## 保存するものと生成するもの

```text
demo/
├─ machine/
│  ├─ machine.toml                 マシンの論理構成
│  ├─ machine.lock                 仕様とguest SDK等の解決記録
│  ├─ board.layout.json            見た目と観測パネルの配置
│  └─ runtime.windows-x64.lock     host実装の解決記録
├─ app/
│  ├─ ond.toml
│  ├─ ond.lock                     アプリの通常の依存記録
│  └─ main.ond
├─ assets/                         元画像や元音声
├─ .ond/generated/<generation>/    再生成可能なmachine用Ond package
└─ out/
   ├─ demo-app/                    guest image、assets、machine定義、必要仕様本文
   ├─ runtime-windows/             CPU、bus、device、loader、machine定義、必要仕様本文
   └─ demo-windows/                任意の一括配布物
```

`machine/`と`app/`を別のrootにし、アプリの通常のsource探索へmachine資料や生成物が混ざらないようにする。`app/ond.toml`と`app/ond.lock`は既存の役割を保つ。論理仕様・SDKの解決とhost実装の解決は別に扱い、hostのlockがなくてもアプリをbuildできるようにする。lockの具体的なschemaと保存単位は0.2.0の詳細設計で定める。

生成SDKは利用者が直接編集するファイルではない。生成世代を固定して、compilerとLSPが同じ内容を読む。生成完了後に参照先を切り替え、進行中のbuildが読むSDKを上書きしない。上記の永続的な生成SDK配置は新設計案であり、現行の[SDK一時directory運用](../../specs/ond/package-build.md)をそのまま説明したものではない。

## 1 マシンを作り部品を選ぶ

GUIで「マシンを作る」を選び、最小ボードのtemplateを開く。CPU、RAM、起動・終了の仕組みはtemplateに含める。部品棚でCommonGraphicの仕様、設定可能な画面サイズ、必要なmemory window、対応実装を読み、`gpu0`として追加する。後からCommonInputとCommonAudioを追加する。

作成先に既存ファイルがある場合は上書きせず、別の保存先または既存projectを開く操作へ案内する。一つのmachineから複数のアプリを作れるようにし、machineに特定の`main.ond`が必須となる構造にはしない。

```console
ond kagura-machine new ./machine --template minimal
ond kagura-device list
ond kagura-device inspect CommonGraphic
ond kagura-machine add ./machine CommonGraphic --id gpu0
```

初期カタログは同梱した仕様と実装で成立させる。device表示名ではなく仕様IDとrevisionを解決し、実装がない配布先も表示する。仕様は使えるが現在のOS向け実装がない場合、machineの編集やアプリbuildは可能で、runやそのOS向けexportが不足を報告する。

部品は製品として一覧に並べ、製品の作者と版、動作を定める仕様、選択されたhost実装を分けて見せる。同じ仕様を実装する別作者の製品も区別する。「仕様書を開く」は同梱本文を読み、binaryのloadやネットワークアクセスを必要としない。

必要機能: 仕様と適合試験の管理 F01、部品カタログ F02、machineモデル F03、製品形式 F19。

## 2 接続を決めて検証する

ボード上で`gpu0`を選び、register windowとVRAM windowをbusへ接続する。「空き領域へ割り当てる」で候補を出し、必要なら先頭addressを手入力する。チップ、接続線、memory mapの行が連動する。

説明用の配置例は次のとおり。実際の規範仕様や既存Enbuの配置を意味しない。

| 領域 | 先頭address | 容量 | 用途 |
| --- | --- | --- | --- |
| System RAM | `0x00001000` | 256 KiB | code、data、heap、stack。stackは内側に予約 |
| gpu0 registers | `0x10000000` | 256 B | graphics制御 |
| input0 registers | `0x10000100` | 256 B | 0.3.0で加える入力 |
| audio0 registers | `0x10000200` | 256 B | 0.3.0で加える音声制御 |
| gpu0 VRAM | `0x20000000` | 64 KiB | 説明用の128×128 RGBA8 surface |

この表はbus全体の定義ではない。起動用の予約領域、正常終了時の扱い、audioのsample転送領域等はtemplateとdevice仕様で別途確定する。GUIは各仕様が要求する全windowを表示し、割り当て忘れを検出する。

```console
ond kagura-machine map ./machine gpu0 --window registers --base 0x10000000
ond kagura-machine check ./machine
```

重複があれば、相手deviceと衝突範囲を示し、実行を止める。修正して保存するとGUIとCLIで同じ検証結果になる。ボード上の座標だけを変えた場合は、接続とアプリ成果物を変更しない。

deviceはwindowの名前、size、許可幅、相対offsetを宣言し、machineが絶対addressへ割り当てる。registerとVRAMの二つのwindowは同じGPU instanceへ接続する。hostの表示先等は別のservice割当てとして扱い、物理windowの生成をdeviceのguest動作と混ぜない。

必要機能: window単位の配置と検証 F04、ボード編集 F11。

## 3 マシン用APIを用意してプログラムを書く

GUIで「このマシン用のアプリを作る」を選ぶ。`app/ond.toml`と`main.ond`を作り、machineから`board/gpu0`等のOnd packageを生成する。エディタを開いた時点で、生成APIの補完、型の診断、定義移動が使える状態にする。

```console
ond kagura-machine new-app ./machine ./app
ond kagura-machine prepare ./machine --project ./app
```

`prepare`は仕様とguest SDKの解決、machine検証、SDK生成、projectとの関連付けを行う。通常のbuildにも組み込み、利用者に毎回の手動実行を要求しない。host用device binaryをloadしてAPIを生成する方式は採らない。

アプリとmachineの関連付けは新設の統合層で保存し、既存`ond.toml`へ未定義の項目を加えない。関連付けの具体的な形式、compilerへのlibrary rootの受渡し、LSPへの通知は0.2.0で確定する。

次はAPI利用感を確かめるためのOndコード案であり、`Clear`、`SetPixel`、`Present`は未実装の仮APIである。

```ond
package main

import "board/gpu0"

func main() {
    gpu0.Clear(0x102030FF as u32)
    gpu0.SetPixel(64 as u32, 64 as u32, 0xFFFFFFFF as u32)
    gpu0.Present()
}
```

`Clear`や`SetPixel`の高水準処理はguest SDK側で実装できる。ホスト固有の描画APIをアプリへ見せず、規範仕様のVRAMやregister操作へ変換する。`main`終了後の最終画面を保持するか等の振る舞いはmachineの起動・終了契約として決める。

0.3.0では同じprojectに入力と音声のAPIを加える。入力を読み、規定の仮想時刻に画面を更新し、生成したPCMをaudio deviceへ渡す例を用意する。イベントloop、frame待機、audio送信を行うAPIも、このフローに必要なSDK機能として設計する。

キーボードと論理buttonの対応はhost側の設定として扱い、キーリピート、フォーカスを失った場合の押下解除、入力をguestへ渡す時点を明示する。設定可能なキー配置の違いを、deviceのguest仕様の違いとして扱わない。

必要機能: machine用SDK生成 F05、project雛形とLSP連携 F06。

## 4 アプリとランタイムをそれぞれbuildする

GUIの「アプリをbuild」はsource、machine契約、SDKからguest imageと必要なmetadataを生成する。「ランタイムを用意」は選択したhost向けのCPU・bus・device実装を解決し、再利用できる実行環境を生成する。

```console
ond kagura-machine build ./machine --project ./app -o ./out/demo-app
ond kagura-machine build-runtime ./machine --host windows-x64 -o ./out/runtime-windows
```

アプリbuildは内部で`ond compile`へ固定したSDKを渡し、machineから得た配置で`ond link`を呼ぶ。アプリの書換えではruntimeを再生成せず、runtime実装の交換だけではアプリを再compileしない。失敗時には以前の成功成果物を保持し、GUIに最新のbuildが失敗したことを表示する。

アプリとruntimeの各最終成果物には、それぞれ必要なmachine定義と規範仕様本文を同梱する。URLやhashだけで済ませず、必要な参照仕様も揃える。本文の欠落や内容不一致は生成失敗にする。製品のhost binaryまでアプリへ取り込む必要はなく、仕様部分だけでアプリをbuildできる。

RAM不足はcode、data、stack等の内訳と共に示す。cacheにはsource、compiler、生成器、SDK内容、link設定等を反映するが、cacheの一致と実行契約の互換性は別に判定する。生成器を更新したことで再buildが必要になっても、それだけで論理machineが別物になるとは限らない。

device実装が見つからない場合は、どの仕様のどのhost実装が不足するかを示す。互換性の不明な別deviceへ暗黙に交換しない。初期実装は既成のdevice実装を組み立てる方式でよく、利用者に各deviceのソースbuildを要求しない。

`build-runtime`は対応runner・接続profile・module・必要serviceを照合し、host別組立計画を作って包装する。製品descriptorの読取だけで行い、binaryのloadやnative linkは標準の組立工程に含めない。静的収録方式では、runnerに入っている実装だけを選べる。未収録deviceのlibraryがあるだけでは組立成功としない。

必要機能: アプリbuildと成果物 F07、runtime解決と生成 F08、host接続profile F20。

## 5 動かしながら内部を見る

GUIの「実行」は必要なbuildを行い、現在のhost向けruntimeで起動する。成果物を直接指定する操作も用意する。

通常の「実行」は必要なbuildが失敗したら起動を止める。保持した過去の成功成果物は、利用者がその成果物を明示して選んだ場合に実行できる。

```console
ond kagura-machine run --app ./out/demo-app --runtime ./out/runtime-windows
```

起動前にmachine、成果物形式、memory配置、device仕様の対応を検査する。動作中のgraphicsチップを選んでVRAMを開き、audioチップを選んで波形を固定表示する。pauseすると同じepochのCPU・memory・device状態を確認でき、resumeで続きを動かせる。

moduleをloadする前に信頼方針と宣言上の適合を確認する。許可された実装からinstanceを作り、window・serviceを接続し、reset・アプリloadを終えてからCPUを実行する。作成途中の失敗では生成済み資源を解放し、部分的なmachineで実行を始めない。時刻の進行と状態観測はruntimeが管理する。

CPU faultはsource位置と理由、deviceの不正accessはaddressとdevice instanceを対応付けて表示する。観測が遅れた場合はデータの古さを表示し、観測操作のためにdeviceの通常readを呼ばない。0.2.0の最初の観測はCLIからの状態snapshot、0.3.0でボードと連動したlive viewへ広げる。

guest faultとhostの画面・音声出力障害を区別する。音が聞こえないときは、生成PCMの有無と出力先の状態を確認できるようにする。reset時にCPU、device、入力、PCM queueをどう初期化するかを仕様で決め、前回sessionの波形を現在の状態として表示しない。永続dataを持つdeviceは、その保持・消去条件も明示する。

必要機能: loaderと実行制御 F09、観測 F10。

## 6 マシンを組み替えてもう一度動かす

例えばgpu0のregister先頭addressを変更する。実行を停止し、構成を検証し、SDKを新しい世代へ更新する。エディタの表示も更新してからアプリを再compile/linkし、新しい構成のruntimeと組み合わせる。MMIO addressがguest codeへ埋め込まれる場合は、再linkだけでは足りない。

| 変更 | 必要な更新 | アプリsourceへの影響 |
| --- | --- | --- |
| チップの座標や表示名 | layoutだけ | なし |
| gpu0のMMIO配置 | machine、SDK、必要なcompile/link、runtimeの構成 | instance IDとAPIが同じなら原則なし |
| `gpu0`を`display0`へ改名 | SDK、import参照、build、runtimeの構成 | import path等への影響を事前表示 |
| device仕様や設定の変更 | 適合性再確認、SDKと両成果物 | APIや容量制約により変更が必要 |
| 同じ仕様のhost実装へ交換 | hostの解決記録とruntime | なし。guest成果物を再利用 |
| 配布先OSの変更 | 対応hostの解決とruntime、配布物 | 同じmachineならなし |

deviceを削除して必要APIがなくなった場合は、importエラーと原因のdeviceを示す。旧machine向けの成功成果物を残しても、新構成用と誤って起動しない。undoは論理構成を戻し、その世代のSDKと成果物が使えるか再検査する。

必要機能: F03〜F11をまたぐ世代管理、変更診断、cache制御。

## 7 作品を配る

まずWindows向けに、アプリとruntimeをまとめて配る。画像や音声を外部ファイルにする段階では、入力形式、変換条件、asset ID、配置とハッシュを定義したasset処理を通す。

```console
ond kagura-machine export --app ./out/demo-app --runtime ./out/runtime-windows -o ./out/demo-windows
```

受取側は開発ツールを入れず、同梱されたlauncherから起動する。相対pathだけで必要な資源を見つけ、アプリを別runtimeへ渡せる形も保つ。後続版では`build-runtime`の配布先をmacOSやLinuxへ変え、元の`demo-app`をそのまま同梱する。

通常のexportにも必要な仕様本文と閲覧用の索引を含める。受取側は作品に付いた「マシンの仕様」から、構成と各deviceの仕様をオフラインで読める。製品全体を同梱しない場合も、規範本文、正確なrevision、出典、配布条件を保持する。仕様を中央サイトや開発者のcacheだけへ残さない。

必要機能: asset処理 F12、配布 F13、複数OSの実装と検証 F14。

## 8 保存物から別実装へ移す

「保存用にexport」で、通常配布に必須のアプリ、machine、規範仕様本文、boot資源、assetsに、適合試験、設計理由、変更履歴、由来等を追加する。通常配布との差は追加資料の範囲であり、仕様本文の有無ではない。ネットワークを切り、元のnative runtimeがない状態でも、保存された契約を読み、適合する別実装を登録して実行できることを確認する。

保存物の一部が欠落した場合は、別の作品に同梱された仕様をID・revision・digestで照合して補えるようにする。異なる本文をコピー数だけで採用せず、競合と来歴を残す。最初の実演は「作者が部品を渡す、利用者が作品を配る、配布元消失後も受取側の通常配布物から仕様を読んで別実装で動かす」という連鎖とする。

必要機能: 独立実装による検証 F01、device実装の登録 F15、長期保存 F16。

## デバイスを作る側のフロー

部品を追加する開発者は、規範仕様と試験を先に用意し、論理状態・時間・faultを定義する。一つ以上のhost実装を作り、同じ試験をadapter経由で適用する。仕様一式と実装一つで最小製品を包装でき、guest SDKや試験記録は追加できる。SDKがない場合はOnd用APIの未提供を表示し、宣言的bindingからの生成や別途のguest SDKを用意する工程が必要になる。

製品の形式上の成立と適合検証済みは別に扱う。公式の標準deviceでは、仕様からの第二実装で説明の不足を検証する既存の品質条件を維持するが、第二実装を全製品に同梱する必要はない。

登録では、仕様書、適合試験、guest SDK、host実装、観測schemaを役割ごとに識別する。作者はmanifestと内容目録を検査して一つの製品fileに包装し、受取側はbinaryをloadせず仕様と対応環境を確認してローカル登録する。実行の有効化は信頼・隔離方針に従う別の操作とする。最初は同梱部品、後にこの外部製品の受渡しへ広げる。公開registryや任意のUI pluginは必須にしない。UIはregister表、memory、画像surface、sample streamという標準viewを観測schemaから選ぶ。

## フローから得た機能一覧

「初出」は必要最小限が使える版であり、後続版での拡張は[バージョン計画](machine-roadmap.md)に記す。

| ID | 機能 | 必要になる操作 | 初出の案 |
| --- | --- | --- | --- |
| F01 | 規範仕様、要件ID、適合試験、独立実装試験 | deviceを定義し、復元性を確かめる | 0.2.0 |
| F02 | deviceカタログと仕様・実装の別解決 | 部品を選ぶ | 0.2.0 |
| F03 | machine定義、instance、論理lock、世代の識別 | 構成を保存する | 0.2.0 |
| F04 | memory map、windowとboot領域の検証 | 接続する | 0.2.0 |
| F05 | guest SDK生成と世代固定 | APIを使う | 0.2.0 |
| F06 | アプリ雛形、machine関連付け、compilerとLSPの同期 | 書き始める、組み替える | 0.2.0 |
| F07 | 独立したアプリbuild、契約metadata、cache | アプリを保存する | 0.2.0 |
| F08 | host解決記録、独立したruntime生成 | 実行環境を用意する | 0.2.0 |
| F09 | loader、互換性検査、実行・停止・fault表示 | 実行する | 0.2.0 |
| F10 | 副作用のないsnapshot、購読、標準view | 内部を見る | 0.2.0のsnapshot、0.3.0のlive view |
| F11 | ボード、部品棚、接続編集、undo、変更診断 | GUIで組む | 0.3.0 |
| F12 | asset変換とID、設定・内容の固定 | 画像や音を作品へ入れる | 0.4.0 |
| F13 | 個別成果物からの包装、launcher、受取側の起動 | 他人へ渡す | 0.4.0 |
| F14 | 対応host別artifact、自動選択、OS間の適合試験 | 別OSへ運ぶ | 0.5.0 |
| F15 | 外部device製品の包装・受渡し・ローカル登録と適合根拠 | 部品を共有し、別実装へ交換する | 0.6.0。同梱の試験用交換は0.2.0 |
| F16 | 必須仕様本文の同梱、拡張保存物、分散資料の照合と復元試験 | 通常配布から後世に残す | 本文同梱と依存検査は0.2.0、追加資料と復元は0.6.0 |
| F17 | Enbu構成と包装adapter | Enbuを通常のmachineとして使う | 0.6.0 |
| F18 | version互換表、明示的移行、既存保存物の回帰試験 | 更新しながら使い続ける | 0.2.0から整備し、1.0.0で保証範囲を確立 |
| F19 | device製品形式、目録、仕様・実装の識別、配布条件、内容検査 | 部品を製品として保存・閲覧する | 0.2.0。同梱品から始め、一般の共有操作は0.6.0 |
| F20 | host接続profile、instanceとwindowの仲介、時間・buffer・寿命・serviceの管理 | device binaryを接続し、同じ実装を複数載せる | 0.2.0に公式の最小実装で試し、0.6.0に外部製品へ展開 |

## 既存実装との接続で解決する事項

現行CLIには追加libraryを渡す`--library-dir`があり、LSPは起動時の`OND_PATH`を読む。一方でLSPのwatched-file処理は`.ond`配下の通知を除外するため、生成SDKを置くだけでは再生成後の診断更新を保証できない。F06は探索先の共有と、SDK世代の変更を確実に反映する仕組みまでを含む。参照: [LSPの起動と変更通知](../../crates/language-server/src/main.rs)。

SDKの永続的なpreview、appとmachineの関連付け、lockの分離、成果物wrapperは今後の設計対象である。既存の言語仕様、ABI、Object／LinkedImage、公開APIに変更が必要になった場合は、具体的な差分と移行案を設計判断として扱う。隣接するKaguraやVS Code拡張、Enbu側の実装が必要な項目は、別repositoryの作業として依存関係を記録する。
