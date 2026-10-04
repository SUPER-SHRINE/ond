# Ond MIR Design

本書は `ond-compiler-core` が生成する target-neutral MIR の設計を定義する。

仕様項目ごとの既存テスト・不足・追加優先順位は[仕様適合テスト・チェックリスト](./conformance-checklist.md)を参照。

MIR は Ond の言語 semantics を保持するが、Kagura を含む特定 target の命令、register、ABI、pointer width、section、relocation を含まない。MIR は compiler 内部形式であり、version 間の binary compatibility を保証しない。

## 1. Pipeline

compiler pipeline は次の層に分ける。

```text
Ond source
    |
    v
AST
    |
    v
HIR
    |
    v
MIR                 target-neutral CFG、非 SSA
    |
    v
SSA IR              target-neutral、最適化用
    |
    v
Target Machine IR   ABI lowering、命令選択、仮想 register
    |
    v
Object
```

MIR と SSA IR は別形式とする。MIR は mutable local と place を使う素直な lowering を優先し、SSA IR は scalar value の最適化を優先する。

## 2. Semantic policy

- source の左から右の評価順序を保持する。
- fault しうる operation の順序も観測可能とし、勝手に入れ替えない。
- raw pointer access はすべて ordered access とする。
- integer operation は operation ごとに overflow behavior を持つ。
- `f32` は既定で厳密な Ond の IEEE 754 semantics に従う。fast-math は将来の明示的な opt-in とする。
- `undef` と `poison` は持たない。Ond の zero value または明示的に定義された value だけを扱う。
- aggregate は原則として place 上に置き、scalar value だけを SSA へ昇格する。aggregate の scalar replacement は optimization として追加する。
- target ABI は MIR に含めない。parameter と return value は論理的な型付き列として表す。

## 3. Ownership

```text
Project
  Package*
    Function*
      Local*
      Value*
      BasicBlock*
        Instruction*
        Terminator
```

ID は function 内で有効とする。

- `LocalId`: mutable storage
- `ValueId`: immutable temporary value または block parameter
- `BlockId`: basic block
- `InstructionId`: instruction

各 ID は対応する table の index と一致する。compiler が deterministic な順序で割り当て、validator が一致を検査する。

## 4. Type model

MIR type は Ond の確定型を表す。

HIR と MIR は project 共通の `TypeTable` を所有し、各型を `TypeId` で参照する。primitive 型は固定 ID、derived type は deterministic に intern した ID、defined type は宣言時に予約した ID を使用する。型の構造を利用する処理は `TypeId` の同一性を保ったまま table から `TypeKind` を参照する。

structのintern/equality/hashはfieldの順序・名前・TypeId・非公開fieldの所属PackageIdで比較し、fieldのSpanは診断用metadataとして比較から除外する。公開fieldの所属はNoneとし、packageをまたぐ同一構造を共有できる。非公開fieldの可視性はSpanのfileではなく所属PackageIdで判定する。validatorは非公開fieldの所属とproject内packageの存在を検査する。配列・pointer・関数型も構成要素のTypeIdによってinternされ、defined typeの宣言ごとの区別は維持する。共有型のSpanは最初の記述元を保持し、各式・宣言の診断位置は個別のHIR/ASTのSpanを使う。

- `bool`
- `u8`, `i8`, `u16`, `i16`, `u32`, `i32`
- `f32`
- `Pointer<T>`
- function pointer
- `[N]T`
- struct ID
- defined type

pointer は型として保持するが、byte width と alignment は持たない。target data layout が Machine IR lowering 時に決定する。

defined type と underlying type の区別は MIR まで保持する。defined type node は underlying type の `TypeId` を参照し、pointer を介した recursive type も有限な型グラフとして表現する。型検査を終えた operation は underlying representation を参照できるが、暗黙に defined type を消去しない。

## 5. Function

`Function` は次の情報を持つ。

| Field | Meaning |
| --- | --- |
| `name` | canonical symbol name |
| `parameters` | parameter local ID 列 |
| `returns` | 論理 return type 列 |
| `locals` | mutable local storage |
| `values` | immutable value table |
| `blocks` | CFG block 列 |
| `entry` | entry block |
| `span` | source origin |

source local、parameter、addressable temporary は `Local` とする。演算結果は `Value` とする。

## 6. Basic block and CFG

各 basic block は instruction 列と 1 個の terminator を持つ。source-level `if`、`for`、`break`、`continue`、`&&`、`||` は MIR に残さない。

