# 検証IDと規範定義元
<!-- ond-spec-id: conformance/implementation-index -->

各検証IDの外部規則は、以下の同じ主題の規範定義元に対応する。\
外部の適合判定は[要件とcase](./requirements.md)の入力・観測結果を比較する。\
`MIR-01`等の実装検証の範囲と管理境界は[文書管理方針](../../../AGENTS.md#sourceとの管理境界)に従う。

## backend入口とtarget境界
<!-- ond-spec-id: targets/kagura-v1/mir-backend -->

外部規則は[定義元](../targets/kagura-v1/README.md)に従う。

## 検証済み入力とsource診断
<!-- ond-spec-id: targets/kagura-v1/mir-backend-io-and-responsibilities -->

外部規則は[定義元](../language/v1/declarations.md#診断と成功結果)に従う。

## 型配置の実装
<!-- ond-spec-id: targets/kagura-v1/mir-data-layout -->

外部規則は[定義元](../targets/kagura-v1/machine.md#5-data-layout)に従う。

## 引数・戻り値とframeの実装
<!-- ond-spec-id: targets/kagura-v1/mir-abi-lowering -->

外部規則は[定義元](../targets/kagura-v1/abi.md)に従う。

## 命令選択と演算実装
<!-- ond-spec-id: targets/kagura-v1/mir-instruction-lowering -->

外部規則は[定義元](../language/v1/expressions.md)に従う。

## global配置とstartupの実装
<!-- ond-spec-id: targets/kagura-v1/mir-intrinsic-global-startup -->

外部規則は[定義元](../language/v1/execution.md#初期化とエントリ)に従う。

## backend内部構成
<!-- ond-spec-id: targets/kagura-v1/mir-backend-architecture -->

外部規則は[定義元](../targets/kagura-v1/README.md)に従う。

## Object生成実装
<!-- ond-spec-id: targets/kagura-v1/mir-backend-object-contract -->

外部規則は[定義元](../artifacts.md#object)に従う。

## runtime helper供給と依存閉包
<!-- ond-spec-id: targets/kagura-v1/runtime-helper-supply -->

外部規則は[定義元](../targets/kagura-v1/abi.md)に従う。

## numeric helperとallocator方式
<!-- ond-spec-id: targets/kagura-v1/numeric-and-allocator-lowering -->

外部規則は[定義元](../targets/kagura-v1/machine.md#31-heap-allocator)に従う。

## 関数内branch拡張
<!-- ond-spec-id: targets/kagura-v1/function-local-branches -->

外部規則は[定義元](../targets/kagura-v1/abi.md#1-registers)に従う。

## relocation実装回帰
<!-- ond-spec-id: targets/kagura-v1/relocation-verification -->

外部規則は[定義元](../linking.md)に従う。

## compiler APIの接続
<!-- ond-spec-id: targets/kagura-v1/standard-path-responsibilities -->

外部規則は[定義元](../build/commands.md)に従う。

## 内部IRとcacheの管理元
<!-- ond-spec-id: conformance/internal-source-owners -->

内部実装の追跡範囲は[Sourceとの管理境界](../../../AGENTS.md#sourceとの管理境界)に定める。\
外部から観測する宣言受理と定数値は[宣言case](./cases/boundaries-declarations.md)と[定数case](./cases/static-seed.md)で比較する。

## 既存回帰と要件
<!-- ond-spec-id: conformance/existing-regression-requirements -->

各要件に対応する完全入力・操作・期待は以下の文書を定義元とする。\
回帰識別名は同じ主題を追跡する索引であり、各入力が指定する観測範囲を表す。

| 要件ID | 完全入力・操作・期待 |
| --- | --- |
| `SYN-01` | [lexical-observations](./cases/lexical-observations.md) |
| `SYN-02` | [lexical-observations](./cases/lexical-observations.md) |
| `SYN-03` | [lexical-observations](./cases/lexical-observations.md) |
| `SYN-04` | [lexical-observations](./cases/lexical-observations.md) |
| `SYN-05` | [lexical-observations](./cases/lexical-observations.md) |
| `SYN-06` | [lexical-observations](./cases/lexical-observations.md) |
| `SYN-08` | [regression-observations](./cases/regression-observations.md) |
| `SYN-09` | [regression-observations](./cases/regression-observations.md) |
| `SYN-10` | [regression-observations](./cases/regression-observations.md) |
| `PKG-04` | [regression-observations](./cases/regression-observations.md) |
| `DECL-03` | [regression-observations](./cases/regression-observations.md) |
| `DECL-04` | [regression-observations](./cases/regression-observations.md) |
| `TYPE-01` | [regression-observations](./cases/regression-observations.md) |
| `TYPE-02` | [regression-observations](./cases/regression-observations.md) |
| `TYPE-03` | [regression-observations](./cases/regression-observations.md) |
| `TYPE-04` | [regression-observations](./cases/regression-observations.md) |
| `NIL-02` | [kagura-evaluation-combinations](./cases/kagura-evaluation-combinations.md) |
| `AGG-01` | [regression-observations](./cases/regression-observations.md) |
| `AGG-02` | [regression-observations](./cases/regression-observations.md) |
| `AGG-03` | [regression-observations](./cases/regression-observations.md) |
| `BOUND-01` | [regression-observations](./cases/regression-observations.md) |
| `PTR-01` | [regression-observations](./cases/regression-observations.md) |
| `STR-02` | [regression-observations](./cases/regression-observations.md) |
| `LEN-01` | [regression-observations](./cases/regression-observations.md) |
| `MEM-01` | [kagura-regression-observations](./cases/kagura-regression-observations.md) |
| `MEM-02` | [memory-placement](./cases/memory-placement.md) |
| `MEM-03` | [kagura-regression-observations](./cases/kagura-regression-observations.md) |
| `MEM-04` | [regression-observations](./cases/regression-observations.md) |
| `FLOW-01` | [regression-observations](./cases/regression-observations.md) |
| `CALL-01` | [regression-observations](./cases/regression-observations.md) |
| `CALL-02` | [regression-observations](./cases/regression-observations.md) |
| `INIT-01` | [build-observations](./cases/build-observations.md) |
| `INIT-02` | [regression-observations](./cases/regression-observations.md) |
| `OUT-01` | [kagura-regression-observations](./cases/kagura-regression-observations.md) |
| `OUT-02` | [regression-observations](./cases/regression-observations.md) |
