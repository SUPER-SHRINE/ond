# 数値仕様のテストデータ

対象: NUM-01/03/04/05、CAST-02/03、FP-01/02。仕様は[Language v1](../../../specs/ond/v1/language.md) §3・§6。
P0は「仕様をテストデータとして列挙し、coreで可能な検証を実行する」工程であり、MIR backendの実機適合完了ではない。

## 構成と件数

[numeric/](../src/conformance/numeric/mod.rs)の機能別moduleでデータを生成し、既存の[共通runner](./conformance-cases.md)へ渡す。
Case ID、spec IDs、source files、受理/拒否、期待型/値/診断、Core層を従来どおり明示する。
生成sourceの借用を許すため共通型にlifetimeを追加した。production APIの変更ではない。
診断は件数・文言・file・byte spanを照合する。数値表のspan指定は該当textの最後の出現を使い、同名定数の宣言と使用を区別する。

| データ | ケース数 | 照合内容 |
| --- | ---: | --- |
| [integers.rs](../src/conformance/numeric/integers.rs) | 392 | 整数定数の型/値、範囲・shift・除算エラー |
| [casts.rs](../src/conformance/numeric/casts.rs) | 264 | 整数変換171件、int↔f32変換93件 |
| [floats.rs](../src/conformance/numeric/floats.rs) | 69 | 固定のbinary32結果bits33件、比較36件 |
| [float_vectors.rs](../src/conformance/numeric/float_vectors.rs) | 25 | 入力bits・operator・結果bitsを保持し、定数結果を照合 |
| [runtime.rs](../src/conformance/numeric/runtime.rs) | 1 project | 180関数の動的MIR契約 |
| 合計 | 751 | 共通形式による適合ケース |

加えて[共有整数vectors](../src/test_vectors/integers.rs)に実行時整数演算・変換の入力/期待結果336件を保持する。
coreは独立整数oracleの自己検証を行い、backendは同じvectorsを実Kaguraで実行する（2026-09-20接続）。coreの自己検証件数とbackend実行件数は別に扱う。
matrix_inventoryとregistry_is_validで件数、既存ケースを含むID重複を検査する。

## 整数6型

型・幅・min/maxはcompilerのTypeTableから取得せず、仕様値を明記する。

| 型 | bit幅 | min | max | shift count |
| --- | ---: | ---: | ---: | --- |
| u8 | 8 | 0 | 255 | 0, 7, 8, 9, 4294967295 |
| i8 | 8 | -128 | 127 | 0, 7, 8, 9, 4294967295 |
| u16 | 16 | 0 | 65535 | 0, 15, 16, 17, 4294967295 |
| i16 | 16 | -32768 | 32767 | 0, 15, 16, 17, 4294967295 |
| u32 | 32 | 0 | 4294967295 | 0, 31, 32, 33, 4294967295 |
| i32 | 32 | -2147483648 | 2147483647 | 0, 31, 32, 33, 4294967295 |

- NUM-01: min−1/min/0/max/max+1、単項+/−、負のzero、2/8/10/16進、hexのmin/max、既定i32の境界。
- NUM-03: 左shift、max/minの右shiftを全countで照合。定数左shiftは正確値で、signedの最上位bitへのshiftは範囲エラー。countの負数/u32範囲外/i32型も拒否。
- NUM-04: 正負4象限（unsignedは正数のみ）、min/max÷1、被除数とremainderの符号、0除算、signed min/−1の拒否とmin%−1=0。
- NUM-05: `+ - * / % & | ^ &^ << >> == != < <= > >=`、単項`+ - ^`を各型で列挙。定数のoverflowは拒否し、runtimeのwrap期待値とは混同しない。
- CAST-02: 6×6の全36方向。sourceのmin/max、destinationのmin/maxと直外側のうちsourceで表現できる値。sourceでも表現できないliteralはNUM-01で拒否する。

実行時のgolden vectorsにはwrap、最小負数の単項−、shiftのmodulo、比較、div/rem fault、全整数変換方向を含む。
oracleはi128の数学的演算とmoduloで計算し、compilerのconstant evaluator・MIR opcode選択・既存簡易VMを呼ばない。
golden側の変換値はbit maskと符号拡張で求める。oracleの成功はbackend実装の正しさを意味しない。

## f32・整数変換

- 各整数型のmin/max→f32を固定bitsで照合。i32/u32の2^24前後は正負のties-to-evenを確認。
- f32→6整数型は下限/上限と直外側、正負fraction、−0、−0.75→0、NaN/±Infを照合。
- i32/u32の上限付近は、数学的maxではなく、binary32で表現可能な直前の値と範囲外になる次の値をhex floatで明記する。
- 四則演算、単項+/−、ties-to-even（加減算の通常値、乗除算のsubnormal境界）、subnormal維持、正負underflow、overflow、signed zero、canonical NaNを固定bitsで検証。
- NaNを左/右/両方に置き、全6比較を検証。signed zero・通常値・±Infでも全比較を照合。

float_vectorsの入力bitsは正確なhex literalへ変換してsourceに埋め込み、期待bitsは固定値を使用する。
将来、同じ入力bitsをbackend実行へ渡してFP-02の定数/runtime bit一致を検証できる。

## 動的MIRの検証と未完了の範囲

runtime.rsの180関数は全整数binary 102、float binary 10、unary 20、変換48からなる。
parameterから読み出すoperand、operand/result型、選択opcode、returnへ渡すvalueを確認する。
signed/unsigned比較・右shiftを区別し、shift countがu32であることを検証する。
integer div/remは先行するNonZero guard、signed divはSignedDivisionOverflow guardを要求する。
f32除算にguardやtrapping属性を付けない。float→integer castはChecked、それ以外はInfallible。
opcode差替え・operand交換・guard削除・float除算へのtrapping属性付与が検査に失敗するmutationテストも置く。

この追補で、f32除算に誤ってChecked属性が付くMIR生成の不一致を修正した。

2026-09-20追補: 336整数vectorsと[25float bits](../src/test_vectors/floats.rs)をMIR backend→Kagura実行へ接続し、wrap/fault/bit結果・NaN正規化を照合した。12方向のint↔f32境界93ケースも追加した。
詳細・残件は[backend適合表](../../../specs/ond/targets/kagura-v1/conformance-checklist.md)を参照。共有データはopt-inの`conformance-fixtures` featureで公開するtarget非依存のテスト資産であり、通常のcompiler APIとは分離する。
既存AST backendの成功やoracleの自己検証を、その証拠として代用しない。
P1で型付き値の代入/引数/return行列を追加済（[型境界表](./type-boundary-conformance.md)）。
literal/定数式×6整数型×各値文脈の全対照は引き続きcoreの追補対象で、型付きparameterの行列とは区別する。
全bit patternの網羅や浮動小数点全ケースの形式的証明は対象外。

## 実行

```text
cargo test -p ond-compiler-core conformance::numeric
cargo test --workspace --quiet
```

P0追加時は全297 Rustテスト成功（compiler-core 190件）。最新の再監査結果は[チェックリスト§1](./conformance-checklist.md#1-対象と判定方法)を参照。
751はテーブル内ケース数であり、Rust test関数数とは別に数える。
