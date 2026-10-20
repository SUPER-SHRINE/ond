# Numeric実装検査の追跡
<!-- ond-spec-id: conformance/cases/numeric-runtime -->

`FP-02.numeric-dynamic-mir-contracts`は内部MIRのopcode、型、guard形状に対する実装回帰IDである。\
この検査は外部言語適合のcaseではなく、他実装に同じIR構造を要求しない。\
数値演算と変換の外部観測は[整数実行vector](./kagura-integers.md)、[変換実行vector](./kagura-casts.md)、[f32 bit vector](./numeric-float-vectors.md)の入力・期待値で照合する。\
対応する規範定義元は[整数演算](../../language/v1/expressions.md#integer-arithmetic)、[f32演算](../../language/v1/expressions.md#floating-point-arithmetic)、[型変換](../../language/v1/types.md#型同一性)である。
