# P1 型・値境界の適合行列

対象: CAST-01、NIL-01、OP-01、BI-01/02、DECL-01/02。
実装は [conformance/boundaries](../src/conformance/boundaries/mod.rs) に機能別に分割する。
共通Case形式で **1,366ケース**を生成し、受理・拒否、spec ID、入力source、検証層、診断を固定する。
Rustのテスト関数数とは異なる（4つの行列実行テスト＋inventoryテスト）。

## 型の集合と行列

型の集合は `u8/i8/u16/i16/u32/i32/f32/bool/*u8/*i32/func()/[2]i32/S/Count` の14種類。
`S` は `struct { x: i32 }` のdefined type、`Count` は `i32` のdefined type。
期待受理は仕様から記述し、コンパイラの型判定関数から生成しない。

| モジュール | 件数 | 検証内容 |
| --- | ---: | --- |
| conversions | 866 | 14×14型対×代入/引数/return、明示cast、nil |
| operators | 322 | 14型×binary19種・unary4種の受理/拒否とreturnの結果型 |
| builtins | 117 | arity、引数位置ごとの14型、戻り値/void、new構文、defined array、source位置builtin |
| declarations | 61 | 型なし・初期値なし拒否、14型のzero/推論/後続異型代入、定数適格性・値 |

### CAST-01 / NIL-01

- 各型対について、型付きparameterを代入・引数・returnに使う。文脈に合わせて型が決まるliteralとは区別する。
- 同一型は受理。異型の暗黙変換は拒否し、defined整数型も同じunderlyingだからといって暗黙変換しない。
- 明示castは数値間、整数とpointer間、同一型を受理。bool/function/aggregateとの禁止変換、異なるpointeeへのcastを拒否する。
- nilを代入・引数・returnに使用。pointer/function以外への使用、型文脈なし、`nil == nil`、整数0の暗黙代入を拒否する。
- pointer/functionとnilの `==`/`!=` は左右両順序を受理。非nullable型との比較は両順序を拒否する。

### OP-01

- binary: `+ - * / % & | ^ &^ << >> == != < <= > >= && ||`。
- unary: `+ - ^ !`。アドレス取得・参照外しは既存memoryテストで別途検証する。
- shiftの右辺は `u32`。その他は同型の左右operandを使用し、比較/論理の結果はbool、算術の結果はoperand型としてreturnで検査する。
- pointer算術、array/structの比較・算術などを拒否する。
- 動的数値のopcode/guardは [数値表](./numeric-conformance.md) の検証を併用する。

### BI-01 / BI-02

- alloc/free/load32/store32/lenの誤arityは引数0〜3個から列挙。正しいarityの型境界は別表で検証する。
- store32のaddress/valueを別々に検査。len(array)のoperandも型検査し、通常どおり1回評価する。
- `new()`/余分な型引数、void戻り値の値としての使用を拒否。
- `new([4]u8)` を `*[4]u8` 戻り値として受理し、`*u8` への代入を拒否。alloc/load32/lenの結果型もreturnで確認する。
- defined arrayのlenを受理。`len`を含む組み込み名はbindingとして宣言できず、global関数やlocal/global/constでshadowできない。

### DECL-01 / DECL-02

- 14型で初期値からの推論と後続異型代入拒否を確認。
- zero初期化はMIRの型・定数値・localへのStore/AggregateCopyまで照合。globalの型と、zero-only宣言が追加initializerを生成しないことも確認する。
- typed nil定数、defined array/struct定数を受理し、function nilの型・値、array/structの要素値と省略要素zeroもHIRで照合。
- 通常call、非nil関数値、global値、外部load、alloc/new、globalのaddress、非定数要素入りaggregateを拒否する。
- 定数cycle・短絡側の非評価・len operandの1回評価・入れ子aggregateは既存CONST/SEM/Q-03テストを併用する。

## 診断と発見した不一致

拒否ケースは診断1件、メッセージの意味部分、source fileと正確なbyte spanを照合する。
受理ケースはすべてMIR validatorも通す。case IDは既存registry/数値行列との重複を検査し、inventoryで件数の意図しない減少を防ぐ。

今回修正した不一致:

1. `nil == p` / `nil != p` が左辺の型文脈不足で拒否されていた。右辺から型文脈を得つつ、HIRのoperand順序を維持する。
2. `Array{1, 2}` / `Array{1 + 1: 7}` のような名前付き配列literalを通常の式で解析できなかった。制御header外ではnamed literalとして解析し、array/structの要素制約は意味検査に任せる。従来の `if flag {}` / `for flag {}` テストも維持する。
3. 型も初期値もないlocal変数が汎用エラーになっていた。宣言箇所に具体的な型必須診断を出す。

## 検証範囲

これはcompiler-coreの型/値境界の検証であり、全ての再帰的な型の組み合わせや実行時動作の証明ではない。
allocatorの寿命・MMIO・CPU fault・数値castの実機結果・各型の物理メモリ上のzero表現はMIR backend工程で扱う。
このP1追加時のworkspaceテストは414件成功（compiler-core307件、ignoredなし）。
最新の再監査結果と残件は[チェックリスト](./conformance-checklist.md)を参照。
