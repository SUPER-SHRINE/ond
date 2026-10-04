# 変更履歴

このファイルには、Ondの利用者に影響する主な変更を記録する。

## 未リリース

## 0.1.3 - 未リリース

### Added

- Windows／Linux配布物に、同梱binaryの第三者ライセンス情報を追加。
- Release配布時にcredential／秘密鍵ファイルをGit管理対象から除外。

### Changed

- Windows／Linux配布archiveとLanguage Server、全workspace packageのversionを0.1.3へ更新。

## 0.1.2 - 移行前の履歴

### Added

- マシン機能の非規範設計案とロードマップ6文書を、Windows／Linux配布物の`docs/design/`へ収録。
- Windows／Linux配布archiveとLSPのSHA-256、size、Git commitを記録するcanonical manifest v1。
- descriptorと配布物を照合するmanifest生成・検証scriptとfixture回帰検査。

### Changed

- 同じReleaseの再実行では同一hashのassetを再利用し、異なるassetの上書きを停止する。
- native smoke testでLSPが返すversionをworkspace versionと照合する。

## 0.1.1 - 移行前の履歴

### Added

- Linux x86_64（glibc）向けの`ond`、`ond-lsp`とtar.gz／SHA-256配布経路。
- Windows／Ubuntu 22.04の品質検査・release smoke testと対象別配布物の集約。
- ローカルでも利用できる配布archive生成script。

### Changed

- developに含まれるOndロゴ更新を収録。
- Git tagとworkspace version、mainへの到達性を検証してからreleaseを作成。


## 0.1.0 - 移行前の履歴

最初の開発者向けリリース。

### Added

- Ond v1の構文、意味論、組み込み機能に関する規範仕様と利用者向けマニュアル。
- target非依存のfrontend、型検査、HIR、MIR、validator。
- Kagura v1向けMIR backend、Object形式、linker、LinkedImage形式。
- 複数package、path依存、Git依存、lockfile、差分build cacheに対応した`ond` CLI。
- 補完、hover、定義参照、参照検索、rename、semantic tokens、診断を提供する`ond-lsp`。
- compiler-coreとKagura backendの適合テスト、およびCLIとLanguage Serverの回帰テスト。
- generic型のpointer receiver methodと、型argumentを持つ名前付きinterface。generic interfaceを所有するpackageでは、公開methodへ委譲するgeneric operatorも宣言できる。
- `trap "reason"`と、fault分類・source mapping・stack trace用unwind情報を持つ`.onddebug` sidecar。

### Changed

- 0.1.0の公開準備として、対応環境、配布方法、既知の制約、再現性の基準を明文化した。
- compilerは同じ関数本体の独立した複数エラーを収集し、型不一致に期待型・実際型と明示interface変換の案内を表示する。generic本体の未定義名は未使用でも定義位置で検出し、具体化依存のエラーは定義を主位置、使用箇所を関連位置として報告する。
- Language Serverの補完、hover、rename、semantic tokensをgeneric method・operator・関数の返り値・不完入力へ広げた。

### Known limitations

- 言語、ABI、MIR、Object／LinkedImage、Rust公開APIの後方互換性は保証しない。
- 正式な検証環境はWindows x64とRust 1.91.1に限定する。
- ROM生成、asset配置、実行環境はEnbuなどの機種SDKから提供する。
- 差分buildのcache garbage collectionは未実装である。

0.1.0〜0.1.2の変更記録は移行前の履歴から引き継いだものです。対応するtagとReleaseはこのrepositoryへ移行していません。0.1.3も未リリースです。
