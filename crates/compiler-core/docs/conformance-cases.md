# 適合ケースの共通形式

## 目的と構成

[チェックリスト](./conformance-checklist.md)のspec IDと、自動実行されるケースを結び付ける。
既存の回帰テストはそのまま維持し、新規ケースからこの形式を利用できる。
test-onlyのRustデータとして定義するため、新しい依存ライブラリや外部データ形式のparserは不要。
production API・MIR・言語仕様の変更ではない。

- [mod.rs](../src/conformance/mod.rs): Case・Expected・Check・診断の共通型
- [runner.rs](../src/conformance/runner.rs): メタデータ検査、工程の実行、期待結果の照合
- [fixture.rs](../src/conformance/fixture.rs): 独立した一時projectの作成と後片付け
- [cases.rs](../src/conformance/cases.rs): 代表ケースのregistryと個別test関数
- [scope.rs](../src/conformance/scope.rs): project/package/import/main/initのケース
- [precedence.rs](../src/conformance/precedence.rs): 優先順位・結合性のASTとHIR定数値
- [constant_contexts.rs](../src/conformance/constant_contexts.rs): Q-03の適格性・型検査・未評価
- [evaluation.rs](../src/conformance/evaluation.rs)、[flow.rs](../src/conformance/flow.rs)、[name_duplicates.rs](../src/conformance/name_duplicates.rs)、[names.rs](../src/conformance/names.rs): P1の評価順・制御フロー・scope
- [control_headers.rs](../src/conformance/control_headers.rs): 空blockとcomposite literalの対照・実行。保留していた不具合を修正し通常実行へ移行
- [numeric/](../src/conformance/numeric/mod.rs): 数値表から共通Caseを生成。範囲・件数・検証層は[数値適合データ](./numeric-conformance.md)を参照
- [tests.rs](../src/conformance/tests.rs): 誤った期待値を受理しないことの検証

## 必須項目

| 項目 | 意味 |
| --- | --- |
| id | registry内で一意なcase ID。例: SYN-12.empty-results.reject |
| specs | チェックリストのID列。実在する行IDであることを検査 |
| layer | Parse / Core / MemoryVm。どこまで処理するかを明示 |
| manifest | trueなら空のond.tomlを作成、falseならmanifest欠落を検証 |
| files | project相対pathとUTF-8 source本文の組。複数file/packageを記述可能 |
| expected | Accept(checks)またはReject(diagnostics)。曖昧な「どちらでも可」はない |

ファイル名はスラッシュ区切りの相対.ond pathとする。重複（大小文字を無視）、絶対path、親directory参照、
Windows drive/ADS/device名等は一時projectを作る前に拒否する。manifestの有無を明示する。
filesは空でもよい（空projectの負例）。fixtureの作成・書き込み失敗は期待するコンパイラ診断に数えない。
各実行には固有のdirectoryを使い、成功・エラー・panicによる巻き戻し時に所有したdirectoryを削除する。

## 検証層

| layer | 実行するもの | 許可する成功時チェック |
| --- | --- | --- |
| Parse | loader → 全fileの字句・構文解析 | AST callback、または受理のみ |
| Core | loader → AST → HIR → MIR → project validator | AST、HIR定数の型/値、MIR callback、または受理のみ |
| MemoryVm | Core工程 → test-only memory_test_vmで実行 | MemoryExecutionが必須。AST/HIR/MIRチェックも併用可 |

CoreのRejectは、loader・字句・構文・意味検査のいずれで拒否されてもよいという工程指定。
期待する原因は診断文・位置で限定する。構文拒否を意味検査済みとみなす指定ではない。
Accept(&[])は受理だけの検証であり、値や命令の正しさを確認したことにはならない。
Core成功時は追加チェックの有無に関係なくvalidatorを通す。

MemoryVmは既存の限定的なMIR実行器を使う。実行前にcompiler-coreとvalidatorを通し、function/input/expectedを指定する。
実行チェックはMemoryVm層でだけ許可し、その層のAcceptには少なくとも1つの実行チェックを要求する。
各チェックは独立したglobal初期化状態から開始する。Kagura実行層は追加していない。
既存の簡易VMを数値全般の正解判定に流用しない。対応範囲と既知不具合は[制御フロー適合ケース](./control-flow-conformance.md)を参照。
loaderの診断も文言・件数・位置で照合し、manifest欠落をLocation::Projectで検証できる。
入力本文はRustのUTF-8文字列のため、不正UTF-8のfile自体はこの形式の対象外。
loader成功後のproject-level診断（main.ond欠落など）もLocation::Projectで検証する。

## 成功時の期待値

- Check::MemoryExecution: description、関数名、入力値、期待戻り値配列を指定。実行エラーや値の不一致は失敗とする。
- Check::Ast: AST形状をResult<(), String>を返すcallbackで検証。descriptionを必須フィールドとして記載する。
- Check::HirConstant: package論理path（rootは .）、定数名、期待型、期待値を明示する。
  値はInteger(u64 bits)、Float32(u32 bits)、Bool、String(bytes)、Nil。期待値はcompilerから逆算せず独立に記載する。
- Check::Mir: 関数・opcode・operand/result型・guard・順序などをcallbackで検証する。
  callbackは失敗理由をErrで返す。失敗時にdescriptionとcase/spec/layer情報が付く。

