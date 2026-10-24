# 実行可能バイナリと生成入口の契約case
<!-- ond-spec-id: conformance/cases/executable-binaries -->

適用版はOnd Language v1、target `kagura-v1`、ABI `ond-kagura-v1`、ONDX version 1、artifact Contract revision 6とする。\
各caseは独立したfile・memory状態から開始し、同一case内では同じcompiler executableを使用する。\
本書の実行操作は生成物の適合を確認する試験手順であり、利用者のtest実行方法や合否判定を定めるものではない。\
各表の要件IDは[要件索引](../requirements.md#成果物buildtarget)に対応する。

## Wire入力と読出し
<!-- ond-spec-id: conformance/executable-binaries/wire -->

要件は`BIN-01`とし、定義元は[標準実行バイナリ](../../artifacts.md#標準実行バイナリ)とする。\
共有入力Wは[RAMコピーのwire例](../../artifacts.md#ramコピーのwire例)の84 byteであり、全bytesを次のhexで指定する。\
空白と改行はbyte列へ含めず、2桁ごとに1 byteへ復号する。

```text
4f4e4458 01000000 01000000 01000000
54000000 50000010 00010010 02000000
01000000 50000000 04000000 50000010 04000000 04000000
04000000 00000000 00000000 54000010 04000000 04000000
000000c0
```

各行はWの独立したcopyへ操作を行い、読出しの受理またはerrorを比較する。\
u32の変更は指定値を4 byte little-endianへencodeして上書きし、他のbytesを保持する。\
error行ではentryへの制御移行を行わない。

変更が複数の成立条件へ違反する場合は成立条件違反のerrorを比較し、最初に診断する理由や検査順を要求しない。\
表の原因名はその入力が破る成立条件を示す。

| Case ID | 入力・操作 | 期待 |
| --- | --- | --- |
| BIN-01.accept | Wを読む | 受理。headerと2個のrecordがwire例の値と一致 |
| BIN-01.magic | offset 0のbyteを00へ変更 | magic不一致error |
| BIN-01.version | offset 4のu32を2へ変更 | version不一致error |
| BIN-01.target | offset 8のu32を2へ変更 | target不一致error |
| BIN-01.abi | offset 12のu32を2へ変更 | abi不一致error |
| BIN-01.truncated | 最後の1 byteを削除 | image_sizeに対する欠落error |
| BIN-01.trailer | 末尾へ00を1 byte追加 | image_sizeを超える末尾byteのerror |
| BIN-01.table-size | section_countを0xffffffffへ変更 | セクション表長・範囲のerror。u32へ折り返さない |
| BIN-01.kind | offset 56のkindを5へ変更 | 未知kindのerror |
| BIN-01.kind-duplicate | offset 56のkindを1へ変更 | kind重複error |
| BIN-01.kind-order | 2個の24 byte recordの順を交換 | kind昇順違反error |
| BIN-01.empty-section | offset 72のBss memory_sizeを0へ変更 | zero-size recordのerror |
| BIN-01.no-text | offset 32のkindを2へ変更 | 必須Text欠落error |
| BIN-01.file-overlap | offset 36のText file_offsetを32へ変更 | ヘッダ・表との重複error |
| BIN-01.memory-overlap | offset 68のBss memory_addressを0x10000050へ変更 | Textとの実行時範囲重複error |
| BIN-01.text-size | offset 40のText file_sizeだけ3へ変更 | file_sizeとmemory_size不一致error |
| BIN-01.alignment | offset 52のText alignmentを3へ変更 | 正の2冪条件違反error |
| BIN-01.entry-align | entry_addressを0x10000051へ変更 | 命令alignment違反error |
| BIN-01.entry-range | entry_addressを0x10000054へ変更 | Text外entryのerror |
| BIN-01.range-overflow | Bss memory_addressを0xfffffffc、memory_sizeを8へ変更 | address終端overflowのerror |
| BIN-01.bss-file | Bss file_offsetだけ80へ変更 | Bss空格納の表現違反error |

padding用入力Pは、WのText直前へzero byteを4個挿入し、image_size=88、entry_address=0x10000054、Textのfile_offset=84・memory_address=0x10000054、Bssのmemory_address=0x10000058へ変更した全88 byteとする。\
`BIN-01.padding-zero`はPを受理し、`BIN-01.padding-nonzero`はPのoffset 80だけ01へ変更してpadding値のerrorを比較する。

## 配置とロード状態
<!-- ond-spec-id: conformance/executable-binaries/loading -->

要件は`BIN-02`とし、定義元は[配置](../../artifacts.md#配置容量と出力範囲)と[ロード完了状態](../../artifacts.md#ロード完了時の状態)とする。\
ROM省略の共有設定はstartup external、RAM base=0x10000000、size=0x100、stack-size=0x40とする。\
RAM全体はa5で初期化し、保存元はRAM・ROMと分離した外部byte列、CPUのSP初期値は0とする。\
各操作はheader entryへの制御移行の直前で観測する。

| Case ID | 入力・操作 | 期待・比較方法 |
| --- | --- | --- |
| BIN-02.ram-copy | Wを共有設定でロード | RAM[0x10000000,0x10000054)がWとbyte単位一致、続く4 byteがzero、SP=0x10000100、制御移行先=0x10000050、heap_base=0x10000058 |
| BIN-02.padding-reserved | Pを同設定でロード | RAMの先頭88 byteがPと一致、Bssの4 byteがzero、heap_base=0x1000005c。ヘッダ・表・paddingをheap範囲へ含めない |
| BIN-02.rom-xip | Wのentry_addressとText memory_addressだけ0x2050、Bss memory_addressを0x10000000へ変更。ROM base=0x2000・size=0x100を追加し、標準全体をROMへ配置してロード | Textの4 byteはROM[0x2050,0x2054)から参照、RAM先頭4 byteがzero、SP=0x10000100、制御移行先=0x2050。ROM保存元を書き換えない |
| BIN-02.capacity | RAM sizeだけ0x50へ変更し、Wのfile/section占有範囲にRAM配置上限を適用する成立条件を検査 | 配置不成立error、entryへの制御移行なし。CLIのmanifest入力操作を行わない |

Data用入力Dはヘッダをimage_size=110、entry_address=0x10000068、initial_sp=0x10000100、section_count=3とし、magic/version/target/abiはWと同じとする。\
record列は次の全値、offset 104のpayloadは`00 00 00 c0 11 22`とし、ヘッダ・recordは共通wire規則でencodeする。

| kind | file_offset | file_size | memory_address | memory_size | alignment |
| --- | --- | --- | --- | --- | --- |
| Text | 104 | 4 | 0x10000068 | 4 | 4 |
| Data | 108 | 2 | 0x1000006c | 4 | 4 |
| Bss | 0 | 0 | 0x10000070 | 4 | 4 |

`BIN-02.data-tail`はDを共有設定でロードし、先頭110 byteのDとの一致、RAM[0x1000006e,0x10000074)の6 byteのzero、heap_base=0x10000074を比較する。\
`BIN-02.empty-data`はWのBss recordのkindだけDataへ変更した入力をロードし、Wと同じcopy範囲・zero範囲・SPを比較する。\
両caseとも格納bytesとzero末尾を区別して判定する。

## CLIのprojectとpath
<!-- ond-spec-id: conformance/executable-binaries/cli -->

要件は`CLI-01`とし、定義元は[commands](../../build/commands.md)、[compileとlink設定](../../build/packages.md#compileとlink設定)、[manifest path](../../artifacts.md#manifest読出しと参照整合)とする。\
各caseのcwdは`/work/run`、projectは`/work/app`とし、Windowsではこの絶対directory構造を同じ相対関係の専用rootへ対応づける。\
projectのsourceは`main.ond`の全文`package main`と`func main(){}`の2行、ond.tomlは次の全文であり、UTF-8・LF・各末尾LFありとする。\
他のproject・依存・既存cache・出力fileは置かず、準備として`ond compile ../app`を成功させる。

```toml
[project]
name = "app"
[compile]
output = "meta/normal.ondbuild"
test-output = "meta/tests.ondbuild"
cache-dir = "cache"
[link]
output = "out/game.bin"
test-output = "out/tests.bin"
startup = "external"
stack-size = 0x100
[link.ram]
base = 0x10000000
size = 0x10000
[link.exit]
mode = "loop"
```

成功はexit 0、バイナリのONDX読出し成功と設定への配置一致、指定先へのfile生成で比較する。\
file出力の成功行ではdebug sidecarの生成と、バイナリ全bytesの識別子への一致も比較する。\
失敗行は終了codeと診断原因を比較し、明記した既存fileと未生成の出力先を保持する。

| Case ID | 操作・入力変更 | 期待 |
| --- | --- | --- |
| CLI-01.project | `ond link --project ../app` | `/work/app/out/game.bin`とgame.onddebugを生成。入力はapp/meta/normal.ondbuild、cacheはapp/cache |
| CLI-01.cwd | cwdを/work/appへ移して`ond link` | 同じ設定元・入力・出力先 |
| CLI-01.output-argument | 元cwdで`ond link --project ../app -o chosen.bin` | `/work/run/chosen.bin`とchosen.onddebugを生成。ond.toml bytesを保持 |
| CLI-01.default-output | 準備compile前にlink.outputを省略し、`ond link --project ../app` | `/work/app/target/a.bin`とa.onddebugを生成 |
| CLI-01.no-auto-compile | 準備compile後main.ondを削除し、`ond link --project ../app` | 保存済みmanifestからlink成功 |
| CLI-01.missing-input | 準備後meta/normal.ondbuildを削除し、`ond link --project ../app` | exit 1。compileを行わずmanifestを再生成しない |
| CLI-01.no-test-fallback | 準備後`ond compile --test ../app`を行いnormal manifestだけを削除し、`ond link --project ../app` | exit 1。meta/tests.ondbuildを自動選択しない |
| CLI-01.no-parent-search | 元cwdで`ond link`。/work/ond.tomlへ上の設定を置くがrun/ond.tomlは置かない | exit 1。run直下の設定欠落を診断し、上位設定を採用しない |
| CLI-01.stdout | `ond link --project ../app -o -` | stdoutはexternalの標準バイナリ全bytesのみ。out/game.bin・debug/test sidecarを生成しない |
| CLI-01.sidecar-collision | `ond link --project ../app -o chosen.onddebug` | exit 1、出力path衝突。fileを書き込まない |
| CLI-01.duplicate-option | `ond link --project ../app --project ../app` | exit 2 |

`CLI-01.explicit-file`は、準備したmanifestの唯一のArtifactRef.pathを`cache/0.ondpkg`へ変更し、元artifact bytesを`/work/input/cache/0.ondpkg`へコピーする。\
変更manifestを`/work/input/job.ondbuild`へ保存し、`ond link ../input/job.ondbuild --project ../app`を行う。\
入力artifactはinput/cache/0.ondpkg、設定はapp/ond.toml、出力はapp/out/game.binとして成功を比較し、input内のond.tomlを要求しない。

`CLI-01.stdin`は同じartifact bytesを`/work/run/cache/0.ondpkg`へ置き、同じ変更manifest bytesをstdinとして`ond link - --project ../app`を行う。\
artifactをrun/cache/0.ondpkgから読み、同じproject設定・出力先で成功することを比較する。\
これらのpath変更はartifactのcontent_hashとbuild_idを変更せず、参照先の元bytesを保持する。

`CLI-01.removed-options`は`--ram 0:16`、`--stack 4`、`--read-only 0:16`、`--return-to 0`をそれぞれ`ond link --project ../app`へ追加し、各々exit 2と出力未生成を比較する。\
`ond compile ../app --cache-dir other`も独立にexit 2となる。

`CLI-01.config-errors`は準備後の設定を独立に変更し、`ond link --project ../app`のexit 1と出力未生成を比較する。\
変更はlink直下へ`unknown = 1`を追加、link.ram.sizeを削除、stack-sizeを文字列`"256"`へ変更、link.ram.sizeを4294967296へ変更、ROMをbase=0x10000000・size=0x10000としてRAMと重ねる、の各入力とする。

## 起動方式・最終address・識別子
<!-- ond-spec-id: conformance/executable-binaries/startup-and-identifiers -->

要件は`BIN-03`、`LINK-01`、`HASH-01`とし、定義元は[起動方式](../../artifacts.md#外部起動とセルフ起動)、[最終配置](../../linking.md#最終配置とstartup)、[識別子](../../build/cache.md#公開識別子)とする。\
source生成caseは前節のprojectを使い、main.ondだけを次の全文へ置換して準備compileを行う。\
実行時はRAMの全byteをa5、出力観測用RAM[0x20000100,0x20000108)の2 wordをdeadbeef、CPU registerを0で初期化し、指定RAM・ROM・観測用RAM以外をunmappedとする。

```ond
package main
var G:u32=7
func f()->u32{return G}
func main(){var p=&G;var out=536871168 as u32 as *u32;out[0]=f()+*p;out[1]=p as u32}
```

各成功生成物の配置済みTextとentry、およびsourceが外部へ書き込む値を観測する。\
成功行では2個の観測store完了後、word 0が14、word 1が4-byte alignedで、標準ヘッダのBss実行時範囲に`[word 1, word 1 + 4)`が包含されることを比較する。\
命令上限200000はこのcaseの観測打切り条件であり、実行環境のtest方針へ追加しない。

| Case ID | 操作・設定 | 期待・比較方法 |
| --- | --- | --- |
| BIN-03.external-ram | ROM省略のままcompile/linkし、標準全体をRAM baseへコピー、zero初期化とSP設定後ヘッダentryから実行 | 2 wordの観測値とGのBss包含。ヘッダ・表を反映したText/Rodata/Dataのmemory_address関係、debug全addressのText包含を比較 |
| BIN-03.external-rom | link.romをbase=0x1000・size=0x10000として追加し、compile/link。標準全体をROM baseへ配置しロード後entryから実行 | 2 wordの観測値とGのBss包含。Text/Rodataのmemory_address=0x1000+file_offset、Data/BssはRAM内 |
| BIN-03.self-rom | 同ROMのbaseだけ0、startup=selfへ変更してcompile/link。出力全体をROM 0へ配置しCPU resetから実行。外部からSPを設定しない | 2 wordの観測値とGのBss包含。前置起動処理がロード・zero・SPを準備。ヘッダ位置はlinkで確定した前置長と一致し、全file offsetはそのヘッダ基準 |
| BIN-03.self-without-rom | startupだけselfとしROMを省略してlink | 必須ROM設定のlink error |
| BIN-03.self-base | startup=self、ROM base=4・size=0x10000でlink | selfのROM base条件違反のlink error |
| BIN-03.self-stdout | self、ROM base=0・size=0x10000としてcompileし、`ond link --project ../app -o -` | stdoutは前置起動処理と標準バイナリを含む全出力bytes。fileとsidecarを生成しない |
| HASH-01.binary-full | 各成功生成物の全出力bytesのSHA-256を独立計算 | debug sidecarのbuild_idと一致。selfでは前置起動処理もhash入力 |
| BIN-03.debug-prefix | self生成物のdebug情報を読む | function/range/unwindは標準Text内。前置起動処理にsource・unwind情報を生成しない |

`HASH-01.binary-vector`はWの全84 byteだけをSHA-256へ入力し、`72d1a90b13286b8c57c4e1576be01d319d4070f963981067b9e55b2b73d4fc43`との完全一致を比較する。\
`HASH-01.binary-prefix`はWの前へ4個のzero byteを加えた88 byteをhashし、`718f1c5a7693158554c0a490403ecf5ca2c56ac0bc15e3f833a37c7ad1e4166e`と比較する。\
後者は全bytesをhashする操作だけの入力であり、selfの起動処理適合を検査する入力ではない。

`HASH-01.sidecar-mismatch`は成功生成物の独立copyと有効なdebug sidecarを用意し、sidecarのbuild_idの先頭hex digitだけを別値へ変更する（0なら1、それ以外なら0）。\
対象バイナリの識別子と照合してsidecar全体を使用拒否することを比較し、program実行の可否やtestの結果を比較項目へ含めない。

## Test生成と個別入口
<!-- ond-spec-id: conformance/executable-binaries/test-generation -->

要件は`TEST-01`とし、定義元は[test build](../../build/test-build.md)、[起動方式](../../artifacts.md#外部起動とセルフ起動)、[link.exit](../../build/packages.md#compileとlink設定)とする。\
前節までとは独立したprojectを/work/appへ作り、CLI節のond.tomlと次の3 source全文をUTF-8・LF・各末尾LFありで置く。

`main.ond`:

```ond
package main
var Counter:u32=3
func main(){}
```

`main_test.ond`:

```ond
package main
import "app/dep"
func TestOne(){*(536871168 as u32 as *u32)=Counter+dep.Value()}
func TestTwo(){*(536871168 as u32 as *u32)=Counter+1}
```

`dep/dep.ond`:

```ond
package dep
var Seed:u32=7
func Value()->u32{return Seed}
```

cwdを/work/appとし、各行の準備として`ond compile --test .`だけを行う。\
link操作は各行に記したものを用い、出力読出し・実行観測の行では`ond link meta/tests.ondbuild`を準備する。\
stdout・output-override行ではこの準備linkを行わず、以前のバイナリ・sidecarを持たない状態から表のlink操作を行う。\
実行を含む行は、ロード契約のsection byte列とzero初期化を準備し、SPをheader.initial_sp、他のregisterを0、観測用RAM[0x20000100,0x20000104)をdeadbeefとして開始する。\
指定RAM・ROM・観測用RAM以外はunmappedとし、観測用RAMへのstoreまたは記載したguard・終了経路を観測する。\
上限200000命令で観測を打ち切る。

| Case ID | 操作・入力変更 | 期待・比較方法 |
| --- | --- | --- |
| TEST-01.generated | 上のcompile/link出力を読む | out/tests.bin、tests.onddebug、tests.ondtestを生成。通常と共通のONDX version 1ヘッダを持ち、中間testsに(package app,name TestOne,symbol main.TestOne)とTestTwoの2件、sidecarに対応する2 wrapper entryを収録。各entryは4-byte alignedな標準Text内。build_idは全binary bytesのhashに一致 |
| TEST-01.normal | 同じprojectを`ond compile .`、`ond link`で生成 | out/game.binとgame.onddebugを生成。通常manifestとartifactはtests fieldを省略し、game.ondtestを生成しない |
| TEST-01.default-output | 準備前にlink.test-outputだけ省略してtest compile/link | target/test.bin、test.onddebug、test.ondtestを生成 |
| TEST-01.output-override | test compile後`ond link meta/tests.ondbuild -o other.bin` | other.bin、other.onddebug、other.ondtestを生成。out/tests.binを生成しない |
| TEST-01.empty | main_test.ondをpackage mainの1行だけとしてtest compile/link | 中間tests.casesとtest sidecarのcasesは空配列。test binと両sidecarを生成 |
| TEST-01.guard | 初期stateからheader.entry_addressへ制御を渡す | case選択要求のtrapで停止、個別caseの観測storeなし。この停止をtest成功通知として扱わない |
| TEST-01.one | 同じ初期stateからsidecarのTestOne entry_pointへ制御を渡す | initializer適用後に対象functionを1回呼び、観測値10。戻った後はlink.exitのloop内に制御を保つ |
| TEST-01.two | 独立した同じ初期stateからTestTwo entry_pointへ制御を渡す | 観測値4、return後は同じloop経路 |
| TEST-01.jump | link.exitをmode=jump,address=0x20000200としてcompile/link。終了先にJZ r0,r0,r0,-1のword=0xc000ffffを読み出せる領域を追加し、TestOne wrapperから実行 | 観測値10を書き、return後にPC=0x20000200へ制御を移す。成功通知用の新metadataを要求しない |
| TEST-01.self | startup=self、ROM base=0・size=0x10000を追加してtest compile/link。全出力をROM 0へ置き、SPを0としたCPU resetから実行 | loaderの準備後はheaderのcase選択guardで停止し、個別caseを自動実行しない。全bytesのhashに一致するtest/debug sidecarを生成 |
| TEST-01.stdout | test compile後`ond link meta/tests.ondbuild -o -` | test用ONDXバイナリ全bytesだけをstdoutへ出し、sidecarを生成しない |
| TEST-01.sidecar-collision | test compile後`ond link meta/tests.ondbuild -o chosen.ondtest` | exit 1、出力path衝突。fileを書き込まない |

`TEST-01.library`は独立projectを使い、CLI節のond.tomlのproject.nameだけをlibへ変更する。\
`lib.ond`の全文`package lib`と`var Value:u32=3`、`lib_test.ond`の全文`package lib`と`func TestValue(){*(536871168 as u32 as *u32)=Value}`を各2行で置く。\
`ond compile --test .`と`ond link meta/tests.ondbuild`がmainを要求せず成功し、sidecarの唯一の(package lib,name TestValue)入口を同じロード初期stateから呼ぶと観測値3となることを比較する。

`TEST-01.sidecar-mismatch`は生成したtest sidecarのbuild_idだけを前節と同じ方法で変更し、sidecar全体の使用拒否を比較する。\
このcaseは実行方法や合否解釈を入力とせず、wrapperの生成、入口、初期化、終了経路と識別子だけを比較する。
