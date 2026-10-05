# デバイス製品のパッケージと仕様継承の設計案

状態: 検討用の非規範設計案。作成日: 2026年10月4日。

deviceを、仕様書と実装を一緒に渡せる一つの製品として扱う。最小の製品は、独立に実装できるデバイス仕様一式と、少なくとも一つのhost向け実装で成立する。複数OS版、ソースコード、guest SDK、第二実装の同梱は、この最小構成の条件にしない。

仕様書は開発者だけが保管する付録ではない。deviceを組み込んだ通常のアプリ成果物、runtime成果物、一括配布物にも必要な仕様本文を残す。作品の流通が、その作品を動かすコンピュータの設計資料の分散保存にもなることを目指す。作者や中央サービスが失われた後も、各地の保存物から設計とその系譜を読み直せる、民俗学的な資料として扱う。

本書は[全体設計](machine-experience.md)のDevice ContractとHost Implementationを包装する提案である。形式名、拡張子、キー、コマンドは未実装であり、既存のOnd言語、ABI、Object／LinkedImage、公開APIを変更しない。製品としての構成と必須条件を先に定め、厳密なschemaとbinary接続方式は0.2.0の詳細設計で承認する。

配布されたbinaryを実際に接続する責務、組立計画、instance・window・時間の管理、組込方式の候補は[デバイスの組立境界](device-assembly.md)で扱う。製品形式が有効でも、現在のrunnerへ接続できるとは限らない。

## 製品と仕様と実装を区別する

| 対象 | 意味 | 例と識別 |
| --- | --- | --- |
| デバイス製品 | 名前、作者、仕様、対応実装を持つ配布単位 | `example/common-graphic`の製品releaseと内容digest |
| デバイス仕様 | guestから見える動作を定める契約 | 名前空間付き仕様ID、revision、本文集合のdigest |
| host実装 | その契約をある実行環境で実現するもの | 実装ID、実装version、対応仕様、host条件、内容digest |
| デバイスinstance | ボードに載せた一個の部品 | `gpu0`、`gpu1`。製品を二個載せても別製品にはならない |

製品名は利用者が探すための名前であり、実行互換性の証明ではない。異なる作者の製品が同じ仕様を実装してよく、Windows版だけの製品から始めてもよい。第三者がmacOS版を作る場合も、元の作者の製品を上書きせず、同じ仕様を参照する別製品として共有できる。

論理machineは仕様ID・revision・digest・設定を固定する。製品の選択記録とhost実装の解決記録は別に保持し、同じ契約への適合根拠がある実装への交換だけではアプリを再compileしない。候補選択と信頼判断を経ずに、同じ仕様名を名乗る製品へ自動で置き換えない。

## 最小構成と追加資料

最小構成の実体は仕様と実装の二つである。これをツールと後世の読者が解釈するため、目録、識別情報、配布条件、内容検査の情報を包装部分として必須にする。

| 構成 | 必須条件 |
| --- | --- |
| 製品manifest | 製品ID、release、表示名、作者の表記、仕様と実装の参照、形式version |
| 規範仕様一式 | 本文、規範schema、必要な図表・固定データ、依存する規範資料の本文。URLだけでは不可 |
| 一つ以上のhost実装 | 対応仕様を明示した実装binaryと必要資源。sourceやダウンロードURLだけでは最小製品としない |
| 実装の接続情報 | OS、CPU、最小実行環境、runtime接続profile、入口、必要なhost依存 |
| 配布条件と由来 | 仕様・実装等それぞれの作者、権利表記、license本文、転載元や派生元があればその記録 |
| 内容目録 | 同梱ファイルの相対path、役割、byte数、digest。規範資料と参考資料を識別可能にする |

仕様は人間が読めるUTF-8の本文を基本とし、規範言語と文書間の優先関係を示す。画像だけの説明、オンライン文書へのリンク、実装ソースだけを規範本文の代わりにしない。画像や数式の表現に別形式を使う場合も、解釈方法と必要資料を同梱する。翻訳を追加するときは、規範本文との関係を明示する。

ソース、再build手順、適合試験とtest vector、独立実装の検証報告、guest SDK、sample、設計理由、変更履歴、写真やボードの外観は追加できる。製品の由来を伝える資料として価値があるが、未添付のものを存在するように表示しない。動作を決めるために必須の表やデータは、追加資料ではなく規範仕様へ含める。

形式上有効な製品であること、あるhostで起動を検証したこと、適合試験に通ったこと、仕様から独立した第二実装を作れたことは別の状態として表示する。試験記録には対象仕様・実装・試験のdigest、実行環境、結果、作成者を記す。公式の最小deviceに対する独立実装のリリース条件は維持するが、第二実装そのものを全製品の同梱条件にしない。

