# 適合ケースの入力・期待結果契約

## Case

| field | contract |
| --- | --- |
| `id` | registry内で一意なcase ID |
| `specs` | [規範要件ID](./conformance-checklist.md)の実在するID列 |
| `layer` | `Parse`、`Core`、`MemoryVm`のいずれか |
| `manifest` | project manifestを作成するかどうか |
| `files` | project相対pathとUTF-8 source本文の組 |
| `expected` | `Accept(checks)`または`Reject(diagnostics)` |

`files`は複数のsource fileとpackageを含められ、空配列も指定できる。\
各実行は独立した一時projectを使用する。\
fixtureの書込失敗はcompilerのsource診断として扱わず、所有する一時directoryは成功・error・panicの各終了経路で削除する。

source pathはスラッシュ区切りの相対`.ond` pathとする。\
絶対path、空のpath要素、`.`、`..`、backslash、drive／ADSに使う`:`、Windowsで無効な`< > " | ? *`、末尾がdotまたはspaceのpath要素を拒否する。\
重複pathは大小文字を区別せず拒否し、Windowsのdevice名`CON`、`PRN`、`AUX`、`NUL`、`COM1`〜`COM9`、`LPT1`〜`LPT9`も拒否する。

## 検証層と成功結果

| layer | 入力から実行する工程 | 許可するcheck |
| --- | --- | --- |
| `Parse` | loader、全fileの字句・構文解析 | AST callback、受理のみ |
| `Core` | loader、AST、HIR、MIR、project validator | AST、HIR定数の型/値、MIR callback、受理のみ |
| `MemoryVm` | Core工程とtest-only memory VMによる実行 | `MemoryExecution`とAST/HIR/MIR check |

`Core`のRejectはloader、字句、構文、意味検査のいずれの拒否も表せる。\
期待する原因は診断の内容と位置で指定する。\
`Accept(&[])`は受理だけを確認し、値や命令の正しさを主張しない。\
Core成功時は追加checkの有無にかかわらずMIR validatorを通す。

MemoryVmの実行checkはこの層だけで許可し、Acceptには1つ以上の`MemoryExecution`を要する。\
各checkは独立したglobal初期化状態から開始する。

| check | 期待値 |
| --- | --- |
| `MemoryExecution` | 必須description、関数名、入力値、期待戻り値配列 |
| `Ast` | 必須descriptionと`Result<(), String>`を返すAST callback |
| `HirConstant` | package論理path、定数名、期待型、期待値 |
| `Mir` | 必須descriptionと`Result<(), String>`を返すMIR callback |

callbackの`Err`はcheck失敗を表す。\
`MemoryExecution`の実行エラーまたは期待戻り値との不一致も失敗とする。\
失敗結果にはdescription、case ID、spec ID、layerを含める。

HIR定数の期待値はcompilerの評価結果から生成しない。\
整数は`u64`で表し、signed値はtwo's complementを符号拡張する。\
floatはNaNやsigned zeroを識別できる`u32`のbinary32 bitsで表す。\
Scalarは`Integer(u64)`、`Float32(u32)`、`Bool`、byte列の`Bytes`、`Nil`を使う。\
型名は`TypeTable::display`の文字列を使い、匿名structの同一性を具体的な`TypeId`番号で固定しない。

## 拒否時の診断

`ExpectedDiagnostic`はseverity、`Message::Exact`または`Message::Contains`、および`Location`を指定する。\
診断件数は期待値と一致させ、診断順に依存しない一対一の対応を要求する。

- Source locationはfile、対象text、0始まりのoccurrenceで指定し、UTF-8 byte start/endをprimary spanと照合
- EndOfFile locationはfileとbyte長でEOFのzero-width spanを照合
- Project locationはsynthetic spanを明示して照合
- internal MIR errorは通常のsource rejectionとして受理しない

コンパイル失敗時もLoadedProjectを保持し、source pathとspanを入力fixtureに対応付ける。\
失敗結果にはcase IDとspec IDを含める。
