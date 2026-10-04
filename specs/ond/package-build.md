# パッケージ成果物・独立コンパイラ契約

## 責務

- compiler-core: source → 型付きHIR → target-neutral MIR。MIRは内部APIであり保存形式にしない。
- compiler: MIR → Kagura Object、runtime helper供給、リンカ。
- interface: target-neutralな公開型・値・関数情報。成果物ローカル型番号を使い、AST/HIR/MIRには依存しない。
- protocol: Object、LinkedImage、成果物のデータ定義だけ。compiler/coreには依存しない。
- cli（ond-cli）: `ond compile`／`ond link`、仮想ソース、依存グラフ、パッケージキャッシュ。
- Enbu SDK: アセットのOndソース生成、Enbuのメモリ配置、boot/loader、ROM構築。

Enbu標準ビルドは独立Ondコマンドを使用する。SDKの実行用Rust依存はprotocolのみ。
compiler/core依存は既存のin-process回帰テスト用dev-dependenciesに限定する。
`ond build`／`ond assets`は復活させない。

## 実装済みの単位と制約

1パッケージにつき1個の `.ondpkg`（UTF-8 JSON）を生成する。全パッケージを
1つのObjectBundleに格納しない。コンパイル結果のmanifestが各ファイルを参照し、
リンク時に結合する。セクションのdataだけBase64、それ以外は通常のJSON。

現段階の差分単位は**Object生成**。ソース読込／構文解析／依存グラフ構築は毎回行う。
全パッケージがキャッシュ一致なら型検査・HIR/MIR生成・Object生成を省略する。
1つでも不一致なら型検査・HIR/MIR生成は全プロジェクトへ実行し、Object生成だけを
不一致パッケージに限定する。`frontend_ran`はこの型付きフロントエンドの実行を示す。

公開型・関数シグネチャ・定数値・global型を含むexport dataを保存・読出できる。
依存パッケージのソースは引き続き必要であり、「依存成果物だけで単一パッケージを
型検査する」完全なパッケージ単位コンパイルには未対応。
次工程はexport dataからcoreの型・名前解決環境を構築するimport口の追加である。
成果物ローカル型IDと名前付き型のidentityは下記の形式で定義済み。
project-global TypeIdをそのまま永続化しない。

## コマンド

`cargo build -p ond-cli --bin ond` で `target/debug/ond.exe` を生成する。
要求・応答JSONラッパーは廃止。操作は引数、入力はファイル、stdinは入力ファイルの代替にする。
成果物の内部形式は引き続きJSON/Base64だが、呼出側が操作用JSONを作る必要はない。

```sh
ond compile .
ond compile ./game --library-dir ./generated -o game.ondbuild
ond link game.ondbuild --ram 0x10002000:0x03FFE000 --stack 1MiB --return-to 0x10000000 -o game.ondimage
ond compile . -o - | ond link - --ram 0x10002000:0x03FFE000 --stack 1MiB --return-to 0x10000000 -o game.ondimage
```

### ond compile

`ond compile [project] [-o FILE|-] [--library-dir DIR]... [--cache-dir DIR] [-v]`

- project省略時はカレントディレクトリ。ond.tomlのあるprojectをコンパイルする。
- ond.tomlの`[dependencies]`のkeyはmanifest内の登録名であり、import pathには使用しない。
  依存先ond.tomlの`[project].name`を論理import pathの先頭へ割り当てる。この名前はOndの
  identifierでなければならず、依存先では必須とする。これにより利用側が登録名を変更しても、
  依存先自身のimport pathは変化しない。依存root直下のpackageはproject名、子directoryは
  `project名/相対directory`でimportする。
- 相対pathはond.tomlのdirectory基準、絶対pathも許可する。解決先directoryにはond.tomlが必須である。
- path依存は`name = { path = "../library" }`、Git依存は
  `name = { git = "https://github.com/owner/repository.git" }`で宣言する。
  文字列だけを指定した場合はURL形式をGit、それ以外をpathとして扱う。Gitの`rev`も指定できる。
- Git checkoutは`project/.ond/dependencies`へ保存し、解決したcommitは`ond.lock`へ記録する。
  lockがあれば同じcommitを使用し、更新はlockを削除して再解決する。`.ond/`は常にsource探索外とする。
- `[resolve]`の`ignore = [".git", ".ond", "target"]`で、manifest directoryからの相対directoryを
  再帰探索から除外する。省略時はこの3件を既定値とする。絶対path、`..`、globは許可しない。
  依存先には依存先自身のignore設定を適用する。子directoryに別のond.tomlがある場合は別project境界として探索しない。
