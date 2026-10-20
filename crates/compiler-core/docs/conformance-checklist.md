# compiler-core 規範要件ID

各適合caseは、ここに示す要件IDと対応する仕様を参照する。\
要件の詳細な定義は、各節に示す仕様を正とする。

## 字句・構文

詳細: [Syntax v1](../../../specs/ond/v1/syntax.md)、[Language v1](../../../specs/ond/v1/language.md)

| ID | 規範要件 |
| --- | --- |
| SYN-01 | Unicode Letter/Nd識別子、ASCII限定公開判定、BOM/NUL |
| SYN-02 | UTF-8ファイル、LF/CRLF、空白、コメント |
| SYN-03 | 2/8/10/16進整数とunderscore制約 |
| SYN-04 | decimal/hex float字句、直接binary32丸め |
| SYN-05 | quoted/raw string、escape範囲、CR・LF |
| SYN-06 | semicolon挿入・EOF・閉じ括弧前省略 |
| SYN-07 | 演算子優先順位7段階、同順位左結合、postfix as |
| SYN-08 | grouped const/var/type、空文、末尾comma |
| SYN-09 | call以外の式文、if/for clauseの文種制限 |
| SYN-10 | 未記載構文を受理しない |
| SYN-11 | 関数型の名前省略・記載・混在、宣言は名前必須 |
| SYN-12 | EBNFと本文の整合 |

## package・名前解決・宣言

詳細: [Syntax v1](../../../specs/ond/v1/syntax.md)、[Language v1](../../../specs/ond/v1/language.md)、[パッケージbuild仕様](../../../specs/ond/package-build.md)

| ID | 規範要件 |
| --- | --- |
| PKG-01 | 全projectでmanifestの有効なnameが必須、実行compileではmain.ondとfunc mainが必須、directoryとpackage名一致 |
| PKG-02 | 相対import、root import禁止、存在しないimport、循環 |
| PKG-03 | import名のfile scope・同path重複・fileごとのalias |
| PKG-04 | 公開/非公開のtype・const・global・function |
| NAME-01 | package/local/parameterの重複宣言 |
| NAME-02 | scope開始点、block/if/for scope、shadow |
| NAME-03 | 組み込み型名・組み込み名のbinding禁止 |
| DECL-01 | 初期値からの型推論、型付きzero、初期値なし型必須 |
| DECL-02 | const適格性、aggregate/typed nil、cycle |
| DECL-03 | blank identifierの破棄、binding/storageなし |
| DECL-04 | :=の新規名必須・現blockの変数だけ再利用 |

## 型・変換・nil

詳細: [Language v1](../../../specs/ond/v1/language.md)

| ID | 規範要件 |
| --- | --- |
| TYPE-01 | 構造型同一性、位置独立、非公開fieldのpackage区別 |
| TYPE-02 | defined型の区別、同じunderlyingへの明示変換 |
| TYPE-03 | 配列長・参照先・関数parameter/result列の一致 |
| TYPE-04 | 直接再帰拒否、pointer経由再帰受理 |
| CAST-01 | 型不一致の暗黙変換拒否 |
| CAST-02 | 定数整数変換・実行時wrap変換 |
| CAST-03 | int↔f32、ties-even/trunc、NaN/Inf/範囲外 |
| NIL-01 | pointer/function型nil、既定型なし、0とは別 |
| NIL-02 | nil function pointer call |

## 数値・演算・評価順序

詳細: [Syntax v1](../../../specs/ond/v1/syntax.md)、[Language v1](../../../specs/ond/v1/language.md)

| ID | 規範要件 |
| --- | --- |
| NUM-01 | literal既定型/型文脈と整数6型の範囲 |
| NUM-02 | arbitrary precision定数、最後にrange check |
| NUM-03 | 定数shiftのu32 count、k mod N、左shift非wrap |
| NUM-04 | 定数/動的div・rem、符号、0、min/-1 |
| NUM-05 | runtime wrap・bitwise・signed/unsigned比較 |
| FP-01 | constant binary32各演算、signed zero/Inf/NaN |
| FP-02 | constant/runtimeのf32 bit一致 |
| OP-01 | 許可operator×operand型、結果型、禁止型 |
| EVAL-01 | &&/\|\|短絡、callee→引数の左から右 |
| EVAL-02 | 一般operand、代入LHSのindex/pointer→RHS→書込順 |

## 配列・struct・文字列・メモリ

詳細: [Language v1](../../../specs/ond/v1/language.md)、[Built-ins](../../../specs/ond/v1/builtins.md)、[Kagura machine](../../../specs/ond/targets/kagura-v1/machine.md)

| ID | 規範要件 |
| --- | --- |
| AGG-01 | 固定長・定数長・推論長、key/穴/重複 |
| AGG-02 | field名付きliteral、nested型明記、非addressable |
| AGG-03 | aggregate値コピーとsource順・1回評価 |
| BOUND-01 | arrayの定数境界拒否、動的bounds check |
| PTR-01 | &local/field/index、deref、raw index u32/unchecked |
| STR-01 | UTF-8・emoji・任意byte escapeとbyte len、raw string |
| STR-02 | `[N]u8`型、完全長一致、copy/mutation/addressability、pointer非変換 |
| LEN-01 | len(array)は定数u32を返しoperandを1回評価 |
| BI-01 | alloc/free/newの型・arity・戻り値 |
| BI-02 | lenの型・arity、builtin名のbinding禁止、`str`は通常identifier |
| BI-03 | thisFile/thisLineの型・arity・論理位置 |
| MEM-01 | new zero/alignment、alloc failure/0、free nil等 |
| MEM-02 | stack/startup配置、escapeしても暗黙heap化なし |
| MEM-03 | bounds check失敗のnon-return/fault契約 |
| MEM-04 | lifetime/OOB raw/double freeなど検査しない領域 |

## 制御フロー・関数・起動

詳細: [Syntax v1](../../../specs/ond/v1/syntax.md)、[Language v1](../../../specs/ond/v1/language.md)、[Kagura ABI](../../../specs/ond/targets/kagura-v1/abi.md)

| ID | 規範要件 |
| --- | --- |
| FLOW-01 | if/else-if、for3形式、最内break/continue |
| FLOW-02 | bool条件、必須block、loop外break/continue拒否 |
| FLOW-03 | 全経路return・終端loopの判定 |
| CALL-01 | 0/1/複数戻り値、展開位置・個数・型 |
| CALL-02 | direct/indirect、forward/import/mutual recursion |
| INIT-01 | file辞書順/import DFS/宣言順/各package1回 |
| INIT-02 | 全global事前zero、内部init、推論/参照順独立 |
| INIT-03 | mainちょうど1つ/引数戻り値なし、init禁止 |
| OUT-01 | SDKによるmain終了処理、ABI/rodata/layout/static link |
| OUT-02 | assetは言語機能外、非source fileの暗黙同梱なし |

## 診断・MIR検証

詳細: [Language v1](../../../specs/ond/v1/language.md)、[MIR backend仕様](../../../specs/ond/targets/kagura-v1/mir-backend.md)、[MIR内部契約](./mir.md)

| ID | 規範要件 |
| --- | --- |
| DIAG-01 | source errorでFrontendError、部分的成功を返さない |
| DIAG-02 | Unicode/複数fileの診断位置 |
| MIR-01 | ID/type/op/dominance/guard/global/initの不正IR拒否 |
