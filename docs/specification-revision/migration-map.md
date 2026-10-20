# 目次・移行表

本表はcommit `06e87c39d99f2389af163d0394d9808f2c3078bd`の文書を対象にした、仕様の旧定義元と移行後の定義元の対応である。\
本移行で概念別の規範章へ整理し、外部契約抽出後の旧文書を削除した。\
詳細は[ID別照合記録](./migration-record.md)に記す。

編集原則は[仕様書の編集方針](./editorial-policy.md)、決裁済みの方針と移行作業は[未決定事項一覧](./open-questions.md)を参照する。\
整数節の文体例は[整数演算と定数評価の改稿見本](./integer-arithmetic-sample.md)に置く。

## 移行後の外部仕様の目次

```text
specs/
  README.md                         仕様入口
  ond/
    language/v1/
      README.md                     言語版と適用範囲
      lexical.md                    字句、改行、EBNFの字句部分
      names-and-scopes.md           package/import、名前解決、scope
      types.md                      型、型同一性、型parameter、layout参照
      declarations.md               const/var/type、関数とoperator宣言
      expressions.md                式、演算、定数評価、評価順序、文字列値
      statements.md                 文、制御構文、defer
      builtins.md                   builtin関数・構文
      execution.md                  言語上のmemory、初期化、entry契約
    targets/kagura-v1/
      README.md                     target profileの入口
      machine.md                    machine、layout、runtime、pointer、allocator
      abi.md                        ABI
    artifacts.md                    Object/LinkedImage等の成果物契約
    linking.md                      外部linking契約
    build/
      packages.md                   manifest、依存解決、source探索、compile単位
      commands.md                   compile/link CLI、入出力、失敗条件
      test-build.md                 test source、entry、実行契約
      cache.md                      外部から観測するcache契約
    debug-metadata/v1.md            debug metadataの外部契約
    conformance/
      requirements.md               要件IDと仕様定義元の索引
      cases/                         仕様から導出した入力・期待値
docs/
  decisions/                         承認済み判断の理由
```

この目次は外部の言語・target・ABI・成果物・linking・CLI契約と適合要件を整理した構成である。\
crateの内部構造、MIRの型やphase、内部helper、アルゴリズムの詳細はRust/Ondソースを実装の管理元とする。\
`docs/decisions/`は承認済み判断の理由を記録し、外部適合契約は`specs/`で定義する。

## 言語仕様の節単位対応

| 現行文書・節 | 移行先 | 外部仕様として整理する内容 | 決定・移行ID |
| --- | --- | --- | --- |
| `specs/ond/v1/README.md`とSemantic ID | `specs/ond/language/v1/README.md` | 言語版、適用範囲、既存IDの追跡 | — |
| `language.md` §1 ソースとパッケージ | `names-and-scopes.md`、`declarations.md` | packageと宣言の成立条件 | SPEC-Q07 |
| §2 宣言 | `declarations.md` | const/var/typeの構文、型、初期値と結果 | — |
| §3 型、§3.1 型parameter、型同一性 | `types.md` | 型集合、同一性、conversion、layout委譲 | — |
| §4 `nil` とポインタ | `types.md`、`execution.md` | pointer値、dereference、target access契約 | — |
| §5 配列とstruct、§5.1 Method | `types.md`、`expressions.md` | aggregate値、copy、method呼出しと評価順 | — |
| §6 式、Integer Arithmetic、Integer Constant Arithmetic、Floating-Point Arithmetic | `expressions.md` | 単項/二項/shift/比較のoperand・result型、数学的正確値と値要求境界のrange check | SPEC-Q01, SPEC-Q02 |
| §7 文字列リテラル | `expressions.md` | byte配列値、literal評価、storage参照 | — |
| §8 文と制御構文、§8.1 `defer` | `statements.md` | 構文、成立条件、評価経路、終了結果 | — |
| §9 関数、§9.1 User-defined operators | `declarations.md`、`expressions.md` | signature、呼出し、operatorの外部動作 | — |
| §10 メモリ管理 | `execution.md`、Kagura `machine.md` | allocation結果・alignment・存続中領域の分離、new zero initialization、alloc領域、failure・overflow・zero-size・nil・free | SPEC-Q04 |
| §11 初期化とエントリ | `execution.md` | global初期化順、通常entry、test build参照 | NOTE-INIT-01 |
| §12 Build Output | `artifacts.md`、`linking.md` | 外部成果物とlinking契約への参照 | SPEC-Q06, SPEC-Q07 |
| `syntax.md` §1 Source Form | `lexical.md` | Unicode 16.0.0に基づく識別子categoryと字句tokenをOnd本文で定義。Go仕様は規則の由来を示す参考資料 | SPEC-Q03 |
| §2 Semicolons and Line Breaking | `lexical.md` | newlineとsemicolon insertion | — |
| §3 Packages and Imports | `names-and-scopes.md` | package/importの名前と解決結果 | — |
| §4 Scope and Shadowing | `names-and-scopes.md` | bindingのscopeと可視性 | — |
| §5 Operator Precedence | `expressions.md` | precedenceと結合順 | — |
| §6 EBNF | 対応する概念章 | 各構文の定義元となる章 | — |
| §7 Grammar Notes | `declarations.md`、`expressions.md`、`statements.md` | 構文ごとの静的成立条件と意味 | — |
| `builtins.md` §1–§11 | `builtins.md` | builtin形式、入力、結果、異常時、委譲 | — |