terminator は次のいずれかとする。

- `Jump`
- `Branch`
- `Return`
- `Trap`
- `Unreachable`

CFG edge は target block と argument 列を持つ。MIR では通常 argument 列は空だが、SSA IR では block parameter へ値を渡すために使う。

```text
then:
    jump merge(%10)

else:
    jump merge(%20)

merge(%value):
    return %value
```

SSA の合流には `phi` instruction ではなく block parameter を使用する。

## 7. Value and place

`Value` は immutable な計算結果で、一度だけ定義する。`Local` は mutable storage であり、`Place` を通して access する。

`Place` は次の base を持つ。

- local
- static symbol
- pointer value

base に projection を順番に適用する。

- dereference
- struct field
- array index

place は最終的な Ond type を保持する。target byte offset、pointer width、Kagura address は持たない。

## 8. Instruction

instruction は 0 個以上の result `ValueId`、operation、source span を持つ。複数 return の call は複数 result を持てる。

主要 operation:

- constant
- unary operation
- binary operation
- cast（infallible / checked）
- address-of
- load
- store
- aggregate copy
- runtime check
- direct / indirect call

整数演算は `Wrap` または `Checked` を明示する。Ond v1 の通常の `+`, `-`, `*`, unary `-` は `Wrap` とする。将来 checked operation を追加しても同じ IR で表現できる。

現行の共通Binary命令ではf32演算にもこの属性を保持し、f32除算を含め`Wrap`（非trapping）を指定する。
float opcodeの値の意味はbinary32であり、整数modulo演算という意味ではない。`Checked`にしてゼロ除算faultを示してはならない。

## 9. Runtime checks and traps

runtime check は暗黙の operation semantics に埋め込まず、独立した instruction とする。

- bounds check
- divisor non-zero check
- signed `minInt / -1` check

check は value を返さず、成功時は次の instruction へ進み、失敗時は Ond が定義する fault を発生させる。

bus access 自体の fault は load/store の `may_trap` property として残る。明示 check があっても hardware access が成功することまでは保証しない。

## 10. Effects

各 instruction kind から次の effect を一意に導出できなければならない。

```text
memory: None | Read | Write | ReadWrite
may_trap: bool
calls: bool
```

`Effects::PURE` の instruction だけが、使用されていないことを理由に無条件で削除できる。

raw pointer load/store は `AccessKind::Ordered` とし、削除、統合、複製、他の ordered operation を越えた移動を禁止する。compiler が所有する local storage への access は `AccessKind::Local` として区別できる。

call は当初 `CallEffects::Unknown` とし、全 memory を読み書きでき、fault しうる barrier として扱う。将来、compiler が証明した internal function に限り `Pure` または `ReadOnly` summary を付与できる。

## 11. Lowering rules

- operand と argument は source の左から右へ 1 回ずつ評価する。
- assignment は右辺をすべて評価してから左辺へ書く。
- `&&` と `||` は branch と merge block へ lower する。
- `if` と `for` は CFG へ lower する。
- array の動的 index は明示的な bounds check を生成する。文字列literalはarrayとして同じ経路を使う。
- raw pointer index は bounds check を生成せず、ordered pointer access を生成する。
- integer `/` は non-zero check と、signed 型では overflow check を生成してから divide を行う。
- integer `%` も non-zero check を生成する。signed `minInt % -1` は Ond semantics に従って `0` とする。
- direct call と indirect call を区別する。
- aggregate assignment と argument passing は value copy として aggregate copy を生成する。
- source span をすべての生成 instruction と terminator へ伝播する。

## 12. SSA conversion

MIR から SSA IR への変換では、address を取得されていない scalar local を昇格する。

昇格対象:

- integer
- `bool`
- `f32`
- pointer value
- function pointer
- compiler temporary

原則として memory に残すもの:

- address を取得された local
- array
- struct
- global storage
- pointer 経由で access される storage

SSA conversion は dominance frontier または sealed-block algorithm を使用できる。合流 value は block parameter と CFG edge argument で表す。

Memory SSA は初期実装の対象外とする。

## 13. Optimization pipeline

最適化レベルは次の方針とする。

```text
O0
  required canonicalization
  validation

O1
  constant folding
  local constant/copy propagation
  CFG simplification
  unreachable block elimination
  dead pure instruction elimination

O2
  SSA conversion
  sparse conditional constant propagation
  global copy propagation
  common subexpression elimination
  redundant bounds-check elimination
  loop analysis and loop-invariant code motion
```