guest SDKがない製品でも仕様は読める。Ondから便利に使うには、共通生成器が理解できる宣言的binding情報、または別途固定したguest SDKが必要になる。製品の成立と「Ond APIをすぐ生成できる」という対応状態を分ける。仕様の閲覧やSDK生成のために同梱host binaryをloadしない。

製品から仕様indexと規範依存だけを取り出し、独立した仕様資料として保存・受渡しできるようにする。これは実装を含む最小製品とは呼び分ける。アプリbuild環境はこの仕様資料と必要なguest bindingだけを受け取ればよく、製品全体やhost binaryの取得を要求しない。

## ファイル配置の候補

展開directoryを基本表現とし、そのまま汎用archiveへ包装できる形式を候補とする。以下の`.kdevice`は仮の拡張子であり、既存の`.ondpkg`とは別の役割を持つ。

```text
common-graphic.kdevice
├─ README.txt                         製品と目録の読み方
├─ device.toml                        製品manifest
├─ inventory.json                     同梱ファイルの内容目録
├─ spec/
│  ├─ index.toml                      仕様の識別と規範ファイル一覧
│  ├─ contract.md                     状態、MMIO、時間、fault等の本文
│  └─ dependencies/<spec-digest>/     参照する規範資料の実体
├─ implementations/windows-x64/
│  ├─ implementation.toml            host条件とruntimeへの接続情報
│  ├─ device.bin                     読込形式は接続profileが定義
│  └─ resources/                     実装が必要とする資源があれば同梱
└─ licenses/
   ├─ specification.txt
   └─ implementation.txt
```

`tests/`、`sdk/`、`source/`、`history/`、`signatures/`等を拡張領域として設けられる。必須領域と任意領域を混在させず、未知の必須featureは診断する。`.kdevice`を選択・検査・展開するだけでコードを実行する機構は持たせない。自己展開実行ファイルを製品の唯一の保存表現にしない。

manifestの最初のschemaでは、少なくとも次の情報を表現する。

| 情報の候補 | 内容 |
| --- | --- |
| `format_version` | 製品を読み取る形式の版。Ond本体の版と独立 |
| `product` | 名前空間付きID、release、表示名、説明、作者、派生元の識別 |
| `contract` | 仕様ID、revision、仕様indexの相対pathとdigest、対応profile |
| `implementations` | 同梱実装ごとのID、version、descriptorのpathとdigest |
| `licenses` | 適用対象ごとのlicense本文へのpathと権利表記 |
| `documentation` | 規範本文と参考資料への入口、資料の言語 |

仕様indexは、各規範ファイルの相対path・byte数・digest、規範言語、優先関係、必須の依存仕様のID・revision・digestを持つ。実装descriptorは対応する仕様digest、host条件、接続profileとその仕様参照、読込対象、実装依存と必要権限を持つ。接続profileには静的組み込みも表現できるようにし、動的pluginを形式の前提にしない。

接続に必要なdescriptorの候補項目は、`artifact_kind`、`connection_profile`のID・revision・digest、`factory_entry`、対応する動作契約と能力、`required_services`である。windowの要求と設定制約は動作契約側、絶対addressと論理portの接続はmachine側に置く。自由文やbinaryの名前から接続方法を推測しない。

組む利用者にnative toolchainを要求しない標準経路では、共通runnerが扱えるmodule形式を第一候補とする。静的libraryはrunner発行者によるlink工程を必要とするため、単に製品fileを追加しただけで既成runnerへ組み込めるとは扱わない。収録済みrunnerの選択と、任意の個別binaryを後付けする組立を区別する。具体的なprofileの採用は0.2.0の試作で判断する。

hostの標準library等、配布物へ含めない実行前提は名前と必要version等を明示する。それ以外の実行依存は、完成した製品では必要な実体を同梱する。外部downloadが必要な参照manifestと、そのまま受け渡せる完成製品は区別する。古いhost環境の再現と、新実装によるguest動作の復元は異なる目的である。

## 識別と内容の検査

作者や製品のIDはregistryへの到達性を必要としない文字列とする。IDや作者名だけで本人性を保証しない。表示名、製品release、仕様revision、実装version、内容digestを混同せずに記録する。

内容digestにはalgorithm名を付ける。初期候補はSHA-256とし、保存したfileの生byteを対象にする。読込側が改行や文字の正規化を行ってから照合する方式にしない。仕様index自身を含めない規範file一覧を作り、確定したindexのbyte列から仕様digestを得る。依存仕様のdigestもindexへ含め、元のindexと本文を一緒に保存する。

製品全体の内容digestは、`inventory.json`に列挙したpayloadの目録から計算する案とする。目録自身と、それに付ける署名は自己参照を避けて列挙対象から外す。目録の元byteを保持し、製品manifestと各fileを検査する。archiveの圧縮方法や並び順の違いによるarchive全体のhashと、payloadの同一性は分ける。正確なserialize規則、対象pathの規則、重複項目・循環参照の拒否は正式schemaで固定する。

