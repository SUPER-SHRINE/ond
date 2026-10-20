# 未決定事項一覧

この一覧はcommit `06e87c39d99f2389af163d0394d9808f2c3078bd`を基準に、[目次・移行表](./migration-map.md)で列挙した文書群を棚卸しして作成した。\
2026-10-20の会話でSPEC-Q01〜SPEC-Q05とSPEC-Q07〜SPEC-Q09の採用方針が決裁され、全9項目がresolvedとなった。\
決定記録と移行作業は区別し、条文と相互参照の移行結果は[本移行記録](./migration-record.md)で管理する。

仕様書の編集原則は[仕様書の編集方針](./editorial-policy.md)、移行先は[目次・移行表](./migration-map.md)、意味の反映例は[整数演算と定数評価の改稿見本](./integer-arithmetic-sample.md)を参照する。\
既存`ond-spec-id`と適合要件IDは維持し、定義元参照を移行先へ更新する。

## 確認済みの表記整合

### NOTE-INIT-01 — `INIT-03`の要件要約

- 状態: 本移行で言語と要件要約の表記を統一済み
- 証拠: [Language v1の初期化とentry](../../specs/ond/language/v1/execution.md#初期化とエントリ)（`ond-spec-id: runtime/initialization-and-entry`）は`init`を通常関数名として許可する
  - [conformance checklist](../../specs/ond/conformance/requirements.md)は定義元本文を正とし、[scope cases](../../crates/compiler-core/src/conformance/scope.rs)の`init_empty`、`init_parameter`、`init_return`、`init_both`、`imported_init`は受理を期待する
- 確認済みの意味: `init`は通常関数として宣言し、明示呼出しで起動する
  - package-level variableはzero valueまたはcompile-time constant initializerを持ち、compilerはglobal initializerを適用する内部関数を生成できる
  - 通常build entryは`main`である
- 整理候補: `INIT-03`要約をsource-level init hookと通常関数の区別が伝わる表記にする
- 移行結果: 対応する概念章・要件索引へ反映し、[照合記録](./migration-record.md)で旧新対応を追跡する

## 決裁済み事項

| ID | 状態 | 種別 | 決定 |
| --- | --- | --- | --- |
| SPEC-Q01 | resolved | 規則の記述 | 単項・整数算術bitwise・shift・比較のoperand/result型対応表 |
| SPEC-Q02 | resolved | 規則の記述 | 数学的正確値と値要求境界のrange check |
| SPEC-Q03 | resolved | 外部規範の選択 | Unicode 16.0.0を固定し字句規則をOnd本文で定義 |
| SPEC-Q04 | resolved | 外部契約の適用範囲 | allocation結果・alignment・lifetime等を規範化し、allocator方式はsource管理 |
| SPEC-Q05 | resolved | 外部契約の抽出範囲 | 利用境界ごとに既存仕様章へ抽出し、MIR内部詳細はsource管理 |
| SPEC-Q06 | resolved | 定義元配置 | Object/LinkedImage/linkingの外部契約をspecsへ整理 |
| SPEC-Q07 | resolved | 版管理の記述 | 既存版識別単位と依存・互換条件を明記しcompiler_idを別扱い |
| SPEC-Q08 | resolved | 配置の詳細 | 外部cache契約を`build/cache.md`へ集約 |
| SPEC-Q09 | resolved | 適合要件の識別 | 既存IDを維持し規範・要件・caseを相互参照 |

### SPEC-Q01 — 比較演算と整数演算の結果型記述

- status: resolved
- 種別: 規則の記述
- 証拠: [Language v1の整数演算](../../specs/ond/language/v1/expressions.md#integer-arithmetic)（`ond-spec-id: expressions/integer-arithmetic`）はshift以外のbinary operand型と結果型を記す
  - [式一覧](../../specs/ond/language/v1/expressions.md#式と評価順)（`ond-spec-id: expressions/overview`）は整数比較を列挙する
- 決定: 整数単項演算の結果型はoperand型`T`、整数算術・bitwise二項演算のoperand/result型は`T`、shiftの左operand/result型は`T`、shift count型は`u32`、比較のoperand型は同一の整数型`T`で結果型は`bool`とする
- 理由: 規則を演算群ごとに対応づけ、比較の真偽値結果を整数演算結果型から区別する
- 承認根拠: 2026-10-20のユーザー決裁Q01=A
- 移行結果: 対応する概念章・要件索引へ反映し、[照合記録](./migration-record.md)で旧新対応を追跡する

### SPEC-Q02 — 定数式の型文脈と範囲検査時点

- status: resolved
- 種別: 規則の記述
- 証拠: [Language v1の整数定数演算](../../specs/ond/language/v1/expressions.md#integer-constant-arithmetic)（`ond-spec-id: expressions/integer-constant-arithmetic`）は数学的正確値と要求型範囲を定める
  - [適合要件NUM-02](../../specs/ond/conformance/requirements.md)は任意精度定数とrange checkを記す
- 決定: 整数定数部分式は数学的正確値で計算し、宣言・代入・argument・return・明示cast等の値要求境界で要求型範囲を検査する
  - 配列length、index、shift countは各固有の成立条件で検査する
- 承認済み期待値: `const X: u8 = (250 + 10) - 10`は受理して250、`const Y: u8 = (250 + 10) as u8`はcompile error、`const Z: u8 = 250 + 10`はcompile error
- 理由: 数学的中間値とsource型の範囲を分離し、明示castはその位置で要求される型変換として検査する
- 承認根拠: 2026-10-20のユーザー決裁Q02=A
- 移行結果: 対応する概念章・要件索引へ反映し、[照合記録](./migration-record.md)で旧新対応を追跡する

### SPEC-Q03 — Go字句規則の参照版

- status: resolved
- 種別: 字句規則と外部規範の扱い
- 証拠: [Syntax v1のSource Form](../../specs/ond/language/v1/lexical.md#source形式)（`ond-spec-id: syntax/source-form`）は字句規則をGoと同じとし、[identifier implementation](../../crates/compiler-core/src/identifier.rs)はUnicode categoryによる開始・継続文字を判定する
  - 依存の`unicode-general-category` 1.1.0はUnicode version 16.0.0を示す
- 決定: Ond本文で字句規則を直接定義し、Unicode 16.0.0を文字分類の版として固定する
  - Go specificationは規則の由来を示す参考資料として扱う
- 選択: Unicode `Lu`, `Ll`, `Lt`, `Lm`, `Lo`と`_`をidentifier開始文字とし、継続文字に`Nd`を加える
- 理由: 文字分類の基準を固定し、Go仕様の将来改訂とOnd v1の定義を独立して再現可能にする
- 承認根拠: 2026-10-20のユーザー決裁Q03=A
- 移行結果: 対応する概念章・要件索引へ反映し、[照合記録](./migration-record.md)で旧新対応を追跡する

### SPEC-Q04 — Kagura allocatorの外部保証範囲

- status: resolved
- 種別: 外部契約の適用範囲
- 証拠: [Kagura Machine Profile v1のHeap allocator](../../specs/ond/targets/kagura-v1/machine.md)（`ond-spec-id: targets/kagura-v1/heap-allocator`）は4-byte header、block列、first-fit、分割、隣接block結合、末尾回収を契約として定める
- 決定: 外部規範は成功した領域のsize・alignment・存続中allocationとの分離、`new`のzero initialization、`alloc`の未初期化領域、failure・size overflow・zero-size・nil、freeの入力と解放の結果を保証する
  - first-fit、header、block分割・結合・回収方式はRust/Ond sourceで管理する
- 影響として承認済み: allocator方式を異ならせるKagura実装では返却addressとfragmentation時のallocation成功可否が変わりうる
  - 現行保証範囲からこの方式要件を外部契約の対象外へ移す変更として承認された
- 理由: 外部仕様はallocation利用者が観測する契約を保証し、allocator方式の選択は実装ごとの内部設計として扱う
- 承認根拠: 2026-10-20のユーザー決裁Q04=B
- 移行結果: 対応する概念章・要件索引へ反映し、[照合記録](./migration-record.md)で旧新対応を追跡する

### SPEC-Q05 — MIR backendの外部契約抽出範囲

- status: resolved
- 種別: 外部契約の抽出範囲
- 証拠: [MIR Design](../../specs/ond/conformance/implementation-index.md)はMIRをcompiler内部形式として記す
  - [Kagura MIR backend契約](../../specs/ond/conformance/implementation-index.md)は§7 `ond-spec-id: targets/kagura-v1/mir-boundary-decisions`と§8 `ond-spec-id: targets/kagura-v1/mir-backend-object-contract`に規範契約を含む
- 決定: 既存仕様の利用境界ごとに外部契約を抽出する
  - language章は値・評価順・数値・fault、machine章はlayout・raw pointer・MMIO・zero-size、ABI章はargument・result・register、artifacts/linking章はsection・symbol・relocation・中継slot、startup章はinit・entry・SDK契約を扱う
  - MIR形状・phase・lowering・helper・命令選択はsource管理とする
- 保持する契約: 外部Objectが共有するCallSlot/JumpSlot 24-byte形式とscratch契約を対象章に残し、内部除算32段loopなどの実装手順はRust/Ond sourceで管理する
- 理由: 利用境界ごとの外部契約を規範の適切な定義元で読める構成にし、compiler内部表現に対する共通実装義務を導入しない
- 承認根拠: 2026-10-20のユーザー決裁Q05=A
- 移行結果: 対応する概念章・要件索引へ反映し、[照合記録](./migration-record.md)で旧新対応を追跡する

### SPEC-Q06 — linking契約の定義元配置

- status: resolved
- 種別: 定義元配置
- 証拠: [Language v1のBuild Output](../../specs/ond/language/v1/execution.md#build-output)（`ond-spec-id: build/output`）は現行link工程をcompiler文書へ委譲する
  - [Ond Linker](../../specs/ond/linking.md)と[linker README](../../specs/ond/linking.md)は外部に関係するObject/LinkedImage、symbol、relocation結果と内部実装記述を含む
- 決定: Object/LinkedImage/linkingの外部契約を`specs/ond/artifacts.md`と`specs/ond/linking.md`へ整理し、compiler内部詳細の管理元をRust/Ond sourceとする
- 根拠: 2026-10-20に採用された外部契約とsource集約方針
- 移行結果: 対応する概念章・要件索引へ反映し、[照合記録](./migration-record.md)で旧新対応を追跡する

### SPEC-Q07 — 成果物・language・ABI・compiler版の互換性記述

- status: resolved
- 種別: 版管理の記述
- 証拠: [package build契約](../../specs/ond/build/packages.md#package成果物独立compiler契約)（`ond-spec-id: build/package-artifact-contract`）は公開情報revision 3を説明する
  - [protocol source](../../crates/protocol/src/lib.rs)の現行値はREVISION=5、TARGET=`kagura-v1`、ABI=`ond-kagura-v1`である
- 決定: 既存識別単位を維持し、language・target・ABI・artifact contractの版、適用対象、相互依存、互換条件を仕様書上の対応表で記す
  - 対応表は現行schema fieldの観測値を整理し、source pragmaや新しいJSON field/version軸を設計する作業は含めない
  - `compiler_id`は生成元ID・再利用識別子として記し、semantic versionとは区別する
  - manifest/artifact/linkerの`compiler_id`一致条件も対象とする
- 対応表の項目: 版識別軸、適用対象、現行版、依存先と互換条件

#### 現行識別値の棚卸し

| 識別軸 | 対象・観測値 | 依存・互換条件 |
| --- | --- | --- |
| language | Ond v1 | source rules version。対象言語規則に適用 |
| target | `kagura-v1` | Kagura machine profileに適用 |
| ABI | `ond-kagura-v1` | 対象targetのcall conventionに適用 |
| artifact contract | revision `5` | target/ABI tupleと共にschema受理条件を検査 |
| `compiler_id` | executable SHA-256由来の生成元/再利用ID | manifest/artifact/linkerで同一値を要求。semantic version軸とは区別 |

- 版の根拠: package-buildのrevision 3見出しは過去公開情報を指し、現行観測tupleは(5, `kagura-v1`, `ond-kagura-v1`)である
  - tests metadata導入時のrevision 4から5への変更は同一commitの履歴を根拠に対象schemaを照合する
- 理由: 既存識別単位を明確にし、format/APIの依存関係とcompiler実行ファイル識別を別の目的として扱う
- 承認根拠: 2026-10-20のユーザー決裁Q07=Aと、識別軸ごとの版依存・互換関係を表記する旨の追加回答
- 移行結果: 対応する概念章・要件索引へ反映し、[照合記録](./migration-record.md)で旧新対応を追跡する

### SPEC-Q08 — 二種類のcacheの案内先

- status: resolved
- 種別: 配置の詳細
- 証拠: [semantic cache](../../specs/ond/build/cache.md#意味解析cache)（`ond-spec-id: build/semantic-cache`）はprocess内の一時cacheを説明し、[package build契約](../../specs/ond/build/packages.md)は永続Object cacheを明示する
- 決定: 外部から観測できるcache契約は`specs/ond/build/cache.md`へ集約し、reuse/invalidation、生成物と診断の整合、broken cache再生成、explicit broken link input error、保存先CLIを記す
  - process内data structureはsource管理とし、通常/test source visibilityはlanguage/test-build章に置く
- 理由: 利用者が観測する永続Object cache動作を一箇所で把握でき、process内semantic cache実装を外部契約から分ける
- 承認根拠: 2026-10-20のユーザー決裁Q08=A
- 移行結果: 対応する概念章・要件索引へ反映し、[照合記録](./migration-record.md)で旧新対応を追跡する

### SPEC-Q09 — 既存適合要件IDの移行対応方式

- status: resolved
- 種別: 適合要件の識別
- 証拠: [適合チェックリスト](../../specs/ond/conformance/requirements.md)は要件IDから定義元を参照し、[適合case契約](../../specs/ond/conformance/cases/README.md)は仕様を根拠に実装評価結果から独立した期待値を定める
- 決定: 既存`ond-spec-id`とSYN/NUM等の要件IDを維持し、規範definition refsを移行先へ更新する
  - 規範規則・要件ID・case IDを相互参照し、期待値は仕様から導出し実装評価結果から独立して記す
  - Rust runnerはsource管理とする
- 理由: 利用中のIDによる追跡を保ち、仕様適合の期待値を実装評価から独立して再現できる
- 承認根拠: 2026-10-20のユーザー決裁Q09=A
- 移行結果: 対応する概念章・要件索引へ反映し、[照合記録](./migration-record.md)で旧新対応を追跡する

## 移行着手条件

SPEC-Q01〜SPEC-Q09の採用規則、保証範囲、根拠、承認が確定し、全9項目がresolvedであるため、本移行着手の判断ゲートを通過した。\
規範本文・移行先文書の作成、条文対応、version/cache/conformance schemaの照合は本移行作業として実施する。\
この決裁記録が示す範囲は、編集方針と各論点の採用規則・保証範囲である。\
規範本文の移行と条文対応は[照合記録](./migration-record.md)へ記録した。
