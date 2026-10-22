# 診断の原因とgeneric関連位置
<!-- ond-spec-id: conformance/cases/diagnostic-causes -->

要件はDIAG-01/DIAG-02とし、[診断と成功結果](../../language/v1/declarations.md#診断と成功結果)、[generic診断](../../language/v1/types.md#型parameter)を適用する。共通のOnd v1、manifestとCore操作は[case契約](./README.md#入力と期待結果の読み方)に従う。各caseは独立projectであり、main.ond全文をUTF-8/LF/末尾LFありで作成する。比較対象はcompileの成否、原因位置、関連位置と独立原因の収集・同一具体化の集約とする。

## 独立したsource原因
<!-- ond-spec-id: conformance/diagnostic-causes/independent -->

Case IDはDIAG-01.independent-causesとする。

```ond
package main
func a(){var x:i32=true}
func b(){var y=missing}
func main(){}
```

Coreで失敗し、aの型不一致とbの未定義名の両方を原因として報告する。各主位置は当該不成立sourceに対応するUTF-8 byte範囲である。

## 同じ具体化と独立した別具体化
<!-- ond-spec-id: conformance/diagnostic-causes/generic-related -->

Case IDはDIAG-02.generic-related-causesとする。

```ond
package main
type S struct{}
func Need[T](x:T)->u32{return x.missing}
func Outer[T](x:T)->u32{return Need[T](x)}
func main(){_=Outer[S](S{});_=Need[S](S{});_=Need[u32](1)}
```

Coreで失敗し、Need[S]の同じ定義エラーを1個の診断へまとめ、主位置をNeed定義内のx.missingへ対応づける。main内のOuter[S]、Need[S]と、Outer定義内のNeed[T]という具体化要求を関連位置へ含める。Need[u32]の独立した具体化失敗も報告し、その主位置は同じ定義内原因、関連位置はmainのNeed[u32]要求に対応する。