## build、target、実装資料の対応

| 現行資料 | 移行後の整理先 | 分類と保持する内容 | 決定・移行ID |
| --- | --- | --- | --- |
| `specs/README.md` | 同じ入口ファイル | 外部規範と検討資料の案内 | — |
| `specs/ond/package-build.md`「責務」「コンパイル単位と成果物」 | `specs/ond/build/packages.md` | manifest、依存解決、source探索、compile単位の外部契約と版識別依存表 | SPEC-Q07 |
| 「コマンド」「ond compile」「ond link」「入出力と失敗時」 | `specs/ond/build/commands.md` | CLI入力、出力、終了結果の外部契約 | SPEC-Q06, SPEC-Q07 |
| 「成果物とキャッシュ」「検証」「公開情報（revision 3）」 | `artifacts.md`、`linking.md`、`build/cache.md` | 公開成果物/linking契約、版依存・互換表、cacheの外部契約。revision 3見出しと現行schema tupleを区別 | SPEC-Q06, SPEC-Q07, SPEC-Q08 |
| `specs/ond/test-build.md` | `specs/ond/build/test-build.md` | test source・entry・実行の外部契約 | — |
| `specs/ond/semantic-cache.md` | `specs/ond/build/cache.md`、process内data structureはsource管理 | reuse/invalidation、診断整合、broken cache再生成、explicit broken link input error、保存先CLIを外部契約として記す | SPEC-Q08 |
| `specs/ond/debug-metadata.md` | `specs/ond/debug-metadata/v1.md` | debug metadataの公開形式と利用結果 | — |
| `specs/ond/targets/kagura-v1/README.md` | 同名target入口 | Ond側Kagura profileの適用範囲 | — |
| `machine.md`のmachine/layout/pointer/runtime | 同名のmachine仕様 | Kagura上の値配置と外部観測されるruntime動作 | SPEC-Q04 |
| `machine.md`のheap allocator節 | 同名のmachine仕様、実装方式はRust/Ond source | 利用者から観測するallocation result/alignment/lifetime/failure契約を記す。first-fit/header/block操作方式は実装source管理 | SPEC-Q04 |
| `abi.md` | 同名ABI仕様 | call conventionと引数/戻り値の外部契約 | — |
| `mir-backend.md` | `expressions.md`、`execution.md`、`machine.md`、`abi.md`、`artifacts.md`、`linking.md` | expressions=値/演算/数値意味、execution=評価順/fault/startup init/entry/SDK、machine=layout/raw pointer/MMIO/zero-size、ABI=argument/result/register、artifacts/linking=section/symbol/relocation/CallSlot/JumpSlotを契約境界ごとに抽出 | SPEC-Q05 |
| `mir-backend.md`の内部lowering構造 | Rust/Ond source管理、sourceへ集約 | MIR構造、内部helper、命令選択・配置アルゴリズムの詳細 | SPEC-Q05 |
| `crates/compiler/docs/README.md` | 外部契約をspecsへ抽出、sourceへ集約 | compiler実装説明はRust/Ond sourceを管理元とする | SPEC-Q06 |
| `ond-ir.md` | MIR等の外部観測契約をspecsへ抽出、sourceへ集約 | AST/HIR/MIR構造とphase詳細はRust/Ond sourceを管理元とする | SPEC-Q05 |
| `ond-linker.md` | Object/LinkedImage/linking契約をspecsへ抽出、sourceへ集約 | 外部symbol/relocation/layout結果を規範化し、内部実装はsourceで管理 | SPEC-Q06 |
| `crates/compiler/src/linker/README.md` | 外部契約をspecsへ抽出、sourceへ集約 | validator構造、関数分割、patch algorithm詳細はRust sourceで管理 | SPEC-Q06 |
| `crates/compiler-core/docs/mir.md` | 外部の意味/fault要件をspecsへ抽出、sourceへ集約 | MIR形式はcompiler内部選択としてsourceで管理 | SPEC-Q05 |
| `conformance-checklist.md` | `specs/ond/conformance/requirements.md` | 既存要件IDを維持し、規範定義元とのreferenceを更新 | SPEC-Q09 |
| `conformance-cases.md`と`src/conformance/` | `specs/ond/conformance/cases/`とRust test harness | 規範規則・要件ID・case IDを相互参照し、入力・期待値は仕様から導出、runnerはRust sourceで管理 | SPEC-Q09 |

MIRはOnd言語の外部形式ではなく、compiler実装が選択する内部表現として扱う。\
Kagura v1の外部契約はMIR採用有無ではなく、specsに定める入力、観測動作、fault、成果物に対する適合で評価する。

## 移行で維持する構成

- process内意味解析cacheはsource管理、永続Object cacheは`build/cache.md`の外部契約として分ける
- debug metadata、test build、package artifacts、semantic cacheを個別の根拠を保って整理する
- language、target、ABI、artifact contract各版と相互依存・互換条件を対応表で示し、`compiler_id`は生成元/再利用識別子としてsemantic versionと分ける
- 適合要件とcase期待値は規範内容としてspecs側に置き、Rust test runnerやcrate内部型は実装sourceで管理する
- `docs/decisions/`には承認済み判断の理由を置き、本文の定義元を参照する
