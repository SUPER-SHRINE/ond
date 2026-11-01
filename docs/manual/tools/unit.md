# ond-unit の使い方

`ond-unit` は、通常の Ond project にある検証用関数を Kagura VM 上で実行し、ケースの終了結果、RAM、MMIO を確認するテストランナーです。\
ここでは各機能を使う場面と最小の設定例を説明します。\
設定項目や結果判定の完全な契約は[ond-unit 規範仕様](../../../specs/ond/tools/unit.md)を参照してください。

## 準備

`ond-unit` は Ond compiler の CLI を使って対象 project を compile、link します。\
Kagura VM は `ond-unit` に含まれ、MMIO 設定を使って実機 device を置き換えられます。\
`ond` と `ond-unit` の両方を PATH から実行できるようにしてください。\
配布版では同じ release の組み合わせを使います。\
たとえば `ond-unit` 0.4.0 には `ond` 0.4.0 を使います。\
ソースから使う場合は Ond repository の root で次を実行し、生成される両方の実行ファイルを PATH から利用できるようにします。

```console
cargo build -p ond-cli -p ond-unit --locked
```

テスト対象には通常の Ond project を指定します。\
`ond.toml` の `[project].final` は省略するか `false` にしてください。\
`final = true` の project は runner project の依存先にできません。

## 最初のケース

次の project は RAM の初期値を読み、値が `42` なら正常終了します。

```text
sample/
├── ond.toml
├── ond-unit.toml
└── checks/
    └── cases/
        └── check.ond
```

sample/ond.toml:

```toml
[project]
name = "sample"
```

sample/checks/cases/check.ond:

```ond
package cases

func Check() {
    var value = 0x8000 as *u32
    if *value != 42 {
        trap "unexpected value"
    }
}
```

sample/ond-unit.toml:

```toml
version = 1

[project]
path = "."

[vm]
ram_size = 65536
stack_size = 4096

[[cases]]
name = "check-initial-value"
package = "sample/checks/cases"
function = "Check"
ram_initial = [{ address = 0x8000, bytes = [42, 0, 0, 0] }]
```

`ram_size` は実行用 RAM 全体の byte 容量、`stack_size` は stack に予約する byte 容量です。\
この例では RAM が64 KiB、stack が4 KiBです。\
大きなデータや深い関数呼び出しを試す場合は、テストに必要な容量に合わせて値を調整します。

ケース関数は引数なし・戻り値なしの公開トップレベル `func()` にします。\
設定に登録した関数の存在、公開範囲、型は通常の Ond compile で確認されます。\
検査本体は普通の Ond 関数なので、条件分岐、loop、trap、補助関数を使えます。\
同じ package の非公開補助関数も通常どおり呼び出せます。\
package path は project 名と source の package 配置に合わせます。

sample directory で実行すると、`ond-unit.toml` が既定の設定として使われます。

```console
ond-unit
```

ケースごとの状態と集計が表示されます。\
全対象ケースが PASS なら終了コードは `0`、FAIL または STEP_LIMIT があれば `1`、ERROR があれば `2` です。

## RAM の初期値と期待値

`ram_initial` はケース開始時に RAM へ書く byte 列、`ram_expected` はケース停止時に確認する byte 列です。\
各配列要素をアドレス順に1 byteずつ対応させます。\
たとえば Ond から32-bit値 `0x12345678` を読みたい場合、Kagura の little-endian 順で下位 byte から指定します。

sample/checks/cases/ram.ond として保存する関数:

```ond
package cases

func ReadAndWrite() {
    var data = 0x8000 as *u32
    if *data != 0x12345678 {
        trap "input mismatch"
    }
    *data = 0x90abcdef
}
```

`ond-unit.toml` に追加する case entry:

```toml
[[cases]]
name = "ram-roundtrip"
package = "sample/checks/cases"
function = "ReadAndWrite"
ram_initial = [
  { address = 0x8000, bytes = [0x78, 0x56, 0x34, 0x12] },
]
ram_expected = [
  { address = 0x8000, bytes = [0xef, 0xcd, 0xab, 0x90] },
]
```

初期値だけを指定すれば入力準備になり、期待値だけならテストが変更した結果を確認できます。\
RAM 領域は通常 link 対象 RAM の範囲内に置き、初期値領域同士は重ねません。\
複数領域を指定すると、それぞれの指定 byte を照合します。

各ケースの CPU、RAM、step 数、device、応答位置は独立して初期化されます。\
あるケースの書き込みや MMIO 応答の消費は、次のケースへ持ち越されません。\
fault で停止した場合も停止までに完了した RAM 書き込みと MMIO access を確認します。\
期待 fault が一致しても、RAM や MMIO の別の期待が外れていれば PASS にはなりません。