同じID・revisionで異なる仕様digestが見つかった場合は競合として両方を保持し、コピー数や更新日時で上書きしない。本文の訂正は新しいrevisionとして記録する。別の作者の派生仕様は独自のIDと派生元のID・revision・digestを持つ。実装追加や製品説明の変更だけなら、変更していない仕様indexと本文を保持し、仕様digestを維持する。

digestは保存された内容の同一性と破損検出に使う。作者の本人性、実装の安全性、仕様適合性は別に確認する。署名を追加しても、信頼する鍵の扱いと適合試験を置き換えない。来歴が欠落した資料は、欠落した状態のまま読めるようにする。

## 最終成果物に必ず残すもの

通常配布と保存用出力のどちらにも、必要な規範仕様本文を含める。仕様なしの軽量版を標準の最終成果物として定義しない。適用先は、新たなmachine機能で生成する製品・アプリ・runtime・一括配布物であり、現行の中間Objectや`.ondimage`単体の形式を直接変更する意味ではない。

| 最終成果物 | 必須で残す内容 | 含めなくてよいもの |
| --- | --- | --- |
| device製品 | 当該device仕様と規範依存、少なくとも一つのhost実装、目録と配布条件 | 全OS版、第二実装、source、SDK |
| アプリ成果物 | guest image、必要assets、解決済みmachine定義、実行の解釈に必要な仕様一式 | host用device binary |
| runtime成果物 | runnerと選択device実装、machine資源、対応machine定義と仕様一式、host接続仕様 | 個別アプリのsourceやimage |
| 一括配布物 | アプリとruntimeの各内容、launcher、仕様の閲覧入口 | 未選択hostのdevice binary、開発用GUI |

仕様一式にはdeviceだけでなく、対象のKagura ISA、Ond ABI、bus、memory map、時間、起動、image／asset形式、仕様包装自体の読み方を含める。規範上必要な参照先を辿り、本文・schema・固定データの依存を閉じる。実行に必要なROMやguest libraryも必要資源として保存する。参考URLや連絡先は残せるが、その取得を動作解釈の条件にしない。

規範本文が欠けている、digestが一致しない、再配布条件が未確認といった場合は、該当箇所を示して最終成果物の生成を止める。仕様なしで黙って成功しない。共有向け製品の受入条件には、仕様本文をその製品・派生する作品と一緒に再配布できることを含める。具体的なlicenseの選択は作者が行い、仕様と実装の条件を同一と仮定しない。

同じ仕様を複数deviceが参照する場合、単一配布物内ではdigest単位で一つにまとめてよい。ただし、別々に配れるアプリとruntimeは各々に必要本文を持ち、外のcacheやregistryにだけ置かない。一括配布でも元のアプリ成果物をそのまま取り出せる性質を維持し、包装時の重複排除でアプリ単体の自己完結性を失わない。

runtime生成で静的にdeviceを組み込んでも、仕様と実装の対応、元製品の識別、配布条件を残す。製品を丸ごと同梱しない包装であっても、仕様本文だけを落とすことはない。部品詳細の「仕様書を開く」と配布物内の資料入口は、ネットワークなしで機能する。

## 共有する側と受け取る側のフロー

以下は責務を示すコマンド案であり、現行CLIでは使えない。

```console
ond kagura-device check ./my-device
ond kagura-device pack ./my-device -o ./out/my-device.kdevice
ond kagura-device inspect ./out/my-device.kdevice
ond kagura-device import ./out/my-device.kdevice
```

作者は仕様と実装を用意し、目録・依存・配布条件を検査して製品を作る。適合試験の記録も付けられるが、形式検査だけを「検証済み」の表示に使わない。完成したfileは任意の保存媒体で受け渡せる。公開registry、アカウント、オンライン認証を製品成立の条件にしない。

受取側の`inspect`はコードをloadせず、作者、仕様、対応環境、配布条件、検査記録を表示する。`import`は内容を検査してローカルの部品棚へ登録する操作であり、binaryを有効化する操作とは分ける。実行時には選ばれた実装と必要権限を示し、信頼・隔離方針に従って有効化する。自動解決は、利用者が許可した提供元や実装の集合内で行う。

展開では絶対path、親directoryへの脱出、symlink等による逸脱、重複entryや大小文字によるpath衝突を拒否し、file数と展開量を制限する。仕様表示でscriptや外部resourceを自動実行・取得しない。共有binaryはhost上のコードであり、hash一致や適合試験の成功だけでは安全と扱わない。一般の第三者製品を実行可能にする前に、隔離と信頼モデルを設計する。

作者の改版、第三者による同一仕様の別実装、仕様を変えた派生製品、元製品の単純な複製を区別して部品棚に表示する。公開registryを将来作る場合も発見手段に限定し、唯一の正本や唯一の復元経路にはしない。ファイルの共有機能は、自動uploadや無断のネットワーク配信を意味しない。

