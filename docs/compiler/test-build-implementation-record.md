# テストビルド実装記録

本書は規範的なテストビルド仕様から移した実装記録であり、現在の実装状態を保証するものではない。

## 実装状態
<!-- ond-spec-id: build/test-implementation-status -->

CLIの`ond compile --test`、test metadata、個別wrapper、`.ondtest`出力に対応した。
通常sourceとtest sourceの可視性は共通frontendで検証し、LSPもtest sourceを読み込み、補完・定義参照・hoverをsourceの文脈に合わせる。

caseの選択・machine stateの初期化・fault後の継続は実行ツール側の責務であり、compiler側の実装範囲に含まない。
sourceの実行時global initializerは既存仕様の対象外だが、linkerの各wrapperはpackage metadataにあるinitializerの依存closureを呼び出す。