## 期待 fault を設定する

通常終了を期待するケースで fault が起きると FAIL です。\
`expect_fault` を指定したケースは、fault が起きて指定条件が一致し、RAM と MMIO の期待も成立すると PASS になります。\
fault の種類に加えて、発生元を `function` または `source` で指定してください。

次の完全な source を sample/checks/cases/api.ond として保存します。\
`trap` は Ond の明示的な trap で、この例では5行目にあります。

```ond
package cases

func Helper() {}
func Pass() { Helper() }
func Reject() { trap "expected" }
func Loop() { for {} }
```

`ond-unit.toml` に追加する case entry:

```toml
[[cases]]
name = "expected-trap"
package = "sample/checks/cases"
function = "Reject"

[cases.expect_fault]
cpu_kind = "InvalidInstruction"
ond_kind = "ExplicitTrap"
function = "sample/checks/cases.Reject"
reason = "expected"
source = { path = "sample/checks/cases/api.ond", line = 5, column = 17 }
```

`cpu_kind` は CPU が返す fault の種類、`ond_kind` は debug metadata と CPU fault の対応から判定する Ond の意味分類です。\
どちらか一方は必須です。

| CPU fault | 確認する内容 |
| --- | --- |
| `InvalidInstruction` | CPU が命令を有効な命令として実行できなかった |
| `UnalignedAccess` | load または store の address が必要な境界に揃っていなかった |
| `UnalignedPc` | 命令を取得する PC が必要な境界に揃っていなかった |
| `BusFault` | RAM や device への access を完了できなかった |

| Ond fault | 確認する内容 |
| --- | --- |
| `BoundsCheck` | 配列や slice の index が有効範囲外になった |
| `DivisionByZero` | 整数除算の divisor が0になった |
| `SignedDivisionOverflow` | 符号付き除算の結果が表現範囲を超えた |
| `InvalidConversion` | 検査付き型変換の値が変換可能範囲外になった |
| `StackOverflow` | 関数呼び出しに必要な stack 領域が足りなかった |
| `ExplicitTrap` | Ond source の trap 文を実行した |

たとえば bounds check の失敗は CPU 上では `InvalidInstruction` として報告され、debug metadata の操作から `BoundsCheck` と判定されます。\
CPU の種類だけから Ond の意味分類を推測することはありません。

`function` には debug metadata の完全修飾 symbol を指定します。\
上の例では package path と関数名を連結した `sample/checks/cases.Reject` が関数名です。\
`source.path` と `source.line` は fault した命令に結び付いた debug source 情報に一致させます。\
`path` は debug metadata の表示識別子として完全一致で比べ、filesystem path として解決・正規化しません。\
`column` は任意です。省略すると行までを照合します。\
指定列は UTF-8 byte 単位です。\
上の例の trap は5行目17列目なので、source の行と列をその値にしています。

`reason` は debug metadata の message と完全一致で比べます。\
trap の文字列が `expected` なら `reason = "expected"` を指定します。\
`address`、`width`、`operation` は access fault の照合を絞るときに使います。\
次の関数を api.ond に追記すると、未整列の4 byte store を発生させられます。

```ond
func UnalignedStore() {
    var value = 0x8001 as *u32
    *value = 1
}
```

対応する case entry:

```toml
[[cases]]
name = "unaligned-store"
package = "sample/checks/cases"
function = "UnalignedStore"

[cases.expect_fault]
cpu_kind = "UnalignedAccess"
function = "sample/checks/cases.UnalignedStore"
address = 0x8001
width = 4
operation = "store"
```

fault の `operation` には `fetch`、`load`、`store` を指定します。\
MMIO の期待 access に使う `read`、`write` とは別の値です。\
複数の fault 条件を指定すると、すべて同じ fault に一致する必要があります。\
たとえば `function` と `source` を併記した場合は両方を照合します。\
fault が起きない場合、別の fault が起きた場合、または条件の一つが一致しない場合は FAIL です。\
照合に必要な debug 情報を取得できない場合は ERROR になります。\
debug metadata は build で生成され、`ond-unit` が image との対応を含めて自動照合します。\
`function`、`source`、`reason` を見つけるには、まず `expect_fault` を付けずにケースを実行してください。\
FAIL 診断に表示される function symbol、`source=path:line:column`、fault kind、reason を設定値として使えます。

## MMIO を stub / mock する

MMIO は実機 device を使わずに読み出し値を与え、読み書きの結果を検証するために使います。\
root 設定には device 名と絶対 address 範囲を定義します。\
次の例は `0x10000000` から4 byteを使う `port` device です。