- 依存先のond.tomlも再帰的に解決する。同じproject名が異なるdirectoryを指す場合はエラーとする。
  同じdirectoryを異なる登録名で参照しても同じ依存として扱う。symlinkは初期実装では拒否する。
- 出力省略時はproject/target/build.ondbuild。出力内容はCompilationManifestそのもの。
- パッケージ別.ondpkgはproject/target/ond-cacheへ保存する。変更する場合だけ--cache-dirを指定。
- 相対引数パスは全て呼出時のカレントディレクトリ基準。既定cache/outputだけproject基準。
- --library-dirは追加パッケージの探索ルートで、複数指定できる。
  例えばDIR/enbu/assets/ids.ondがenbu/assetsパッケージになる。
- OND_PATH環境変数でも追加パッケージの探索ルートを指定できる。複数ルートは
  OS標準のpath-list区切り（Windowsは`;`、Unix系は`:`）で並べる。
  --library-dirとOND_PATHを同時に指定した場合は両方を使用し、同名パッケージには
  下記と同じ衝突検査を適用する。
- ライブラリroot直下の.ond、symlinkは拒否する。project root自体をlibrary rootに指定できない。
  project内のlibraryディレクトリは通常のproject package収集から外し、任意libraryとして扱う。
- importで到達する任意パッケージのみ採用。文字列escapeをデコードして依存を追跡する。
- 同名パッケージをprojectや複数library rootで見つけた場合は、
  ソース相対名の集合と内容が全て一致する場合のみ受理する。暗黙に混合・上書きしない。
- --cache-dir以外のcache操作やtarget/ABI/revision指定は要求しない。
- -vは再生成／再利用したパッケージ名をstderrに表示する。manifestへ統計を混ぜない。
- compileのstdinは使用しない。sourceをstdinから単独コンパイルするモードは提供しない。

### ond link

`ond link FILE|- --ram BASE:SIZE --stack SIZE --return-to ADDRESS [--read-only BASE:SIZE] [-o FILE|-]`

- FILEはcompileのmanifest。-ならstdinから読む。
- 標準出力へ出す場合の出力はLinkedImageそのもの。fileへ出す場合はLinkedImageと、拡張子を`.onddebug`へ置換したdebug sidecarを生成する。省略時はカレントディレクトリの`a.ondimage`と`a.onddebug`。
- LinkedImageとdebug sidecarは同じ`build_id`を持つ。`build_id`は空の`build_id`を持つLinkedImageのcanonical JSON bytesのSHA-256である。
- `-o -`ではpipe互換性のためLinkedImageだけをstdoutへ書き、sidecarは生成しない。
- --ramはRAM領域、--stackはその内側に予約するstack量、--return-toはmain正常終了後の絶対ジャンプ先。
  これらは機種SDKが決めるため必須。Enbu固有の値をOndの既定値にしない。
- --read-onlyを指定するとText/Rodataを別領域へ配置する。boot-bank用profileでも使用する。
- 数値は10進または0x付き16進。KiB/MiB suffixを受理し、u32範囲外を拒否する。
- entry/heap/stackの内部symbol名はOnd規約から設定し、引数には露出しない。
- contract/target/ABI/依存関係/初期化順はmanifestとパッケージ成果物から読出・検証する。
- compilerは共通startupを作り、要求されたruntime helperを一度だけ供給してリンクする。

### 入出力と失敗時

-o -はstdout。通常ファイル出力は同一ディレクトリの一時ファイルからrenameで公開する。
compile/linkが失敗しても指定先の以前の成果物を上書きしない。
ログ・診断はstderrのみ。成功exit 0、コンパイル／リンク／ファイルIO失敗exit 1、
不正なコマンド／引数exit 2。失敗時のJSON応答は返さない。
引数なし、--help/-h、ond helpは使用法をstdoutに表示して正常終了する。
--以降は位置引数として扱う。

SDKは一意な専用一時ディレクトリへ生成ソースを書き、--library-dirで渡す。
利用者のソースには書き込まない。コンパイル後に一時ディレクトリを削除する。
SDKはcompileのstdoutからmanifestを受け取り、そのmanifestだけをlinkのstdinに渡す。

## 成果物とキャッシュ

PackageArtifactのフィールドはcontract、compiler_id、package、build_id、
dependencies（パッケージpath→build_id）、imports（ソース順）、
initializer（省略ではなくnull可）、interface、object、link前debug metadata。

build_idは次を正規順にJSON化したSHA-256:

