---
name: ond-quality-loop
description: Ondのローカル長期品質ループを実行・復旧し、根拠に基づく課題発見、制限付き修正、検証、ローカルアーカイブを行う。専用の自動品質保守に使用し、通常の機能開発や単発のコードレビューには使用しない。
---

# Ond品質ループ

`.ond-quality/`配下のローカル台帳を管理し、1回の実行で扱う課題を最大1件に限定する。Git管理するワークフロー定義と、ignoreされた運用状態を分離する。

課題を作成・更新する前に、[課題記録の形式](references/issue-format.md)を読む。

課題のタイトル・本文・判断理由・実行要約・完了報告・コミットメッセージは日本語を基本とする。機械処理用frontmatterのキーと列挙値、コマンド、識別子、原文エラーは英語のまま保持してよい。

## 実行開始と復旧

1. `pwsh -NoProfile -File tools/quality-loop.ps1 -Action Status`を実行する。
2. `git status --short --branch`を確認する。無関係な変更を上書きしない。
3. 中断されたロックが残っている場合は、期限切れのときだけ`-Action Recover`を使用する。`-Force`を使う前に、ロック、`.ond-quality/active/`、worktreeの差分を確認する。
4. `-Action Acquire`でリースを取得し、返されたrun IDを保持する。
5. 処理中の課題または未完了の差分がある場合は、新しい課題を選ぶ前に復旧する。所有者や意図を判断できない場合は課題を`blocked`へ移し、差分には触れない。

## 観測モード

`state.json`が`mode: observe`を示す間は、ソースやGit管理された文書を変更しない。

- `pwsh -NoProfile -File tools/quality.ps1 -Profile Full`を実行する。
- 失敗、`last_completed_commit`以降の変更、仕様との不一致、panicやエラーの経路、回帰テストの不足を調査する。
- 具体的な根拠または再現可能な失敗がある課題だけを、最大1件記録する。
- `queue`、`active`、`blocked`、`archive`を横断して重複を確認する。
- 課題を記録した場合は`observed`、問題がなかった場合は`no-issue`として実行を完了する。

観測実行が3回成功すると、ローカル状態は自動的に`guarded-fix`へ移行する。

## 制限付き修正モード

高確信・低リスクで、再現可能な課題だけを実装する。対象となる課題のうち、重要度が高く古いものを優先する。

1. Markdownファイルを`queue`から`active`へ移し、statusと試行履歴を更新する。
2. 報告された理由で失敗する回帰テストを追加するか、既存テストを特定する。
3. 根本原因を解消する最小の変更を行う。
4. 対象を絞った検証後に、`pwsh -NoProfile -File tools/quality.ps1 -Profile Full`を実行する。
5. 無関係な差分が含まれていないことを確認する。
6. 専用の`codex/quality-loop`ブランチで、その課題だけの日本語コミットを1件作成する。mergeやpushは行わない。
7. コミット、変更ファイル、検証結果、判断理由を課題に追記し、`archive/YYYY-MM/`へ移す。

検証が失敗した場合、課題が曖昧になった場合、または同じ課題の失敗が2回に達した場合は`blocked`へ移す。問題があると分かっているコミットを残さない。その課題に属すると記録されたファイルだけを戻し、無関係な作業を保持する。

## 厳守する境界

- 言語仕様、ABI、MIRの意味論、Object／LinkedImage形式、公開API、依存関係、リポジトリ横断の変更には、ユーザーの承認を求める。
- Kaguraの変更、ネットワーク利用、merge、push、共有履歴の書き換えを行わない。
- 検査を通すためにテスト、診断、`tools/clippy-baseline.json`を弱めない。
- `codex/quality-loop`で始まる専用ブランチ以外では、ソースの自動修正を行わない。
- 将来の実装判断に影響する恒久的な理由だけを、Git管理された設計判断文書へ昇格する。通常の実行履歴はローカルに保持する。

## 実行の完了

必ず次を呼び出す。

`pwsh -NoProfile -File tools/quality-loop.ps1 -Action Complete -RunId <id> -Outcome <outcome> -Summary <日本語の要約>`

ワークフロー自体を完了できなかった場合は`failed`を使用する。3回連続で失敗した場合は自動作業を停止し、ユーザーへ日本語で報告する。