型名はTypeTable::displayの文字列表現を使う。primitive、defined型のcanonical name、配列、pointer等を記述できる。
匿名structの表示はTypeId依存なので、この文字列を固定しない。構造の同一性はMIR callbackでTypeTableを調べるなど、
IDの具体的な番号に依存しない検証を使う。aggregate値の汎用matcherはまだない。
signed整数の期待bitsは符号拡張されたtwo's-complement u64、f32はNaNやsigned zeroを区別できるbitsで指定する。

## 拒否時の期待診断

ExpectedDiagnosticでseverity、Message::ExactまたはContains、Locationを指定する。
現在は診断codeが存在しないため、原因の識別にはメッセージを用いる。

- 件数は完全一致。想定外の追加エラーを見逃さない。
- 診断順は問わないが、実際の診断と期待診断は1対1で対応させる。
- Source locationはfile、対象text、0始まりのoccurrenceで指定する。
  file内の該当部分のUTF-8 byte start/endがprimary spanと完全一致することを確認する。
  Unicodeを文字数として数えたり、FileIdと入力配列の順序が同じと仮定したりしない。
- EndOfFile locationは空fileを含むEOFのゼロ幅spanを、fileとbyte長で照合する。
- Project locationはsynthetic spanを明示的に期待するときだけ使用する。
- internal MIRエラーは、期待診断の文言に一致しても通常のsource rejectionとして受け入れない。
- spanを含めたsource情報は、コンパイル失敗時にも保持したLoadedProjectから解決する。

## 追加例

~~~rust
Case {
    id: "NUM-02.answer.core",
    specs: &["NUM-02"],
    layer: Layer::Core,
    manifest: true,
    files: &[SourceFile {
        path: "main.ond",
        text: "package main\nconst Answer = 20 + 22\nfunc main() {}\n",
    }],
    expected: Expected::Accept(&[Check::HirConstant {
        package: ".",
        name: "Answer",
        ty: "i32",
        value: Scalar::Integer(42),
    }]),
}
~~~

1. 機能別moduleのCASESへ登録し、個別のtest関数でrunを呼ぶ。新しいmoduleはregistry_is_validの結合対象にも追加する。
2. registry_is_validでID重複とspec ID/fixture/検証層の指定を検査する。
3. 新しい種類の期待値が必要ならCheckを拡張するか、明示したdescription付きcallbackを使う。
4. チェックリストの該当行にcase IDと実際に確認する層を追記する。受理のみで検証済へ格上げしない。

~~~text
cargo test -p ond-compiler-core conformance
cargo test -p ond-compiler-core conformance::cases::empty_results_rejected
cargo test --workspace
~~~

失敗メッセージにはcase ID、spec IDs、layerを付ける。入力はそのIDからregistryを参照できる。
既存の語彙/意味検査/VM回帰テストを一括移設する必要はない。

## 初期ケース

| case ID | 明示した検証 |
| --- | --- |
| SYN-11.unnamed-parameter.ast | 関数型の引数名がNoneであるAST |
| SYN-12.empty-results.reject | 空戻り値の構文拒否、診断文と右括弧span |
| NUM-02.imported-constant.core | 別file import、HIRの整数/bool/[3]u8/nil/f32型・値、MIR定数と演算除去 |
| NAME-03.unicode-multifile.reject | Unicodeコメント後の別fileでの予約型名拒否、正確なfile/span |
| PKG-01.missing-main-file.reject | project-level synthetic診断の明示照合 |

## P0 scope/grammarの追加ケース

- SYN-07: 七段階の隣接優先順位、各binary段階の左結合、unary・postfix連鎖をASTで照合。型の整合する式はHIR定数の型・値も照合。
- PKG-01: manifest/main.ond欠落、空project・空source、root/directory/package句の不一致。package句だけの宣言なしpackageは受理する（「空source」と区別）。
- PKG-02: root相対の入れ子import、root import拒否、未知/不正path、自己循環・三段循環。
- PKG-03: 各fileが自身のimportを宣言し、fileの辞書順に依存せず型・const・global・関数を参照。同pathの重複と、別fileの同aliasが異なるpathを指すケース。
- INIT-03: main欠落・重複・型名との衝突・不正signature、別packageのmain、initの引数/戻り値各組み合わせ。
- Q-03: 通常call/短絡側callの拒否、短絡算術の値非評価、配列len operandの1回評価、未評価部分の名前/型エラー、定数castの評価時範囲エラーと短絡内の非評価。計算した左辺・入れ子・評価される側との対照を含む25ケース。通常の代入・引数・return・globalについてはconstant_value_tests.rsでMIRと実行結果も確認する。

Q-01/02/04の包括的な決定反映テストはspecification_decision_tests.rsを引き続き利用する。
数値全型・全演算子の行列は後続P0で追補済（下記）。scope/grammarの成功だけを数値仕様全体の適合証明とはしない。

数値P0の追補は[numeric-conformance.md](./numeric-conformance.md)を参照。
共通型のCase/SourceFile/Expected/Locationは生成データの借用にも対応する。
固定registryは従来どおりstaticな定義を維持し、生成ケースと合わせてID重複を検査する。

P1型/値境界の1,366生成ケースは [type-boundary-conformance.md](./type-boundary-conformance.md) を参照。
`boundaries/` の4モジュールで同じCase runnerを使い、受理/拒否の対照と診断位置を検証する。
zero初期化のMIR値・格納先とaggregate定数のHIR値も確認する。件数はinventoryテストで固定する。

P1診断の強化は [diagnostic-conformance.md](./diagnostic-conformance.md) を参照。
`diagnostics.rs` にUnicode・複数fileの6ケースを登録し、既存Rustテストの拒否期待値も
`test_support::reject` / `reject_at` から共通runnerで検証する。