fault または ordered memory access の順序を変える optimization は行わない。fast-math を必要とする変換は明示的な fast-math option がない限り行わない。

## 14. Validation

validator は少なくとも次を検査する。

- ID と table index の一致
- entry block と CFG edge target の存在
- block parameter と edge argument の個数および型
- value の単一定義
- operand value と local の存在
- instruction result の個数と型
- branch condition が `bool`
- return value の個数と型
- place base と projection operand の妥当性

通常 build では主要 phase の境界、compiler 開発 mode では各 pass の前後で validator を実行する。

## 15. Text form

MIR は安定した binary format を持たない。compiler test、diagnostic、`--emit mir` のため、deterministic な human-readable text dump を提供する。

text form は debugging interface であり、version 間の構文互換性を保証しない。

## 16. Target boundary

Kaguraへの具体的な適用、型layout・ABI・命令対応と実装段階は
[MIR backend境界設計](../../../specs/ond/targets/kagura-v1/mir-backend.md)を参照。
この設計文書の存在はbackend接続済みを意味しない。

MIR は次を含まない。

- Kagura register
- Kagura instruction encoding
- immediate width
- calling convention
- stack frame offset
- section
- symbol relocation
- cartridge bank
- Enbu MMIO address

これらは target Machine IR、object emitter、linker、cartridge builder が順に決定する。

## 17. Implementation status

現在実装済み:

- global の型推論・参照・代入・アドレス取得と `PlaceBase::Static` lowering
- package initializer の生成、全globalの先行zero初期化と import 記述順 DFS の起動契約

- package 宣言の `SymbolId` table と import/visibility を考慮した symbol lookup
- local/package scalar・array・struct constant の評価、循環参照・zero division・宣言型の範囲検査
- 整数定数式を使用した array length の解決
- 未対応・不正なソースは `FrontendError` とし、成功時は完全な typed HIR/MIR のみ返す
- target data layout を含まない HIR/MIR 共通の semantic type
- `TypeId` と project-wide `TypeTable` による型グラフ
- defined type、struct field、pointer recursive type の型定義
- 関数 parameter・return type の typed HIR lowering
- 関数参照・直接／間接 call の型付き HIR と MIR lowering（forward reference、再帰、import を含む）
- 複数戻り値の宣言・代入・return 転送、戻り値を破棄する call 文
- pointer のアドレス取得・参照外し・unchecked u32 index、address-taken local の記録と ordered access
- 固定長 array・struct の zero value、sparse/nested composite literal、field/index の読み書きと値コピー
- struct field の公開範囲・重複・型と、addressability の検査
- 文字列literalをdecode後の長さを持つ`[N]u8` compositeへ変換し、通常のarray constant/index/copyへ統合
- array/文字列literalの定数index検査と、arrayの明示的なruntime bounds check
- parameter/local の `LocalId`、scalar `var`、`:=`、代入、cast、return の typed HIR lowering
- lexical block scope、if/else（init を含む）、全形式の for、break/continue の typed HIR と CFG lowering
- `&&` / `||` の短絡評価：右辺専用 block と合流 block parameter
- 関数 parameter local と return signature の MIR lowering
- 単純な scalar `return` に含まれる literal、parameter load、単項・二項演算の MIR lowering
- integer 除算に対する zero・signed overflow check の明示化
- MIR data model
- CFG edge と block parameter の表現
- place と ordered access の表現
- explicit runtime check
- effect classification
- structural/type validator、定義順・到達可能CFGのdominance・命令の型制約・関数symbol/signatureの検査
- 除算・array indexの支配的な安全checkまたは静的証明の検査
- `new`/`alloc`/`free`/`load32`/`store32` の型付きintrinsic lowering
- integer/f32 literal と定数castの範囲検査、曖昧なimport名の拒否
- `compile_loaded_project` 成功前の自動MIR検証
- deterministic text dump の基盤

未実装または移行中:

- SSA IR と SSA conversion
- optimization pass manager

Kagura backendの標準入力は検証済みMIRである。旧AST backendは撤去済み。

### Compilation success contract