## 分散した資料から復元する

この設計でいう「多数決的な復元」は、多数の場所に残った同一資料を照合し、欠落や破損を補うことである。数の多い仕様を自動で正しいものにする規則ではない。少数しか残っていない古いrevisionや地域的な派生も、保存すべき資料として扱う。

1. 保存されたアプリとmachineの固定記録から、必要な仕様ID・revision・digestを調べる。
2. 他の作品やdevice製品から、対応するindexと本文を集める。記録されたdigestと照合し、欠けたfileだけを補う。
3. 同じ名前でも内容が違う資料は併存させ、来歴、訂正記録、試験資料を照合する。根拠が足りなければ未確定と表示する。
4. 揃った規範仕様から実行環境を実装し、適合試験、machine構成、元のアプリとassetsを用いて規定動作を確認する。

元の固定記録や信頼できる内容目録自体が失われた場合、同じfileが多数あるだけでは真正性や元の作品が使ったrevisionは確定できない。複製元が同じかもしれず、コピー数を独立した証拠の数と扱わない。復元判断の根拠と不確かさを残し、元の資料を上書きしない。

通常配布にも規範資料を残し、拡張保存物では設計理由、変更履歴、作者・派生元、適合試験、外観、使用例などを追加する。動作の復元だけでなく、当時なぜそのコンピュータを作ったかも残す。追加資料は任意とし、作者が共有しないsourceや個人情報を自動収集しない。

device仕様だけでは、失われたアプリ固有のコード、画像、音楽を生成できない。保存したguest imageとassetsを動かす環境の復元を基本目標とする。sourceからの再buildや特定のプレイ履歴の再生には、source依存、build条件、初期状態、入力列等を別に残す必要がある。

## バージョン計画への反映

| 版 | この設計で追加する完成条件 |
| --- | --- |
| 0.2.0 | 同梱deviceにも製品manifestを使い、仕様と一つのhost実装を別々に参照できる。一種類のhost接続profileを公式実装で試す。アプリとruntimeに必要仕様本文を同梱し、host binaryなしのアプリbuildを維持する |
| 0.3.0 | ボードの部品詳細から、製品・仕様・実装の違いと同梱仕様書を確認できる |
| 0.4.0 | 通常のWindows配布物にも仕様一式を必須同梱し、開発環境やネットワークなしで読める |
| 0.5.0 | 同じ仕様を持つ複数host実装を解決し、配布物を増やしても仕様の同一性を維持する |
| 0.6.0 | 第三者製品をfileとして受け渡し、検査・登録・明示的な有効化ができる。来歴等を加えた拡張保存物と、複数保存物からの欠落補完・別実装による復元を検証する |
| 1.0.0候補 | 製品形式、仕様目録、同梱規則、互換性と移行手順の保証範囲を確立する |

0.2.0では製品形式の土台を作り、一般の第三者binaryの自動取得・実行や公開共有サイトを完成条件にしない。製品の最小構成と、共有基盤を公開する時期を分ける。

正式な形式化ではarchive方式、path規則、schema、digest対象、依存循環、接続profile、license情報の検証方法を確定する。適合fixtureとして最小製品、複数host、別作者の同一仕様実装、仕様欠落、内容改変、同一IDの競合、不正path、古い形式を用意する。復元試験では中央registryと元runtimeを使わず、保存物を読む段階ではそのbinaryを起動しない。

## 0.2.0詳細設計：製品形式と仕様本文依存閉包

状態: 非規範・未採用の提案。2026年10月5日。対象はF19、F16の本文同梱、F18の識別と形式version記録。schema、拡張子、lock、互換性保証を確定する文書ではない。

### 根拠と今回の境界

本書前半の「最小構成と追加資料」「識別と内容の検査」「最終成果物に必ず残すもの」を具体化する。[ロードマップ](machine-roadmap.md)の0.2.0ゲートBでは、host binaryなしのアプリbuildと、アプリなしのruntime生成、および両成果物のオフライン本文同梱が必要になる。[制作フロー](machine-workflow.md)のF19/F16/F18を検査可能な単位にする。

本書が提案するのは包装と資料検査の契約である。最小machineのRAM・graphics・時間・起動・終了・load・観測の意味、SDK API、接続ABIは別設計とする。[組立境界](device-assembly.md)の三契約を混ぜず、最小machine契約との共同判断は末尾に列挙する。現行Object、LinkedImage、`.ondimage`、`ond.toml`、`ond.lock`、言語仕様、公開API、依存libraryは変更しない。製品loader、schema実装、fixture実体の追加は採用後の別作業とする。

### 入力と成立条件

