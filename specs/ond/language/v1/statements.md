# 文と制御構文
<!-- ond-spec-id: language/v1-statements -->

本章は文、制御フロー、deferを定義する。

## 構文
<!-- ond-spec-id: language/v1-statements-grammar -->

次のEBNFは本章の構文を定義する。\
他章のproductionは[文法索引](./lexical.md#文法の定義元)を参照する。

```ebnf
Block         = "{" { Statement ";" } "}" .
Statement     = DeclStmt
              | SimpleStmt
              | ReturnStmt
              | TrapStmt
              | DeferStmt
              | BreakStmt
              | ContinueStmt
              | IfStmt
              | ForStmt
              | Block .
DeclStmt      = ConstDecl | VarDecl .
SimpleStmt    = EmptyStmt | ExpressionStmt | Assignment | IncDecStmt | ShortVarDecl .
EmptyStmt     = .
ExpressionStmt= CallExpr .
Assignment    = ExpressionList "=" ExpressionList
              | Expression AssignOp Expression .
AssignOp      = "+=" | "-=" | "*=" | "/=" | "%="
              | "&=" | "|=" | "^=" | "&^=" | "<<=" | ">>=" .
IncDecStmt    = Expression ( "++" | "--" ) .
ShortVarDecl  = IdentifierList ":=" ExpressionList .
ReturnStmt    = "return" [ ExpressionList ] .
TrapStmt      = "trap" string_lit .
DeferStmt     = "defer" Block .
BreakStmt     = "break" .
ContinueStmt  = "continue" .
IfStmt        = "if" [ SimpleStmt ";" ] Expression Block [ "else" ( IfStmt | Block ) ] .
ForStmt       = "for" ( Block | ConditionBlock | ForClauseBlock ) .
ConditionBlock= Expression Block .
ForClauseBlock= [ SimpleStmt ] ";" [ Expression ] ";" [ SimpleStmt ] Block .
```

## 静的成立条件
<!-- ond-spec-id: language/v1-statements-conditions -->

- `ForStmt` の `for Block` は無条件 loop を表す
- `DeferStmt`の宣言位置は`for` bodyと別のdefer blockの外側であることを成立条件とし、違反した位置はcompile errorとなる
- defer blockの制御構文は[defer規則](#defer)に従う
- `ExpressionStmt`はcall expressionで構成し、それ以外の単独式はcompile errorとなる
- `TrapStmt`のoperandは[builtin trapの成立条件](./builtins.md#8-trap)に従う
- `for init; cond; post {}` の `init` と `post` に使えるのは `SimpleStmt` のうち `ExpressionStmt`、`Assignment`、`IncDecStmt`、`ShortVarDecl` である
  - 空欄は省略を表す
- `if init; cond {}` の `init` に使えるのも同じである
- 文法上 `Expression` は任意の式を許すが、型検査で `if` と条件付き `for` の条件は `bool` に制限する
- multi-value function call は `VarSpec`、`Assignment`、`ShortVarDecl`、`ReturnStmt` で使える
  - multi-value展開を記した位置以外は単一値の式を要求する
- assignmentの左辺は`_`、書き込み可能な変数・参照外し・builtin配列/raw pointerのindex・struct field、または適合するcustom `[]=`を持つ`collection[index]`とする
- 単純代入`=`は最初に全左辺のbase、index、pointer operandを左から右へ各1回評価して保持し、次に全右辺を左から右へ各1回評価して保持する
  - custom index書込みではgetterを要求せず、receiver調整後の値とindexを保持する
- その後、左辺への書き込みを左から右へ行い、builtin配列格納先のbounds checkは各書込み直前、custom indexは当該setterを1回呼ぶ時点で書込みを行う
  - custom setterのreceiver、index、valueの型は選択済み`[]=`宣言へ完全一致することを成立条件とする
- 後続の格納先で bounds check が失敗しても、それ以前の書き込みは保持する
- 左辺の operand 内で行う配列読み取り（`*P[i]` の `P[i]` など）、`&A[i]` のアドレス取得、および右辺の配列読み取りは、その評価時に bounds check を行う

getterは[operator解決](./expressions.md#user-defined-operator)で選んだindex読取り、setterは同じ規則で選んだindex書込みを表す。\
両者は独立に解決し、それぞれcustom宣言またはbuiltin操作を選ぶ。

- compound assignmentの左辺は`_`以外の代入先1個、右辺は式1個とする
  - 変数・参照外し・struct fieldの代入先はbaseとpointer operandを1回評価し、現在値を1回読んだ後に右辺を1回評価する
  - 対応二項演算を1回行い、結果型が通常storageの型と完全一致することを検査して書き込む
  - index代入先はgetterとsetterを独立に解決し、baseとindexを各1回評価して、それぞれのreceiver調整後の値とindexを保持する
  - getter1回、右辺1回、対応二項演算1回、setter1回の順で実行し、custom/builtinの各組合せに同じ順序を適用する
  - getter、二項演算、setterがすべて静的に成立し、二項演算結果型がsetterの要求するvalue型と完全一致することを要求する
  - builtin setterのvalue型は配列/raw pointerの要素型とする
  - builtin配列getterのbounds checkは読取り時、builtin配列setterのbounds checkは書込み時に行う
- 未注釈literalを含むindexの文脈型付けは単純代入ではsetter、compoundではgetterとsetterのcustom parameterから同じindexへ課される文脈を照合する
  - 文脈が同じ型を要求する場合、その型で1回型付けし、異なる型を要求する場合はcompile errorとなる
  - customの文脈がない場合は通常の式の型付けを行い、確定した1個のindex値を選択済みの各操作に照合する
  - 変数や明示変換で型が確定したindexは、その型でgetter/setterを独立に解決する
- 各代入段階でfaultした場合は後続段階を実行せず、完了済みwriteと他のeffectを保持して停止する
- `++`と`--`は後置statementの構文である
- 戻り値を持つ関数で本体末尾へ構文上到達できる入力はcompile errorとなる
  - `return`と`trap`は当該経路を終了し、blockは先行文から通常継続できる経路を次文へ接続する
  - 当該経路のreturn、trap、最内loopへのbreak/continue、終端loopの後の文は、その経路の到達判定へ寄与しない
  - `if`は条件が定数でもthenとelseの両経路を持ち、else省略時の経路は通常継続する
  - 条件付きforは条件の定数真偽によらず退出経路を持つ
  - `for {}`とconditionを省略した3 clause forは、当該loopを対象とする構文上到達可能なbreakがない場合に後続文への経路を持たない
  - 内側loopを対象とするbreakは外側loopの退出経路を作らない
  - 呼出先の本体や実行時の定数真偽で構文上の経路を追加・削除しない
- 到達判定で経路に寄与しない文にも、構文・名前・型など他の成立条件を適用する

## 文と制御構文
<!-- ond-spec-id: statements/control-flow -->

- 文はsemicolonで区切り、sourceの改行に[semicolon挿入規則](./lexical.md#semicolonと改行)を適用する
- block は `{}` で囲む
- `if`は式のconditionを取り、丸括弧は通常の式のグループ化として解釈する
- `if`、`else`、`for` は必ず block を伴う
- `if` と条件付き `for` の条件は `bool` でなければならない
- `for`は本章のEBNFで示す無条件、条件付き、3 clauseの形を持つ
- `break` と `continue` は最内の `for` を対象とする
  - `continue`はloop bodyの残りを飛ばし、3 clause形式ではpostを実行してからconditionを再評価する
  - 条件付き形式はconditionの再評価、無条件形式は次のbody実行へ進む
- `return` は現在の関数を終了する
- `trap`の停止とdebug理由の記録は[builtin trap](./builtins.md#8-trap)に従う

### `defer`
<!-- ond-spec-id: statements/defer -->

`defer` は、文へ到達した関数 invocation の終了処理を block で登録する。

```ond
func process() -> i32 {
    var first = alloc[u8](32)
    defer {
        free(first)
    }

    if needExtra() {
        var extra = alloc[u8](64)
        defer {
            free(extra)
        }
    }

    return 0
}
```

- 登録済みの defer block は、関数本体末尾への到達または `return` による正常終了時に、登録と逆の順序で各 1 回実行する
- `return` の戻り値とその副作用は defer block より先に評価する
  - 確定済みの戻り値はdefer完了後も保持する
- defer block 内の式は登録時ではなく実行時に評価し、参照する local variable のその時点の値を使う
  - local storage は defer block の実行完了まで有効とする
- deferの登録集合は、そのinvocationで到達したdefer文によって定まる
- defer block は宣言、代入、call、`if`、およびネストした `for` を含む通常の block とする
  - block内の`return`と別の`defer`はcompile errorとなる
- defer block 内の `break` と `continue` は、その block 内にネストした最内の `for` だけを対象にできる
- deferの位置は`for` body外を成立条件とし、各deferサイトへの登録は関数invocationにつき最大1回となる
- defer block が正常に完了する限り、残りの登録済みdefer blockを引き続き実行する
  - faultはその場で実行を終了し、非終了のblockではその処理を継続する
- defer block内で呼んだ関数の戻り値はblock内で処理または破棄できる
  - 失敗を呼出元へ返す終了処理は通常処理として明示的に実行する