字句解析はUnicode識別子（現実装のカテゴリtableはUnicode 16.0）、2/8/10/16進整数、decimal/hexadecimal float、underscore制約、quoted/raw stringとescape検査、EOFのsemicolon挿入に対応する。原文tokenとUTF-8 byte spanを保持し、import pathにも同じ文字列デコーダを使用する。hexadecimal floatはguard/sticky bitを使用して直接binary32へroundTiesToEvenで丸め、f64経由の二重丸めを避ける。整数の桁数は字句解析では制限せず、HIRのliteral変換が現時点ではu64の制約を持つ。旧AST backendの独立literal変換は撤去済み。

`compile_project` / `compile_loaded_project` は、全宣言の意味検査・HIR生成・MIR生成・project validatorに成功した場合だけ `Compilation` を返す。未知の名前、型不一致、定数評価エラー、未対応構文はsource span付きの `FrontendError` とする。関数本体は文単位で状態を巻き戻して独立した後続エラーを収集し、失敗した宣言名への依存から生じる派生診断は抑制する。成功結果に未検査の関数や部分的なinitializerは含めない。

`Body::Pending`、`pending_diagnostic`、`Compilation::pending_diagnostics()`、HIR関数のAST本文/signature fallbackは撤去済み。型解決内部の `TypeKind::Pending` は前方参照の予約にだけ使用し、完成MIRではvalidatorが拒否する。`Compilation::ast` は解析結果の参照用に保持し、codegenでは使用しない。

MIR loweringの内部不整合やvalidator違反はinternal errorの診断として返し、空のunreachable bodyへ置き換えない。SSA化や最適化passが未実装であることは、sourceの受理可否やMIRの意味とは別の実装状況である。

整数定数の評価は多倍長整数による正確値計算を使用し、中間値をi128幅に制限しない。定数shiftはu32のcountを左operandの型幅で剰余化し、左shiftの結果をwrapしない。宣言型・aggregate要素型・明示cast先の範囲、zero division、signed minimum / -1を検査する。literal自体の既定型・型文脈による範囲検査は維持する。

array・struct定数は全明示要素を評価した疎なCompositeとして保持し、省略要素を型付きゼロとして扱う。文字列literalもbyte要素を持つarray定数である。定数field/index、arrayのlen、型付きnilを評価できる。通常call・allocation・外部memory readは定数にならず、短絡演算の未評価側に含まれる場合も拒否する。arrayの`len`はoperandを1回評価するため、operandに通常callや不正な定数演算を含む式は定数にできない。f32は各演算でbinary32に丸め、NaNをcanonical化する。定数cast・indexは検査済みの値をHIR/MIRへ渡し、runtimeでwrapする中間計算を再実行しない。

通常の宣言・代入・引数・return・global初期化・条件式・aggregate要素・組み込み引数・添字は、HIRのvalue境界で共通の必須定数評価を行う。定数部分式全体を正確値で評価してから要求型の範囲を検査し、確定値をMIRへ渡す。実行時式に含まれる独立した定数operandも同様に評価するが、local/globalの初期値は伝播せず、変数を含む演算のwrap/fault契約を変えない。arrayの`len`もoperandを通常どおり評価し、その副作用とfaultを保持した上で型から長さを解決する。これは最適化passではなく意味検査の一部である。

`[...]T{...}` はASTのarray length省略として保持し、compositeの全indexを調べて最大index + 1を推論する。添字なし要素は直前のindex + 1（先頭は0）に配置する。空literal、負数・非定数・重複index、u32で表現できない長さ、およびcomposite以外での推論長型は拒否する。推論後は通常の固定長Array TypeIdを使い、HIR/MIRには推論状態を残さない。要素の評価順はsource順を維持する。

関数型のparameter名省略・記載・混在に対応する。ASTでは関数宣言のSignature（名前必須）と関数型のTypeSignature（名前はOption）を分離し、名前を省略しても仮のbindingを生成しない。HIR/MIRのFunctionTypeにはparameter型と戻り値型だけを渡し、記載した説明用の名前は型同一性に影響しない。実行時の整数castはwrap、f32→整数castはcheckedの契約を維持する。Kagura backendも解決済みMIRの型を使用する。

### Validator responsibilities

- 型ID・local/value/block/instruction ID、定義の一意性、edge引数、return/call signature、place projectionの型整合性。
- 未解決型、無限サイズになる値型循環、struct field重複。pointer/functionを介した再帰は許可する。
- 同一block内の定義前参照、および到達可能CFG上の支配関係。到達不能block間の支配関係は要求しない。
- 演算の型領域、cast制約、parameter local一覧、address-taken、intrinsic signature/effects。
- direct call/function constantの解決先、global参照、symbol衝突、package ID、initializer signature、全packageを一度ずつ含みimport先が先行する起動順。
- 整数除算のnonzero/signed overflow、array添字のbounds check。checkは使用位置を支配する必要があり、定数から安全性を直接証明できる場合は省略可能。
- 生pointerの寿命・nil・範囲は言語契約どおり検査対象外。後続passも変換後にvalidatorを実行する。