| 単位 | 必須の実体と情報 | 成立と区別するもの |
| --- | --- | --- |
| 最小device製品 | 製品manifest、当該device仕様と規範依存本文、1つ以上のhost実装binaryと必要資源、実装descriptor、内容目録、対象別配布条件とlicense本文 | 複数OS、SDK、source、第二実装、適合試験合格は最小同梱条件ではない |
| 仕様資料入力 | 仕様index、規範本文・schema・固定データと推移的な規範依存、資料入口、対象別配布条件とlicense本文 | host実装を含まないので製品とは呼ばない。製品manifestの実装参照解決を要求しない |
| アプリbuild入力 | source、解決済み論理machine定義、仕様資料入力、必要なguest binding/SDKとlink情報、必要assets | 仕様資料だけでコードが生成できるとはしない。SDK未提供は別の不足診断。host binaryは不要 |
| runtime生成入力 | 解決済み論理machine定義、仕様資料入力、host別runner・実装・接続profile・必要資源と選択記録 | アプリsource、guest image、entry addressは不要 |

製品から仕様資料を抽出するときはindexの元byteと仕様内相対配置を維持し、製品全体の目録をそのまま流用しない。抽出資料の収録物に対する目録を別に作り、元製品ID・release・digestを由来として保持する。仕様資料を再包装しても仕様digestを維持する。

実装binaryの代わりにsourceやURLだけを置いたものは製品検査を失敗させる。hostの標準library等の明示的実行前提はdescriptorへ記録し、それ以外の実行依存資源は同梱する。閲覧hostと実装hostが違っても資料検査と閲覧はできる。形式有効、host起動確認、適合試験合格、独立実装確認はそれぞれ別に記録し、形式検査だけで「安全」「適合済み」と表示しない。

### 識別と版の提案

以下の語は論理項目名であり、採用済みのserialize keyではない。

| 項目 | 提案する規則 | 理由 |
| --- | --- | --- |
| 仕様ID | 名前空間付き文字列を完全一致で比較。例 `example/graphic`。0.2.0候補文法は小文字ASCIIのsegmentを `/` で2個以上連結、各segmentは `[a-z0-9][a-z0-9-]*` | オフライン識別と表記揺れ防止。registry到達性・作者本人性は要求も保証もしない |
| 仕様revision | 空でないASCIIの `[A-Za-z0-9][A-Za-z0-9._-]*` を候補とし、大文字小文字を区別する不透明な完全一致値 | `v1`等の既存版を振り直さず、大小比較、範囲指定、SemVer互換推定をしない |
| 仕様参照 | ID・revision・algorithm付きdigestの3つを必須とする | 同じ名称の異なる本文を検出し、最新版への暗黙置換を防ぐ |
| path | 仕様の身元ではなく成果物内の所在。IDからfilesystem pathを直接生成しない | IDと配置を分離し、安全な再包装を可能にする |
| 製品release | 製品IDに属する版。実装追加で変えても未変更の仕様indexとdigestは保持 | 製品更新と仕様改訂を分離 |
| 実装version・内容digest | 実装ID、対応仕様参照、host条件、接続profileと共に記録 | 実装交換・再現記録に使い、アプリの論理互換性判定に混ぜない |
| format version | 製品・仕様index・アプリwrapper・runtime wrapper・machine・解決記録で別の形式IDと版を明示 | Ondのrelease番号、仕様revision、接続API versionから独立させる |

既存仕様に対するID割当てとrevision対応は、規範本文の内容変更や既存版の改称を伴わない対応表として別途承認する。上の文法に収まらない既存識別があれば切り詰めや正規化をせず採用判断へ戻す。

0.2.0の読込候補は形式IDとformat versionの明示的対応表方式とし、未知版・未知の必須featureは生成、登録、起動前検査で拒否する。未知の任意metadataは無視できるが元byteを保持する。対応外の保存物も上書きせず、資料を安全な文字列として閲覧できることと、正式な形式検査成功を分ける。移行は元の保存物を残して別出力を作る明示操作とし、移行前後の形式・digestと変換器版を記録する。互換性保証の確立はF18の後続課題であり、本書で全0.x版の互換を約束しない。

### index、目録、digestの提案

仕様indexは自身のID・revision、形式IDと版、規範言語、入口、文書間の優先関係、規範fileのpath・byte数・digest、依存仕様参照を持つ。参考資料と参考URLを規範依存から区別する。本文内リンクだけを走査して依存を推測せず、作者が規範参照をindexに列挙し、本文との対応をレビューする。ツールの閉包検査は宣言された依存についての検査であり、自然言語の参照漏れまで証明しない。

digestの初期候補は `sha256:` と小文字16進64桁。検査順序と対象は次の案とする。

