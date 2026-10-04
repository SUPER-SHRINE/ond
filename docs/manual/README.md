# Ond言語マニュアル

このマニュアルは、Ondでプログラムを書く人のための利用者向けガイドである。最初のプロジェクト作成から、言語機能、パッケージ構成、詳しい仕様の探し方までを、実用例を中心に説明する。

Ond v1の厳密な定義は[`specs/ond/v1`](../../specs/ond/v1/README.md)にある。このマニュアルと規範仕様に差がある場合は、規範仕様を正とする。

## 読み方

初めてOndを使う場合は、次の順に読むとよい。

1. [はじめに](getting-started.md) — コンパイラを用意し、最小のプロジェクトをコンパイルする
2. [言語ツアー](language-tour.md) — Ond v1の主要な言語機能を一周する
3. [プロジェクトとパッケージ](projects-and-packages.md) — 複数パッケージと依存関係を扱う
4. [言語リファレンス案内](reference.md) — 詳細な規則を目的別に調べる

すでに静的型付き言語に慣れている場合は、言語ツアーから始めてもよい。言語ツアーにはgeneric型・関数・method、generic interface、interfaceへ委譲するoperatorの例も含む。OndはGoに近い見た目を持つが、暗黙変換、slice、組み込み文字列型、garbage collectionは持たない。特にメモリ管理とinterfaceの規則は推測せず、該当章を確認してほしい。

## マニュアルと仕様の役割

| 文書 | 主な読者 | 役割 |
| --- | --- | --- |
| このマニュアル | Ondでプログラムを書く人 | 学習順、例、日常的な使い方 |
| [Ond v1仕様](../../specs/ond/v1/README.md) | 実装者、厳密な挙動を確認したい人 | 構文、意味論、組み込み機能の規範 |
| [Kagura v1 target仕様](../../specs/ond/targets/kagura-v1/README.md) | SDK・runtime・低レベルコードの実装者 | ABI、メモリ配置、実行形式 |
| [パッケージビルド契約](../../specs/ond/package-build.md) | build tool・SDKの実装者 | CLI、依存解決、成果物、cacheの厳密な契約 |

## 対象バージョン

このマニュアルはリポジトリ内のOnd v1と言語コンパイラを対象とする。機種固有の起動、asset同梱、ROM生成は言語機能ではなく、Enbuなどの機種SDKが担当する。