### Memory intrinsics

`Callee::Intrinsic` はtargetに依存しない明示的なruntime operationであり、通常のfunction symbol lookupをしない。

- `New(TypeId)`: 引数なし、`*T`を返す。Tのzero valueで初期化した領域を確保する。サイズ・alignmentは後段で解決する。
- `Alloc(TypeId)`: `u32 -> *T`。`T`を指定個数格納できる未初期化領域を確保する。0個とallocation failureはnil。
- `Free`: 任意のpointerを受け取り、戻り値なし。nilはno-op。
- `Load32` / `Store32`: addressはu32またはpointer。読み出し結果・書き込み値はu32。targetのmemory access契約へ後段でloweringする。
- `New`もallocation failureはnil。全intrinsicのcall effectは保守的に `Unknown` とし、評価・メモリアクセス順序を保持する。組み込み名はbindingとして宣言できず、user declarationでshadowしない。

### Function calls and multiple results

- 宣言された関数は signature を参照して解決する。関数本体の lowering 順には依存せず、forward reference と相互再帰を扱える。
- HIR の `ExprKind::Function` は canonical symbol name と関数型の `TypeId` を持つ。関数値を local に保存したり、引数や戻り値に渡したりできる。
- `hir::Call` は callee、検査済み signature、引数、source span を持つ。引数は1式につき1値とし、個数と型を検査する。単一値の式としての call は戻り値が1個の場合だけ許可する。
- `hir::ValueList` は通常の式列か、単独の call を表す。`var a, b = pair()`、`a, b := pair()`、`a, b = pair()`、`return pair()` で戻り値を展開する。複数値をタプル型や複数の call に変換しない。他の式との混在や引数位置での暗黙の複数値展開は行わない。
- callee、各引数を左から右の順で評価する。複数代入では右辺をすべて評価してから書き込む。短絡論理式の右辺の call は、その経路が実行される場合だけ実行する。
- MIR は直接 call を `Callee::Direct`、関数値経由の call を `Callee::Indirect` として出力する。1個の Call instruction に戻り値数だけの `ValueId` を割り当てる。call 文で値を破棄する場合も、instruction の result signature は保持する。
- call effect は保守的な `Unknown` とする。レジスタ・スタック・calling convention はこの段階では決めず、Kagura backendが検証済みMIRから決定する。

### Memory and byte-array literal lowering

- field/index は target offset ではなく `Place` の型付き projection として保持する。raw pointer index は `Offset`、固定長 array index は `Index` と区別する。pointer offset は bounds/nil/lifetime check を行わない。
- arrayのindexは整数型を維持して`Check::Bounds`へ渡す。このcheckはsigned negative indexも範囲外とする。現段階では安全側に倒し、定数indexにもruntime checkを残すことがある。
- composite literal は compiler temporary を zero 初期化し、指定された要素を source 順に1回ずつ評価して書き込む。未指定要素は zero のままとする。巨大な zero array を要素数分の命令に展開しない。
- aggregate storage は local/temporary の place に置く。式・call の境界の aggregate `ValueId` は storage alias ではなく immutable な値の snapshot とする。aggregate store は temporary を経由する `AggregateCopy` を生成する。引数・戻り値も snapshot を受け渡し、physical copy/ABI は後段で決定する。これらの aggregate value は scalar SSA promotion の対象外。
- 代入の左辺 address/index を左から評価し、右辺の値をすべて snapshot として評価した後に書き込む。右辺評価中や先の左辺への書き込みで、後続のコピー元が変わることはない。
- 文字列literalはUTF-8/escapeをdecodeし、byteごとの`u8`要素を持つ通常の`Composite`としてHIRへ置く。MIRには文字列専用type、constant、instructionを持たせず、zero初期化・要素store・`AggregateCopy`へlowerする。
- 読み取り専用の文字列literal実体は`Static.null_terminated`で区別する。このflagは論理的なarray型や`StaticValue`を変更せず、target backendがhidden sentinelの配置と同一Object内の完全一致・suffix一致共有に使用する。
- `len`はoperandを1回評価し、その結果を捨てた後、array型の長さからHIRの`u32`定数へ解決する。runtime文字列length命令は存在しない。
- `thisFile()`は論理source pathを持つnull終端staticへの`*u8`、`thisLine()`は1始まりのsource行を持つ`u32`定数へHIR lowering時に解決する。hostの絶対pathはHIR/MIRへ渡さない。
- `&`はaddressableなstorageに限定し、そのlocalの`address_taken`を記録する。文字列literal式そのものはcomposite literal同様にaddressableではないが、変数へ格納した後の要素は通常のarray要素としてaddressableかつ代入可能である。