1. 各fileの保存された生byteについてbyte数とSHA-256を照合する。改行、BOM、Unicode、空白を照合前に正規化しない。
2. 仕様indexは自分自身を規範file一覧へ入れない。規範fileのdigestと必須依存のID・revision・digestを含め、確定したindexの生byteのdigestを仕様digestとする。参照側の3つの識別値とindex内部のID・revisionも一致させる。
3. 依存indexを再帰検証する。pathをindexに記すfileはそのindex所在directoryをrootとする。依存の所在は成果物の目録にある「仕様参照 → index path」の対応から解決し、index内にhost絶対pathや外部cacheの所在を書かない。
4. 製品内容digestは製品目録の生byteのdigestとする。目録はmanifest、仕様indexと本文、descriptor、実装binary、資源、license、閲覧入口、任意同梱資料を列挙する。目録自身とそれに付く署名だけを自己参照防止のため除外する。署名を除いた全payloadがちょうど1回収録されていることを検査し、未列挙fileも拒否する。署名の配置と検査方式は採用まで使用しない。
5. archive全体のdigestは輸送時のbyte同一性であり、製品内容digestと別に記録する。圧縮設定・archive entry順の違いだけでは、保存した目録とpayloadのdigestを変えない。

同じ意味でもindexの空白等を変えれば仕様digestが変わる。元byteを保存してコピーし、読込後の再serializeを同一性の基準にしない。新規生成時のfield順・配列順・改行・encodingの規則と既知digest vectorは正式schema採用時に固定する。本書のdigest手順だけで異なるwriter間のbyte再現性を保証したとはしない。

同じID・revisionに異なるdigestがある場合、資料は併存保持し競合と診断する。一つの生成対象の閉包内では競合を拒否し、コピー数・更新日時・探索順で選ばない。異なるrevisionは完全参照が区別できれば資料として併存可能であり、machineとして併用できるかは別の構成検査で判断する。訂正は新revision、派生は別IDと派生元の完全参照を提案する。同じdigestでもID・revisionが食い違う参照を同一仕様として通さない。digestは本人性・安全性・適合性の証明ではない。

### オフライン閉包と出力の検査

生成対象ごとにroot仕様参照集合を明示し、各rootから必須依存を辿った集合を収録する。

| 出力 | rootと資源の選び方 |
| --- | --- |
| device製品 | 提供するdevice動作仕様、および同梱する各実装の接続profileと必須契約。全同梱実装を目録で検査 |
| 仕様資料 | 抽出対象として明示した動作仕様と、その規範依存。実装専用profileは動作の規範依存でない限り不要 |
| アプリwrapper | 解決済みmachineが必要とするISA、Ond ABI、bus、device、map、時間、起動・終了、load/image、使用asset形式とwrapperの読み方。guest image・assetsと必要guest資源を収録 |
| runtime wrapper | 対応machineの契約集合、runner・選択実装が必要とする接続profileとwrapperの読み方。runner・選択実装・boot ROM等のmachine資源を収録 |

machine側から完全参照のroot集合を受け取り、包装側で必須項目が揃ったか検査する。どの契約が必須かを包装側が自由文から推測しない。ROM等は実行資源であり、それを解釈する仕様本文の代用品にはしない。アプリとruntimeのどちらが各資源を持つかはmachine・load契約との共同判断とする。

探索は検査中・検査済みを完全参照単位で記録し、同一依存のdiamondは一度だけ検査する。検査中の参照へのback edgeは循環として拒否する。再帰digestの相互依存を避けるため規範依存をDAGとする案であり、相互に不可分な資料は一つの仕様index内の複数fileにまとめる。参考リンクの循環は規範依存とはしない。

包装形式の読み方も同梱するが、各indexが自分のdigestや自分を含む包装仕様を再帰参照する形にはしない。bootstrap資料は他indexへ依存しない葉の規範資料とし、indexと目録の読み方を人間が読める本文で持たせる。root側からその資料を固定参照する方式を提案する。bootstrap自体のID・revision・収録単位とschemaの規範性は承認対象であり、循環を例外的に無検査で通す方式にはしない。

欠落、改変、ID競合、対応外形式、配布条件未確認のいずれかがあれば最終出力を確定しない。診断には出力種別、rootから失敗参照までの依存経路、期待ID・revision・digest、成果物内path、理由を含める。license本文の存在検査と、作品への再配布条件を作者が確認した記録の検査を分離する。license名だけから転載可能と推定しない。

検査済みbyteを隔離した一時出力へコピーし、出力側のindex・本文・目録を再検査してから確定する。検査とコピーの間の変更を許容せず、失敗時は部分出力を成功扱いにせず以前の成功成果物を保持する。抽出後のアプリとruntimeをそれぞれ別directoryへ移し、cache・元製品・ネットワークを使わず入口から全規範資料を読めることを受入条件とする。単一成果物内は完全参照で重複排除できるが、別々に配る成果物の外に本文を追い出さない。将来の一括配布でもアプリ単体の取り出し後に同じ検査を行う。

### pathと読込の境界案

