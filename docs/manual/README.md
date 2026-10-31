# ユーザー向けドキュメント

Ondは、プログラムを実行環境の違いから切り離すことを目指しています。

まず[Ondの思想](getting-started/overview.md)でCPU・機器・プログラムの関係を読み、[最初のプログラム](getting-started/first-program.md)へ進んでください。\
ビルド後の準備は[ビルドと起動の手順](getting-started/build-and-launch.md)で確認できます。

この案内はOnd 0.4.0を対象とします。

仕様の一般規則は[Specs](../../specs/README.md)を参照してください。

## はじめに

- [Ondの思想](getting-started/overview.md) — CPU・機器・プログラミング言語の役割と、Ondが目指すことを説明します
- [インストール](getting-started/installation.md) — repositoryから0.4.0向けCLIをbuildします
- [最初のプログラム](getting-started/first-program.md) — 小さなプログラムを作り、実行fileを生成します
- [ビルドと起動の手順](getting-started/build-and-launch.md) — 生成物と実行開始までの準備を説明します

## 言語ツアー

- 値
  - [整数](language-tour/values/integers.md)
  - [f32](language-tour/values/floats.md)
  - [真偽値](language-tour/values/booleans.md)
  - [文字列リテラル](language-tour/values/string-literals.md)
  - [nil](language-tour/values/nil.md)
- ソース
  - [コメント](language-tour/source/comments.md)
  - [識別子](language-tour/source/identifiers.md)
  - [改行](language-tour/source/newlines.md)
  - [セミコロン](language-tour/source/semicolons.md)
- 宣言
  - [変数 var](language-tour/declarations/variables.md)
  - [短縮変数宣言 :=](language-tour/declarations/short-declarations.md)
  - [定数 const](language-tour/declarations/constants.md)
  - [型定義 type](language-tour/declarations/type-definitions.md)
- 演算
  - 算術: [加算 +](language-tour/operators/arithmetic/addition.md)、[減算 -](language-tour/operators/arithmetic/subtraction.md)、[乗算 *](language-tour/operators/arithmetic/multiplication.md)、[除算 /](language-tour/operators/arithmetic/division.md)、[剰余 %](language-tour/operators/arithmetic/remainder.md)、[単項 +](language-tour/operators/arithmetic/unary-plus.md)、[単項 -](language-tour/operators/arithmetic/unary-minus.md)
  - 比較: [==](language-tour/operators/comparison/equal.md)、[!=](language-tour/operators/comparison/not-equal.md)、[<](language-tour/operators/comparison/less.md)、[<=](language-tour/operators/comparison/less-equal.md)、[>](language-tour/operators/comparison/greater.md)、[>=](language-tour/operators/comparison/greater-equal.md)
  - 論理: [!](language-tour/operators/logical/not.md)、[&&](language-tour/operators/logical/and.md)、[||](language-tour/operators/logical/or.md)
  - bit演算: [&](language-tour/operators/bitwise/and.md)、[|](language-tour/operators/bitwise/or.md)、[^](language-tour/operators/bitwise/xor.md)、[単項 ^](language-tour/operators/bitwise/not.md)、[&^](language-tour/operators/bitwise/and-not.md)、[<<](language-tour/operators/bitwise/left-shift.md)、[>>](language-tour/operators/bitwise/right-shift.md)
  - [型変換 as](language-tour/operators/type-conversion.md)
- 代入: [=](language-tour/assignment/assignment.md)、[複合代入](language-tour/assignment/compound-assignment.md)、[++](language-tour/assignment/increment.md)、[--](language-tour/assignment/decrement.md)
- 制御フロー: [if](language-tour/control-flow/if.md)、[for](language-tour/control-flow/for.md)、[break](language-tour/control-flow/break.md)、[continue](language-tour/control-flow/continue.md)、[defer](language-tour/control-flow/defer.md)、[trap](language-tour/control-flow/trap.md)
- 関数: [宣言](language-tour/functions/declarations.md)、[呼び出し](language-tour/functions/calls.md)、[return](language-tour/functions/return.md)、[複数戻り値](language-tour/functions/multiple-results.md)、[関数型](language-tour/functions/types.md)、[関数ポインター](language-tour/functions/pointers.md)、[メソッド](language-tour/functions/methods.md)
- データ構造: [配列](language-tour/data/arrays.md)、[struct](language-tour/data/structs.md)、[field access](language-tour/data/field-access.md)、[index access](language-tour/data/index-access.md)
- ジェネリック: [関数](language-tour/generics/functions.md)、[型](language-tour/generics/types.md)
- メモリ: [pointer型](language-tour/memory/pointers.md)、[address-of &](language-tour/memory/address-of.md)、[dereference *](language-tour/memory/dereference.md)、[alloc](language-tour/memory/alloc.md)、[new](language-tour/memory/new.md)、[free](language-tour/memory/free.md)
- 組み込み: [len](language-tour/builtins/len.md)、[sizeof](language-tour/builtins/sizeof.md)、[alignof](language-tour/builtins/alignof.md)、[thisFile](language-tour/builtins/this-file.md)、[thisLine](language-tour/builtins/this-line.md)
- パッケージ: [package](language-tour/packages/package.md)、[import](language-tour/packages/import.md)、[scope](language-tour/packages/scope.md)、[visibility](language-tour/packages/visibility.md)
- 構成: [role](language-tour/configuration/roles.md)
- 初期化: [global initialization](language-tour/initialization/globals.md)、[main](language-tour/initialization/main.md)
- operator定義: [算術](language-tour/operator-definitions/arithmetic.md)、[index getter](language-tour/operator-definitions/index-getter.md)、[index setter](language-tour/operator-definitions/index-setter.md)、[len](language-tour/operator-definitions/len.md)
- 低水準: [asm（Kagura v1）](language-tour/low-level/asm.md)

## 実用チュートリアル

- [projectとpackage](tutorials/projects-and-packages.md)
- [libraryと依存](tutorials/libraries-and-dependencies.md)
- [roleで構成を選ぶ](tutorials/roles.md)
- [診断を読み、修正する](tutorials/diagnostics.md)

## リファレンス

- [`ond.toml`リファレンス](reference/manifest.md)