既存 runtime テストにあった pointer pointee 型の暗黙変更は明示的な integer 経由の cast へ変更した。定数範囲外 array index を runtime fault の検証に使っていた fixture は動的 index に変更し、定数範囲外の拒否は core の診断テストで検証する。

### Blank identifiers and statement syntax

- `_` は binding を作らず値を破棄する。HIR の `LocalDecl.locals`、`Assign.targets`、`VarItem.globals` は戻り値との位置対応を保つ optional slot を持ち、破棄位置を `None` とする。MIR にはその保存先を作らない。
- 破棄する式も型検査・評価し、call、副作用、fault、複数代入の評価順序を維持する。package-level `var _ = f()` は initializer 内で実行する。`const _` は評価・検査するがsymbolを作らない。
- `_` parameter はsignature/引数位置を保持し、名前解決のbindingは作らない。`_` は値として参照できず、`:=` の新しい名前にも数えない。`:=` の同名左辺重複は `_` 以外で拒否する。
- `}`・`)` の直前はsemicolonを省略できる。block内の空文、空のfor clause、call引数の末尾commaを受理する。単独式文とif/forのinit・postに置く式文はcallだけを許可する。
- これらはcompiler-coreのAST/HIR/MIRで検証する。coreでの受理とKagura上の実行検証は別の検証層として扱う。

### Globals and package initialization

- HIR `VarItem` は global symbol/type/span と initializer `ValueList` を保持する。型注釈なしのglobalは後方参照や複数戻り値から型を推論できる。推論の循環は診断し、明示型があれば循環した値参照自体は拒否しない。
- 型解決中にinitializerを検査しても、その値を実行したことにはしない。推論は呼び出し元のlocal scopeを参照せず、実際の初期化式はpackageの記述順で1回ずつ実行する。
- HIR/MIR packageの`initializer`は内部関数`<package>.$init`で、通常関数と別に保持する。`$`はsource identifierに使えないのでuser functionと衝突しない。source-level `func init()` は引き続き禁止。
- MIR `Global` は配置前の型付きstorage宣言であり、全てzero valueから開始する。section・address・byte sizeは決めない。全packageの全globalを先にzero初期化し、その後`Project::initialization_order`の順に存在するinitializerを呼び、最後に`main.main`を呼ぶのがconsumerの契約。
- orderはmainを根とし、ファイル名順・import記述順の深さ優先探索でimport先を先に並べ、各packageは1回だけ含める。型推論上の依存関係やPackageIdの大小では並べ替えない。
- zero-only宣言はinitializer内で再度zero storeしない。先行するinitializerの関数callが後方globalへ書いた値を消さない。初期化前のglobal参照はzero valueを読み取る。
- initializer内でもaggregate（文字列literal由来のbyte配列を含む）/pointer/call/複数戻り値を通常のHIR/MIRと同じ規則で扱う。globalアクセスには型付きstatic placeを使い、pointer経由ならordered accessにする。

旧AST backendとその文字列literal拒否処理は撤去済み。byte arrayへの文字列literal代入は標準MIR経路でコンパイルする。target-neutral MIRの成功だけでtarget容量制限や実機動作まで保証するものではない。
- 不正・未対応のglobal initializerはコンパイルエラーとし、部分的な初期化MIRを成功結果として返さない。
- MIR packageはimport依存関係を保持する。validatorはglobal symbol/typeとstatic参照、initializer signature、初期化orderの重複・欠落・依存順を検査する。dumpにはglobal zero宣言・initializer・起動順を表示する。
- Kagura backendはMIR起動契約から実機entryとpackage初期化呼出を生成する。

符号付き型の範囲外constant castを使っていたconformance fixtureは、同じ値を表す範囲内の負数literalへ変更した。実行時castのwrap semanticsとは区別する。
