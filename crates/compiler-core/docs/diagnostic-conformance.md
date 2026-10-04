# P1 診断の期待値強化（DIAG-01/02）

「コンパイルが失敗した」だけではなく、期待する原因と位置で失敗したことを検証する。
安定した診断コードの導入ではなく、既存の診断メッセージ・severity・source spanに対するテスト契約を強化した。

## 既存テストの強化

[test_support.rs](../src/test_support.rs) の `reject` / `reject_at` は共通Case runnerへのアダプター。
次を検証する。

- コンパイル結果がFrontendErrorであり、部分的なCompilationを返さない。
- 診断は期待する1件のみ、severityはError。
- メッセージに期待する具体的な原因を含む。
- 正確なファイルとUTF-8 byte範囲が一致する。FileIdのロード順には依存しない。
- `internal MIR` エラーをsource errorとして受け入れない。

期待する文字列が複数ある場合、`reject` は最後の出現を使用し、`reject_at` は0始まりの出現番号を指定する。
失敗には呼び出し元のテストファイル・行と入力sourceを表示する。
これらは既存Rustテストのアダプターであり、static registryを一括移設してはいない。

対象はfunction type、type identity、constant value、inferred array、blank identifier/構文制約、Unicode公開規則・import pathの拒否ケース。
推論長配列では `unwrap_err` 後に診断の存在だけを確認していた14ケースも具体化した。

字句テストは原因・件数・severity・FileId・UTF-8境界内の非空spanを検査する。
字句テストの全ベクトルについて正確なstart/endを固定したわけではない。正確なbyte範囲の代表検証は下記DIAG-02で行う。
string decoder/f32変換/試験VMのエラーも具体的な原因を照合し、ハーネスや数値MIR検査器の負例も期待する検証失敗を確認する。

## 共通形式で追加した6ケース

[conformance/diagnostics.rs](../src/conformance/diagnostics.rs) に登録。

| case ID | 固定した内容 |
| --- | --- |
| DIAG-02.multifile-unicode-hir | mainより前に並ぶ別ファイル、CRLF・日本語・emoji後の型不一致。宣言と使用で同じ識別子が現れる場合の使用側span |
| DIAG-02.imported-package-body | import先の日本語ファイル名、4-byte文字の後にある未知識別子のHIR診断 |
| DIAG-01.multiple-independent-errors | 正常関数があっても成功しない。別ファイルの独立した2診断を順不同で一対一照合 |
| DIAG-01.invalid-global-no-partial-success | 正常main/関数と誤ったglobal初期値が共存してもFrontendError |
| DIAG-02.unicode-lexical-byte-range | CRLF・日本語後のemojiに対する字句診断の正確な4-byte範囲 |
| DIAG-02.unicode-parser-byte-range | CRLF・日本語・emoji後、区切り欠落の診断が2番目のvarを指す |

これらはメッセージ全体をExact照合する。追加のハーネステストでは、原因・出現番号・ファイル・件数を壊すと検証が失敗することも確認する。

## 発見した実装不一致

`type T [...]u8` で同じ診断が2件出ていた。defined typeの解決に失敗した後もitem/bodyのloweringへ進み、同じ型宣言を再解決していたため。
型定義の先行解決にエラーがある場合は、その段階でFrontendErrorを返すよう修正した。
無効な型に依存する後段の派生エラーは出さず、正常な型定義のもとでの独立したbodyエラーは従来どおり収集する。
このP1修正は、すべての依存関係で診断重複が存在しないことまで保証するものではなかった。

## 関数本体とgeneric診断の追補

2026-10-03の追補で、同じ関数本体の文ごとに回復し、独立した複数エラーを1回のcompileで収集するようにした。失敗した宣言が作る名前はpoisonし、その名前の後続利用から生じる派生診断は抑制する。これは「最初の1件で停止」でも、型の決まらない式を仮の型で続行する回復でもない。

generic関数・method・operatorでは次を固定した。

- 型argumentに依存しない値名とscopeは、宣言が未使用でも宣言時に検査する。
- 型argumentに依存する本体エラーは定義内をprimary spanとし、具体化を要求した箇所と入れ子の具体化chainをrelated labelにする。
- 同じ具体化の同じ定義エラーを複数の使用箇所が要求した場合は、1個の診断に使用箇所を併記する。別のgeneric宣言や別の失敗は続けて報告する。
- 代入・引数・returnの型不一致は`expected` / `found`の具体型を表示する。pointerからinterfaceへの暗黙変換には、`as InterfaceType`が必要であることも付記する。

回帰テストは`semantic_tests.rs`の同一本体・未使用generic・定義エラーの複数収集と、`operator_overload_tests.rs`のgeneric診断位置・複数具体化・interface変換診断を対象とする。

## 確認結果と範囲

- compiler-core/src内の `is_err()` だけのテストは解消。
- P1追加時はworkspace全419テスト成功（compiler-core311、ignoredなし）。その後の追補はrepositoryのFull品質検査で確認する。
- 型付きHIR/MIR生成までが対象。backendのruntime fault診断や全既存テストの診断全文固定は別の検証範囲。
