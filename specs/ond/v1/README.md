# Ond Language v1
<!-- ond-spec-id: language/v1-index -->

- [Syntax](./syntax.md)
- [Language Semantics](./language.md)
- [Built-in Functions](./builtins.md)

このディレクトリは Ond source language の規範仕様を収録する。Kagura 上での型表現、ABI、実行形式、host/device contract は [Ond for Kagura v1](../targets/kagura-v1/README.md) が定義する。

## Semantic ID
<!-- ond-spec-id: language/semantic-id-policy -->

`specs/ond` 配下の各見出しには、探索処理から安定して参照できる `ond-spec-id` を HTML コメントとして直後に置く。
見出し名やファイル配置を変更しても既存 ID は維持し、別の意味へ再利用しない。新しい見出しには一意な小文字 kebab-case のパスを割り当てる。
`tools/test-spec-ids.ps1` と共通品質チェックが、欠落、形式違反、重複を検出する。