archive entry、目録path、indexのfile path、descriptor資源pathに共通の保存path検査を使う。本文中のリンク表記とは区別する。成果物目録のpathは成果物root、indexのfile pathは仕様root、descriptor資源pathはdescriptor所在directoryを基準とし、連結した最終所在にも検査を行う。

- 保存pathは `/` 区切りの相対path。空path、空segment、`.`、`..`、先頭 `/`、backslash、drive/UNC、colon、NUL・制御文字を拒否する。decodeしてから通すURL表記を採らず `%` も拒否する。
- 初期の移植性重視案としてfile名segmentをASCIIの英数字・`_`・`-`・`.`に限定し、末尾のdotを拒否する。Windows予約名（拡張子付きも含むCON、PRN、AUX、NUL、COM1〜9、LPT1〜9）をcase-insensitiveで拒否する。人間向けUnicode表示名はmetadataへ置く。この制限と上限値は採用判断が必要。
- 同一pathの重複、ASCII大小文字だけが異なるpath、fileとdirectoryのprefix衝突を、archive展開前とdirectory入力の両方で拒否する。同じ内容でも重複entryは許さない。
- regular fileとdirectoryだけを受け付け、symlink、hardlink entry、junction/reparse point、device file、FIFOを拒否する。directory入力も全path要素で逸脱を拒否し、検査中の差替えで外部fileを読めない方法を実装時に検証する。単なる文字列prefix比較をroot判定に使わない。
- 資源上限の試験用候補はentry数10,000、総非圧縮量1 GiB、単一file 256 MiB、path 240 byte、directory深さ32、仕様数1,024、依存深さ64。圧縮headerの申告値だけでなく実際の読込量にも適用し、整数overflowや上限超過を拒否する。量の妥当性は最小machine資源の実測後に承認する。

本文中のローカルリンクは元byteを保持し、保存pathの文法をそのまま当てない。現行Ond ABIの `./machine.md` やfragment付き参照のように、fragmentを所在から分離し、本文所在directoryを基準に `.` と `..` を解決してから、最終所在が同梱目録に登録された許可範囲内の通常fileであることを検査する。rootを越える参照、絶対path、外部resourceの自動取得は許さない。URIのencodingを用いる場合は一度だけdecodeしてから所在を検査し、OSへ再decodeを委ねない。fragmentは同梱本文内の見出しとして扱い、filesystem pathへ渡さない。既存の相対配置を保てない再包装では、元本文を変更せず参照元fileとリンク表記から完全仕様参照・収録pathへの宣言的な対応表を同梱する案とする。規範上必要なリンクの解決漏れは生成を止め、参考リンクの未解決はその旨を表示する。対応表とviewerの形式も正式schemaの承認対象である。

閲覧、検査、抽出、登録、SDK準備、runtime包装のために同梱binaryをload・実行せず、installer、hook、外部生成器を呼ばない。形式・host情報は宣言から得る。資料表示はscript、外部resource取得、能動的HTML/SVG等を実行しない安全なtext表示を基本とし、外部URLは参照表示に留める。binaryの実行有効化・信頼判断・隔離はF20/F15の別工程とする。

### fixtureと検査条件の提案

以下は採用後に作るfixtureの仕様であり、実装済みのtestではない。`P0`は仮のdevice仕様D、その依存B、1つのhost実装、接続仕様H、bootstrap資料、目録、対象別licenseからなる最小製品。`S0`はD/Bと必須資料を保持する仕様のみ入力、`A0`と`R0`は同じ論理machineのアプリ/runtimeとする。実際の規範本文、ID、digest vector、host profileは承認後に固定する。