- contractと実行コンパイラの識別子
- ond.toml全内容のSHA-256（設定変更は保守的に全パッケージを無効化）
- パッケージの論理パス、ソース相対パスと各内容のSHA-256
- 直接import先のbuild_id

compiler_idは実行ファイル全体のSHA-256。CLIの再ビルドで変化すれば再コンパイルする。
依存先の本体だけの変更でもbuild_idが変わり、推移的な依存元を全て再生成する。
依存しないパッケージのObjectは再利用する。mtimeには依存しない。

成果物は `<build_id>-<content_hash>.ondpkg`、
キャッシュ索引は `<build_id>.json`。content_hashは成果物bytesのSHA-256。
一時ファイルを同じディレクトリに作成し、書込完了後renameで公開する。
壊れた索引・checksum不一致・不適合なキャッシュはmissとして再生成する。
明示的なlink入力の破損／不一致はエラーであり、暗黙に再コンパイルしない。
コンパイル失敗時は成功manifestを返さず、以前の成果物は削除しない。
古いキーの回収・キャッシュGCは未実装。

既存coreの意味を保つため、rootから到達しないパッケージも検査・Object生成する。
startupに含めるinitializerはrootから到達するパッケージだけ。
import順・パッケージ内のソース順を保持し、ファイル名の追加・削除・renameもキーへ反映する。

revision/target/ABI不一致は拒否する。このrevisionはホスト側の成果物／通信形式のもので、
Enbuカートリッジヘッダにversionを追加するものではない。

debug sidecarのrange、fault分類、unwind規則は[Ond Debug Metadata v1](./debug-metadata.md)に従う。

## 検証

driverの単体／プロセステストで次を検証する:

- cold/warm、diamond依存、実装だけの変更と推移的再生成、非依存Object再利用
- ソース追加・削除・rename、manifest／compiler変更、仮想ソースと衝突
- 壊れたキャッシュからの復旧、診断位置、失敗後の既存成果物保持、循環import拒否
- startup初期化順、runtime helperを含む既存全量コード生成とのLinkedImage一致
- 混在／不足した依存成果物、compiler/revision不一致、startup metadata不一致の拒否
- ファイルと実際のパイプでcompile → link、終了コード、Base64セクション、異常時のstdout分離・既存出力保持

## 公開情報（revision 3）

`interface`はpackage、types（型定義の配列）、exports（公開名→宣言）からなる。
Objectとは独立したtarget-neutralデータで、本文・機械レイアウト・一時的なTypeId/PackageId/Spanを含めない。

generic宣言templateはこの成果物へ保存しない。generic型・関数・method・operatorを利用するpackageはsource宣言から具体化する。具体化済みの型と関数は、通常の非generic成果物と同じ形式で扱う。

- exports: Type、Constant、Global、Function。小文字のbindingは公開しない。
- types: primitive、Pointer、Array、Function、Struct、Interface、Defined。
  参照番号はこの成果物のtypes配列内のindexのみ。名前付き型の完全修飾名をnominal identityとして保存する。
- 公開宣言から到達する型だけを再帰的に保存する。非公開の名前付き型も公開関数の戻り値などから到達する場合は含むが、
  exportsには追加しない。recursive pointerの循環参照を許す。
- struct fieldには名前、型、private_owner（論理パッケージpathまたはnull）を持たせる。
  非公開フィールドの型同一性・アクセス制限に必要な所有情報を失わない。
- Interface型は宣言順のmethod名・source-visible signature・private_ownerを持つ。Ond v1のinterface methodは常に公開のため、interface methodのprivate_ownerは常にnullとする。concrete named typeには、到達可能な公開methodの名前・receiverがpointerか・source-visible signature・link symbolを持たせる。receiver自身はsignatureのparameter列へ含めない。
- 関数は引数型列と戻り値型列。引数名は型同一性に関与しないので保存しない。
- Constantは確定型と評価済み値。Integerはu64 bit値、Float32はu32 IEEE bit値を保持し、
  JSON浮動小数点へ変換しない。Bool、typed Nil、Zero、Composite（index/field→値）にも対応する。
  Compositeは省略された要素をzeroとする。式・参照先のソース・評価コードは保存しない。

読出時には型参照の範囲、公開binding名、nominal名の重複、aggregate indexの重複、
成果物とinterfaceのpackage一致を検証する。これは参照整合性の検査であり、
まだ型検査環境へimportして意味検査を行う実装ではない。

revision 1と2は受理しない。キャッシュはcontract/compiler識別子の変化でmissとなる。
旧キャッシュの自動削除はしない。