```toml
[[mmio]]
name = "port"
base = 0x10000000
size = 4
```

次の関数を api.ond に追記すると、port から `7` を読み、`9` を書き込みます。

```ond
func ReadWritePort() {
    var port = 0x10000000 as *u32
    var value = *port
    if value != 7 {
        trap "port mismatch"
    }
    *port = 9
}
```

読み出し応答だけを用意し、回数や順序を検証しない設定は stub として使えます。\
case entry の `cases.mmio` に `read_responses` を設定します。

```toml
[[cases]]
name = "port-stub"
package = "sample/checks/cases"
function = "ReadWritePort"

[[cases.mmio]]
device = "port"
read_responses = [{ width = 4, value = 7 }]
```

access の回数、順序、address、幅、値まで検証する場合は `expected_accesses` を加えます。\
stub と mock は別々の case entry なので、どちらか一方でも両方でも登録できます。

```toml
[[cases]]
name = "port-mock"
package = "sample/checks/cases"
function = "ReadWritePort"

[[cases.mmio]]
device = "port"
read_responses = [{ width = 4, value = 7 }]
expected_accesses = [
  { operation = "read", address = 0x10000000, width = 4, value = 7 },
  { operation = "write", address = 0x10000000, width = 4, value = 9 },
]
```

応答列と期待 access 列は独立しています。\
`read_responses` は読み出しのたびに1件消費し、設定した width と実際の幅が一致する必要があります。\
`expected_accesses` は完了した access 列を device ごとに順番と件数まで照合します。\
別 device 同士の全体順序は照合しません。\
書き込み値は後の読み出し値にならないため、読み出し応答を別途設定してください。\
応答列に未使用の値が残っても、それだけでは失敗しません。\
応答列が尽きた場合や幅が違う場合は FAIL で停止し、期待 fault があっても PASS にはなりません。

`expected_accesses` を省略すると、その device の access 列は照合しません。\
空列を指定すると、その device に access がないことを検証します。

```toml
[[cases]]
name = "no-port-access"
package = "sample/checks/cases"
function = "Check"
ram_initial = [{ address = 0x8000, bytes = [42, 0, 0, 0] }]

[[cases.mmio]]
device = "port"
expected_accesses = []
```

MMIO access の幅は1、2、4 byteです。\
期待値の address と全 access 幅は、宣言した device 範囲に収めます。\
MMIO 領域は RAM と重ならず、互いにも重ならないようにします。

## project role を選ぶ

対象 project が名前付き role を宣言している場合は、設定の `[project].role` で compile に使う role を選べます。\
sample/ond.toml の既存 project 設定へ次の table を加えます。

```toml
[roles]
names = ["Test"]
```

次の source を sample/checks/cases/role.ond として保存します。

```ond
package cases

role Test {
    func VerifyFeature() {
        var value = 0x8000 as *u32
        if *value != 42 {
            trap "feature mismatch"
        }
    }
}
```

sample/ond-unit-role.toml:

```toml
version = 1

[project]
path = "."
role = "Test"

[vm]
ram_size = 65536
stack_size = 4096

[[cases]]
name = "role-feature"
package = "sample/checks/cases"
function = "VerifyFeature"
ram_initial = [{ address = 0x8000, bytes = [42, 0, 0, 0] }]
```

sample directory で `ond-unit ond-unit-role.toml` を実行します。\
選択した role は対象 project の全 package と登録済みの全ケースに適用されます。\
role を省略すると名前付き role を選ばない共通構成になります。\
補助関数や role 内の宣言には通常 Ond project と同じ可視性・参照規則が適用されます。\
詳しくは[Ond project role 仕様](../../../specs/ond/build/roles.md)を参照してください。

## ケースを別ファイルへ分ける

case が増えたら、共通環境を root に置いたまま package ごとの TOML file に分けられます。\
次の例は root case と2つの child file を含み、前掲の sample project と source file を使います。

```text
sample/
├── ond.toml
├── ond-unit.toml
├── cases/
│   ├── io.toml
│   └── memory.toml
└── checks/
    └── cases/
        ├── check.ond
        ├── api.ond
        └── ram.ond
```

sample/ond-unit.toml:

```toml
version = 1
case_files = ["cases/io.toml", "cases/memory.toml"]

[project]
path = "."

[vm]
ram_size = 65536
stack_size = 4096

[[mmio]]
name = "port"
base = 0x10000000
size = 4

[[cases]]
name = "check-initial-value"
package = "sample/checks/cases"
function = "Check"
ram_initial = [{ address = 0x8000, bytes = [42, 0, 0, 0] }]
```

