# ビルドキャッシュ契約
<!-- ond-spec-id: build/cache -->

## 成果物とキャッシュ
<!-- ond-spec-id: build/artifacts-and-cache -->

PackageArtifactとmanifestのfieldは[成果物schema](../artifacts.md#packageartifactとcompilationmanifest)に従う。

build_idは次を正規順にJSON化したSHA-256:

- contractと実行コンパイラの識別子
- ond.toml全内容のSHA-256（設定変更は保守的に全パッケージを無効化）
- パッケージの論理パス、ソース相対パスと各内容のSHA-256
- 直接import先のbuild_id

generic宣言があるprojectでは、owner packageが生成する具体化済みfunction/static symbol、利用される型構造、生成定数の値、公開interfaceもbuild_idへ含める。\
`sizeof(T)`や`alignof(T)`の評価後も、function・initializer・staticに含まれる確定定数値によって評価結果の変化を検出する。\
型構造はfield・method・underlying typeとnominal identityを含めて正規化し、内部の採番やsource位置が変化しても同じ構造に同じ識別値を与える。\
呼出元が新しい特殊化を要求した場合はowner packageのbuild_idが変わり、既存要求だけならowner packageを再利用する。

project全体のsource相対pathと内容hashは、contract・compiler・manifest設定・import graphとともにprivate project manifest keyへ含める。\
全artifactの参照先、checksum、package集合、依存関係を検証できた場合だけfrontendを省略する。\
source状態が変われば型付きfrontendで具体化集合、型構造、生成定数の値を再確認し、package build_idが変わったObjectを生成する。\
compiler_idは実行ファイル全体のSHA-256。\
CLIの再ビルドで変化すれば再コンパイルする。\
内容の同一性はhashで判定する。

成果物は `<build_id>-<content_hash>.ondpkg`、\
package索引は `<build_id>.json`、project索引は `project-<key>.ondmanifest`。\
content_hashは成果物bytesのSHA-256。\
一時ファイルを同じディレクトリに作成し、書込完了後renameで公開する。\
壊れた索引・checksum不一致・不適合なキャッシュはmissとして再生成する。\
明示的なlink入力の破損または不一致はlink errorとなる。\
compile失敗はerrorを返し、以前の成功成果物を保持する。\
古いcache keyの回収時点は実装裁量とする。

project内の収集済みpackage全体を検査・Object生成の対象とする。\
startupに含めるinitializerはrootから到達するパッケージだけ。\
import順・パッケージ内のソース順を保持し、ファイル名の追加・削除・renameもキーへ反映する。

revision/target/ABIの受理条件は[版互換表](../artifacts.md#版識別と互換条件)に従う。

debug sidecarのrange、fault分類、unwind規則は[Ond Debug Metadata v1](../debug-metadata/v1.md)に従う。

## 検証
<!-- ond-spec-id: build/verification -->

再利用の前提はsource内容、manifest、compiler_id、contract、import graph、依存build_idとartifact checksumの一致である。\
source読込、構文解析、依存graph収集はcompileごとに行う。\
全package artifactの一致を検証できた場合は型付きfrontendとObject生成を省略できる。\
一つでも不一致ならproject全体に型付きfrontendを実行し、Object生成は再計算したpackage build_idの変化に応じて行う。\
`frontend_ran`は型付きfrontendを実行したことを示す。

検査はcold/warm、diamond依存、実装だけの変更、generic具体化の追加、sourceの追加・削除・rename、manifest/compiler変更、仮想sourceとの衝突を入力として再利用範囲を照合する。\
壊れたcacheからの再生成、明示link入力の破損error、fresh linkとのimage一致、診断位置、既存出力の保持も外部の観測結果である。

## process内意味解析キャッシュ
<!-- ond-spec-id: build/process-semantic-cache -->

同一process内の解析結果再利用に関する既存IDを保持する。\
再利用・失効・保持の規則を次の節にまとめる。

### 意味解析cache
<!-- ond-spec-id: build/semantic-cache -->

意味解析結果を再利用するキャッシュは、packageごとに通常sourceとtest sourceを別の単位として扱う。\
ここでの再利用は同一processまたは解析session内の結果に適用する。\
Object生成の永続キャッシュは[パッケージ成果物・独立コンパイラ契約](./packages.md)に従う。

### 参照環境
<!-- ond-spec-id: build/semantic-cache-visibility -->

通常sourceの意味解析は、自packageとimportした依存packageの通常sourceだけを参照する。\
test sourceの意味解析は、自packageの通常sourceとtest source、およびimportした依存packageの通常sourceを参照環境とする。\
名前の公開範囲、同一packageのprivate declarationへのアクセス、test sourceの判定は[テストビルド仕様](./test-build.md)に従う。

test sourceから通常functionを呼んでも、そのfunctionの定義とbodyは通常sourceの参照環境を維持する。\
通常sourceに定義されたgeneric function、method、operatorの特殊化にも、定義元の参照環境を適用する。\
型、constant、globalの解決にも同じ規則を使う。

通常sourceの型付きbodyと名前解決の検証結果は、test sourceの検証結果から独立して再利用できるものとする。\
test sourceだけの変更では、不変な通常sourceと通常依存関係の解析結果を引き続き再利用できる。

### 同一性と失効
<!-- ond-spec-id: build/semantic-cache-invalidation -->

再利用の判定にはsourceの内容とidentity、および参照する通常sourceの依存関係を用いる。\
同じ内容と参照環境を再利用条件とする。\
依存関係には型、constant、generic定義と同一packageのprivate declarationを含め、それらすべての内容と参照環境の一致を再利用条件とする。

test sourceだけが変わった場合、対象packageのtest解析結果を失効させる。\
通常sourceと通常依存関係が不変なら、そのpackageと他packageの通常解析結果を再利用できる。\
他packageのtest解析結果は、参照環境が変わらない限り失効させない。

通常sourceが変わった場合、そのpackageの通常解析結果とtest解析結果、および変更された通常sourceを参照する解析結果を失効させる。\
importの追加・削除、fileの追加・削除・renameでも、変更後の参照環境に基づいて再利用を判定する。

source列挙順や解析snapshotが変化した後も、再利用した診断のprimary labelと関連labelを現在のsourceへ対応づける。\
失敗した解析結果も同じ失効規則を適用し、修正後は現在のsourceに基づく結果と診断を返す。\
snapshot内の内部IDとdata structureの管理方式はRust sourceで管理する。

### 保持期間
<!-- ond-spec-id: build/semantic-cache-lifetime -->

意味解析キャッシュの保持期間は、それを所有するprocessまたは解析sessionの範囲内とする。\
キャッシュを破棄した場合も、再解析によって同じ意味解析結果を得られなければならない。\
LSPでは連続する編集の検証間でpackageごとの通常解析結果とtest解析結果を再利用する。\
再利用と再解析は同じ可視性、診断、生成物の意味を与える。
