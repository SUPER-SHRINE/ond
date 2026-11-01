# Ond Language v1
<!-- ond-spec-id: language/v1-index -->

本書群はOnd source language v1の規範定義元である。\
各章は以下の言語規則を定義する。

## 適用範囲
<!-- ond-spec-id: language/v1 -->

`ond`（音頭）は Kagura runtime を主 target とする静的型付き言語である。\
ソースファイルの拡張子は `.ond` とする。\
Kagura 上の具体的な実行契約は [Ond for Kagura v1](../../targets/kagura-v1/README.md) が定義する。

機能集合は、本文に明示した構文、型、演算、動作、適合条件と、それらから導出する機能の集合である。\
構文の成立には各章の静的条件を満たすことが必要で、違反した入力はcompile errorとなる。

- [字句とsource形式](./lexical.md)
- [package、import、scope](./names-and-scopes.md)
- [型と値構造](./types.md)
- [宣言と関数](./declarations.md)
- [式、演算、定数評価](./expressions.md)
- [文と制御構文](./statements.md)
- [Role構文と可視性](./roles.md)
- [Inline assembly](./inline-assembly.md)
- [builtin](./builtins.md)
- [memory、初期化、entry](./execution.md)

適合要件と入力・期待値の対応は[適合索引](../../conformance/requirements.md)、target、ABI、成果物の版依存は[互換表](../../artifacts.md#版識別と互換条件)を参照する。
