# 仕様書再編の承認済み判断
<!-- ond-spec-id: decisions/specification-migration -->

この記録は2026-10-20のユーザー決裁に基づく仕様書の編集原則、外部契約の範囲、配置方針を記す。\
規範本文は`specs/ond`に置き、crate内部の設計説明はRust/Ond sourceを管理元とする。\
詳細な論点と承認根拠は[移行記録](../specification-revision/open-questions.md)を参照する。

## 決裁済みの原則
<!-- ond-spec-id: decisions/specification-editorial-principles -->

言語機能集合は本文に明示する構文、型、演算、動作、適合条件と、その規則から導出される集合として定義する。\
規範節は有効な入力、成立条件、意味と観測結果、失敗条件と結果、target委譲で構成する。\
実行動作と実装裁量は明示規則で定める。\
数学的正確値による定数計算、array/structの値copy、左辺値に応じた短絡評価経路などを肯定形で記述する。\
compile/link errorやruntime faultも条件に結び付く確定結果として記す。

## 決裁済み論点
<!-- ond-spec-id: decisions/specification-approved-questions -->

| ID | 決定 |
| --- | --- |
| SPEC-Q01 | integer unary/arithmetic/bitwise/shift/comparisonのoperandとresult型対応を表にする。比較resultはbool、defined typeは宣言固有identityを保つ |
| SPEC-Q02 | 整数const部分式を数学的正確値で計算し、宣言/代入/argument/return/明示castの値要求境界でrange checkする。固有境界の条件は個別規則で定める |
| SPEC-Q03 | Unicode 16.0.0分類を固定し、字句規則はOnd本文で定義する。Go specificationは由来を示す参考資料として扱う |
| SPEC-Q04 | allocator外部契約は確保領域のsize/alignment/存続allocationとの分離、new zero、alloc初期状態、failure/free/zero-sizeを保証し、first-fit/header/block操作はsource管理とする |
| SPEC-Q05 | MIR backend由来契約をlanguage、machine、ABI、artifact/linking、startupの利用境界へ抽出する。内部MIR形式と実装方式はsource管理とする |
| SPEC-Q06 | Object、LinkedImage、linkingの外部契約はspecsで定義する |
| SPEC-Q07 | 既存版識別単位を保ち、対象・版・依存/互換条件を仕様内の対応表で示す。compiler_idは生成元/再利用識別子とする |
| SPEC-Q08 | 外部cache契約を`specs/ond/build/cache.md`へ集約し、process内data structureをsource管理する |
| SPEC-Q09 | 既存ond-spec-id、requirement ID、case IDを維持し、規範・要件・caseを相互参照する。期待値は仕様から導出し実装結果から独立に定める |

## 移行後の管理境界
<!-- ond-spec-id: decisions/specification-source-boundaries -->

外部言語・target・ABI・Object/LinkedImage/linking/artifact/CLI・cache・debug・test build契約は`specs/ond`の章が定義する。\
Rust型、MIR形、phase、helper、validator構造、命令選択・探索アルゴリズムはsourceで管理する。\
既存MIR IDや内部case callbackは実装追跡の識別子として残し、MIR共通形式を将来のcompiler実装に要求しない。\
外部Objectが共有するCallSlot/JumpSlot 24-byte形式とscratch契約はlinking/ABIの外部規範に含める。

本移行のID別対応と適合データ照合は[移行記録](../specification-revision/migration-record.md)を参照する。
