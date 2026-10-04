# 言語リファレンス案内

日常的な疑問から、対応する規範仕様へ移動するための索引である。

## 構文を調べる

- source文字、identifier、literal、comment: [Syntax §1](../../specs/ond/v1/syntax.md#1-source-form)
- 改行とsemicolon: [Syntax §2](../../specs/ond/v1/syntax.md#2-semicolons-and-line-breaking)
- packageとimport: [Syntax §3](../../specs/ond/v1/syntax.md#3-packages-and-imports)
- scopeとshadowing: [Syntax §4](../../specs/ond/v1/syntax.md#4-scope-and-shadowing)
- operator precedence: [Syntax §5](../../specs/ond/v1/syntax.md#5-operator-precedence)
- 完全なgrammar: [Syntax §6](../../specs/ond/v1/syntax.md#6-ebnf)

## 値と型を調べる

- `var`、`const`、`:=`: [Language §2](../../specs/ond/v1/language.md#2-宣言)
- 基本型、defined type: [Language §3](../../specs/ond/v1/language.md#3-型)
- generic型・関数・method: [Language §3.1](../../specs/ond/v1/language.md#31-型parameter)
- `nil`とraw pointer: [Language §4](../../specs/ond/v1/language.md#4-nil-とポインタ)
- array、struct、method: [Language §5](../../specs/ond/v1/language.md#5-配列と-struct)
- generic interfaceとinterface値: [Language §5.2](../../specs/ond/v1/language.md#52-interface)
- 式、演算、変換: [Language §6](../../specs/ond/v1/language.md#6-式)
- 文字列リテラル: [Language §7](../../specs/ond/v1/language.md#7-文字列リテラル)

## 処理の流れを調べる

- `if`、`for`、`break`、`continue`、`return`: [Language §8](../../specs/ond/v1/language.md#8-文と制御構文)
- `defer`: [Language §8.1](../../specs/ond/v1/language.md#81-defer)
- 関数、複数戻り値、function pointer: [Language §9](../../specs/ond/v1/language.md#9-関数)
- user-defined operator: [Language §9.1](../../specs/ond/v1/language.md#91-user-defined-operators)

## runtimeとbuildを調べる

- `new`、`alloc`、`free`とメモリ管理: [Language §10](../../specs/ond/v1/language.md#10-メモリ管理)
- `trap`とdebug sidecar・stack trace情報: [Debug Metadata](../../specs/ond/debug-metadata.md)
- entry functionとglobal初期化: [Language §11](../../specs/ond/v1/language.md#11-初期化とエントリ)
- 組み込み関数と組み込み構文: [Built-ins](../../specs/ond/v1/builtins.md)
- compile、link、依存解決、cache: [パッケージビルド契約](../../specs/ond/package-build.md)
- Kagura上のsize、alignment、fault、実行環境: [Machine Profile](../../specs/ond/targets/kagura-v1/machine.md)
- Kagura ABI: [ABI](../../specs/ond/targets/kagura-v1/abi.md)

## よく間違えやすい点

- 数値型やdefined typeの間に暗黙変換はない。
- 文字列リテラルは`[N]u8`であり、組み込みの`string`や`str`型はない。
- arrayとstructは値copyされる。
- raw pointerのindex、寿命、二重解放はruntime検査されない。
- method呼び出しで暗黙のaddress取得や参照外しは行わない。
- interfaceはpointerから明示変換して作り、boxingやtype assertionはない。
- genericの型argumentは明示する。ただしgeneric methodはreceiverから、generic operatorはoperandから型parameterが推論される。
- package-level initializerから通常の関数は呼べず、自動`init` hookもない。
- `ond compile`の出力は実行ファイルではなく、link前のmanifestである。
