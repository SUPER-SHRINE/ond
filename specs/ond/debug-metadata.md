# Ond Debug Metadata v1

本書は、Ond compilerが生成するdebug sidecarと、Kagura VMが任意で利用できるfault表示・stack trace情報を定義する。debug metadataは実行時ABIへ追加の引数、shadow stack、debug frameを導入しない。

## 1. 成果物

`ond link -o game.ondimage`は次の2ファイルを生成する。

- `game.ondimage`: 実行に必要なLinkedImage
- `game.onddebug`: source mapping、fault分類、function range、unwind規則を持つJSON sidecar

両者の`build_id`は一致しなければならない。VMは一致しないsidecarを使用してはならない。`-o -`ではstdoutを単一のLinkedImage JSONに保つためsidecarを生成しない。

## 2. sourceとfault range

sidecarのrangeは`[start, end)`の最終load addressで表す。各rangeはfunction symbol、任意のsource path・1始まりline/column、operationを持つ。

operationは`Source`、`Call`、`MemoryRead`、`MemoryWrite`、`BoundsCheck`、`DivisionByZero`、`SignedDivisionOverflow`、`InvalidConversion`、`StackOverflow`、`ExplicitTrap`である。

VMはCPU fault時のPCを含む最も狭いrangeを参照できる。bus/alignment faultはCPU側のfault種別と`MemoryRead`または`MemoryWrite`を組み合わせて分類する。compilerが挿入したcheckは対応するoperationを直接持つ。runtime helper内でfaultした場合はhelper frameと、その呼出元の`Call`または変換rangeをstack traceから参照できる。

`ExplicitTrap`だけは`message`を持つ。`trap "reason"`のliteralはdebug metadataへだけ格納し、LinkedImageのText/Rodata/Data/Bssへ格納しない。他のoperationの表示文言はVMが選べる。

## 3. functionとunwind

function entryは`address`、`size`、symbol、source、frame size、return-address slot、およびunwind rowを持つ。rowはそのaddress以降で有効な次の状態を表す。

- `cfa_sp_offset`: 現在のSPへ加算してcallerのSPを得る値
- `LinkRegister`: return addressはKaguraのlink register `r15`にある
- `StackOffset(n)`: return addressは現在のSPから`n` byteの位置に保存されている

VMはfault PCを含むfunctionを探し、PC以下で最大addressのrowを選び、return addressとcaller SPを復元する。この処理をfunctionが見つからない、addressが進まない、stack範囲を外れる、または実装上のframe上限へ達するまで繰り返せる。

unwind tableは実行時memoryへ配置されない。release buildでもsidecarを保存すればtraceでき、sidecarを配布しなければ実行imageのmemory使用量は増えない。

## 4. VM実装の責務

debug表示はKagura規格の必須機能ではない。対応するVMは最低限、次を行うことが望ましい。

1. LinkedImageとsidecarの`build_id`一致を確認する。
2. CPUが報告したfault種別と停止PCを取得する。
3. rangeからsource location、operation、明示理由を解決する。
4. function/unwind tableと停止時register・stack memoryからstack traceを構築する。

sidecarがない、または該当rangeがない場合でも、VMは従来どおりCPU faultとして停止できなければならない。
