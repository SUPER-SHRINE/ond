# ローカル課題記録

1課題につき1つのMarkdownファイルを作成する。`queue`、`active`、`blocked`、`archive/YYYY-MM`の間を移動しても、同じファイルを使い続ける。

課題のタイトル、本文、判断履歴、完了記録は日本語で記述する。コマンド、識別子、パス、原文エラーは改変せず引用できる。frontmatterのキーと列挙値は、機械処理との互換性のため以下の英語表記を維持する。

## 必須frontmatter

```yaml
---
id: ONDQ-YYYYMMDD-NNN
title: 問題を具体的に表す短い日本語タイトル
status: queued
severity: P0 | P1 | P2 | P3
confidence: high | medium | low
risk: low | medium | high
area: compiler-core | compiler | cli | lsp | protocol | interface | docs | tooling
detector: quality-command | recent-change-review | specification-review | manual
base_commit: 完全なコミットID
created_at: ISO-8601形式の日時
attempts: 0
---
```

- `P0`: データ消失、セキュリティ上の影響、または出力が広範囲で利用不能になる問題。
- `P1`: 正しさに関する確認済みの不具合。
- `P2`: 影響範囲が限定された不具合、または重要な保守性の問題。
- `P3`: 軽微な改善。

重要度が高くても、自動修正のリスク境界を越えてはならない。

## 必須セクション

```markdown
## 根拠

正確な失敗内容、コマンド出力の要約、またはソースと仕様の矛盾。

## 再現手順

最小かつ決定的な手順。再現できない場合は、その理由を記載する。

## 期待される動作

守るべき観測可能な不変条件。

## 修正範囲案

変更が見込まれるファイルと、許容できる最小の変更。動かしてはならない境界も列挙する。

## 試行履歴

- 日時、実施内容、結果、判断理由。

## 解決内容

根本原因、変更ファイル、対象検証と全体検証、コミットID、採用しなかった案と理由。
```

## 状態遷移

- `queue`: 根拠を確認し、重複を除外済み。まだ変更には着手していない。
- `active`: 現在の実行で選択された唯一の課題。
- `blocked`: ユーザーの判断、追加権限、外部の変更、または2回の失敗後の支援が必要。
- `archive/YYYY-MM`: 修正と検証が完了した、または変更不要として理由を残して終了した。

ファイルを移動するたびに`status`を更新する。検証結果とコミットIDがない修正済み課題をアーカイブしない。変更せず終了する場合は、課題を無効または不要と判断した根拠を残す。