| ID | fixtureの作り方 | 期待する検査結果 |
| --- | --- | --- |
| C01 | P0。SDK・source・第二実装・他OS実装を付けない | 製品形式有効、適合性と起動確認は未確認。別OSからも閲覧可能 |
| C02 | P0からhost実装binaryだけ削除、またはURL/sourceへ置換 | 製品不成立。S0は別の入力種別として成立 |
| C03 | S0とmachine、source、必要guest SDKでアプリを生成。host binary置場を空にする | host binary探索・loadなしでA0を生成。SDKも除いた派生例はSDK不足を診断 |
| C04 | machine・仕様・runner・実装だけからR0を生成し、A0/R0を別々に移す | アプリsource/imageなしでR0生成。双方とも外部cache・networkなしで全必須本文を閲覧 |
| C05 | D→B→CのC本文/indexを削除。URLだけ残す | 欠落した完全参照とD→B→Cを診断し最終生成失敗。参考URL未取得だけでは失敗しない |
| C06 | 本文1 byte変更、byte数だけ変更、indexの改行だけ変更、manifest/実装/資源変更を別ケースにする | 対応するfile、仕様、製品のdigest検査が失敗。検査後のコピー元変更も出力再検査で失敗 |
| C07 | 同じID・revisionで異なるdigestの仕様を二つのrootから要求 | 両資料を保持して競合診断、生成失敗。完全参照一致のdiamondは成功し1件に集約 |
| C08 | 期待digestに対応するindex内部のID/revisionと参照側を食い違わせる | digestだけの一致で通さず識別不一致。別revisionの資料併存自体は許可 |
| C09 | index宣言の自己参照・A→B→A、および深さ上限超過を作る | 循環/深さ超過で有限時間内に失敗。hash不一致が先に出ても、graph検査単体で循環経路を確認 |
| C10 | `../x`、`/x`、`C:/x`、UNC、backslash、`a//b`、`a/./b`、`%2e%2e/x`、`a:b`、末尾dot/space、予約名、制御文字を個別投入 | archiveとdirectoryの共通検査で拒否しroot外を読書きしない |
| C11 | 重複entry、`A.md`と`a.md`、file `a`と`a/b`、link経由のroot外参照、特殊file | 拒否。既存出力とroot外fileが不変、部分登録なし |
| C12 | 未列挙payload、規範file一覧の重複、対応表の重複・欠落、宣言サイズ偽装と量の上限超過 | 曖昧な目録/過大入力として失敗。上限ちょうどは他条件が正しければ成功 |
| C13 | license本文欠落、対象対応欠落、再配布条件未確認を個別投入 | 該当対象を示して生成失敗。仕様と実装の条件を代用しない |
| C14 | 未対応形式版・必須feature、未知の任意metadata、旧形式の保存物 | 前二者は正式検査拒否、任意metadataは保持。移行は別出力、元digest不変 |
| C15 | 製品の実装追加/製品release更新だけ行い仕様byteを保持。archive圧縮/entry順だけ変える | 前者は製品digestのみ変化、後者はarchive digestのみ変化し仕様・製品内容digestは不変 |
| C16 | load/起動で印を残す試験用binary、hook宣言、外部画像/scriptを持つ資料を入力 | 読込から包装までload/process起動/外部通信が0回。binaryの印がないことだけでなく呼出し境界を計測 |
| C17 | 元byteの現行Ond ABI/machine本文、`./`、root内の`../`、fragment付きリンク、root外への脱出、percent encodingを含むリンクを収録 | 前三者は同梱先へ安全に解決、脱出は拒否。参照対応表を使う再包装でも本文digest不変、networkなしで必要箇所を読める |

拒否fixture共通で終了状態が失敗であること、成功成果物・登録記録を残さないこと、以前の成功成果物と元資料を保持することを検査する。診断文面完全一致ではなく原因と完全参照・依存経路・pathの有無を検査する。WindowsとLinuxのpath差を同一fixtureで確認する。F19の形式fixture合格とF01の独立実装適合試験合格は別の証拠とする。

### 採用前に判断する事項と後続依存

| 判断 | 推奨案と根拠 | 未解決時に止める依存実装 |
| --- | --- | --- |
| schema・serialize・拡張子・archive | 展開directoryを基本に、人間が読めるmanifest/indexと生byte目録。既存案のTOML/JSON/`.kdevice`は候補のまま。読みやすさとdigest固定の両立を採用時に比較 | parser/writer、正式fixture byte列、製品pack/import |
| ID・revision・digest・path規則 | 本書の完全参照、SHA-256、ASCII path、DAGを候補とする。曖昧な探索とOS差を減らす | resolver、validator、既存仕様のID対応表 |
| bootstrapと規範schema | 自己参照しない葉の読み方資料を同梱。schemaと本文の優先関係も固定 | 仕様目録の正式化、閉包生成 |
| lockと固定記録 | 論理仕様参照集合とhost解決記録を分離。独立buildのためhost側lockをアプリbuild必須にしない。既存`ond.lock`拡張か新設かは未採用 | lock schema、再解決/移行、生成物の世代管理 |
| 最小machineとの共同判断 | ISA/ABI・device・時間・起動/load/観測等の正確なroot参照、mapとの関係、ROM/guest libraryの所在と収録責務、app/runtime wrapper境界 | 実machineのA0/R0、必須契約の完全性検査、loader連携 |
| host接続側との共同判断 | 接続仕様参照、descriptorのbinary種別・入口・必要serviceと静的収録時の表現。製品有効と接続可能を分離 | 実装選択、runtime包装の正式schema。binary接続ABI自体は別設計 |
| 配布条件・上限・移行表 | 対象別licenseと確認記録、最小machine実測での上限承認、形式ごとの対応表 | 配布可能判定、正式上限、旧保存物変換 |

最小machine設計の意味論や具体的数値を本書から確定しない。双方の案が揃った段階で、root参照の受渡し例とA0/R0の資源表を照合し、共同判断を記録する。判断待ちの形式に依存する本実装は進めず、採用された部分からfixtureと検査器を別PRで導入する。設計文書のレビュー可能状態と、0.2.0ゲートBの実装・検証完了を区別する。