sample/cases/io.toml:

```toml
package = "sample/checks/cases"

[[cases]]
name = "port-mock"
function = "ReadWritePort"

[[cases.mmio]]
device = "port"
read_responses = [{ width = 4, value = 7 }]
expected_accesses = [
  { operation = "read", address = 0x10000000, width = 4, value = 7 },
  { operation = "write", address = 0x10000000, width = 4, value = 9 },
]
```

sample/cases/memory.toml:

```toml
package = "sample/checks/cases"

[[cases]]
name = "ram-roundtrip"
function = "ReadAndWrite"
ram_initial = [{ address = 0x8000, bytes = [0x78, 0x56, 0x34, 0x12] }]
ram_expected = [{ address = 0x8000, bytes = [0xef, 0xcd, 0xab, 0x90] }]
```

root case の package は各 case で指定でき、root と child の package は混在できます。\
child file の先頭に書いた package は、そのファイル内の全 case に適用されます。\
子 case では package を指定せず、ファイル先頭に1つだけ設定します。\
実行順は root cases の後に case_files の指定順、その中では各配列の順です。\
case 名はすべての file を通じて一意にしてください。\
CLI で選択しても登録順と case index は変わりません。\
相対 project.path と case_files path は root 設定 file のある directory を基準に解決します。\
case_files は literal path の列で、glob pattern は展開されません。\
この設定は sample directory で ond-unit を実行すると使えます。

## 実行上限を調整する

各 case の既定上限は `1_000_000` step です。\
終了しない loop や予想外に長い処理を制限したい場合は、case ごとに `max_steps` を指定します。\
前掲 api.ond の `Loop` 関数を使う例です。

```toml
[[cases]]
name = "loop-is-bounded"
package = "sample/checks/cases"
function = "Loop"
max_steps = 1000
```

`max_steps` は正の整数で、既定値より小さくも大きくもできます。\
上限内に正常終了も fault もせず、次の命令試行が必要な場合は `STEP_LIMIT` になります。\
この上限は wall-clock 時間ではなく、CPU 命令試行数です。\
fault した命令の試行も1 stepに数えます。\
起動処理、global 初期化、runner の `main()` も計数に含まれます。\
既定では STEP_LIMIT の後も後続 case を実行します。\
STEP_LIMIT は期待 fault の一致で PASS にはなりません。

## CLI でケースを選ぶ

前掲の case_files 設定を sample directory で使う場合、`check-initial-value` と `port-mock` を選択できます。\
基本構文は次のとおりです。

```text
ond-unit [CONFIG] [--case NAME]... [--fail-fast]
```

設定 file を省略すると、current directory の `ond-unit.toml` を使います。\
別の設定 file を使う場合は path を指定します。

```console
ond-unit path/to/ond-unit.toml
```

`--case` を繰り返すと、登録済み case name から実行対象を選べます。

```console
ond-unit ond-unit.toml --case check-initial-value --case port-mock
```

重複して指定した名前は1回だけ実行します。\
選択した case も全体の登録順で実行されます。\
存在しない名前は実行前に ERROR になります。\
選ばれなかった case の設定と関数も、設定検証と通常 compile の対象です。

既定では FAIL や STEP_LIMIT の後も残りの case を実行して集計します。\
`--fail-fast` は最初の FAIL または STEP_LIMIT で停止します。\
設定や compile の ERROR は fail-fast の有無にかかわらず停止します。

```console
ond-unit ond-unit.toml --fail-fast
```

## 結果を読む

各 case の結果には状態、step 数、停止 PC、必要に応じて理由と fault 情報が表示されます。\
通常終了または期待 fault が成立し、RAM と MMIO の指定もすべて一致すると PASS です。\
終了期待との不一致、RAM の byte 不一致、MMIO の期待不一致は FAIL です。\
上限内に終了できない場合は STEP_LIMIT、設定・compile・実行環境の問題は ERROR です。

集計には PASS、FAIL、STEP_LIMIT、ERROR の件数が表示されます。\
終了コード `0` は全対象 case が PASS、`1` は ERROR がなく FAIL または STEP_LIMIT がある場合、`2` は ERROR がある場合です。\
終了コードを使って CI や shell script から結果を判定できます。

## 生成物

`ond-unit` は対象 project の `target/ond-unit/` に runner 用 project を生成し、通常の Ond compile cache を使います。\
設定や case の登録に合わせて生成 project は更新されます。\
利用者の source と `ond.toml` は書き換えられません。\
通常、生成 project のファイルを直接編集せず、case 設定と Ond source を更新します。