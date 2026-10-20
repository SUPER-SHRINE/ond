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
- `TrapStmt`のstring literalはUTF-8としてdecodeできなければならず、通常のexpressionや値ではない
- `for init; cond; post {}` の `init` と `post` に使えるのは `SimpleStmt` のうち `ExpressionStmt`、`Assignment`、`IncDecStmt`、`ShortVarDecl` である
  - 空欄は省略を表す
- `if init; cond {}` の `init` に使えるのも同じである
- 文法上 `Expression` は任意の式を許すが、型検査で `if` と条件付き `for` の条件は `bool` に制限する
- multi-value function call は `VarSpec`、`Assignment`、`ShortVarDecl`、`ReturnStmt` で使える
  - multi-value展開を記した位置以外は単一値の式を要求する
- assignment の左辺は `_`、変数、参照外し、配列または raw pointer の index、struct field のいずれかで、書き込み可能でなければならない
- 単純代入 `=` は最初に左辺の index と pointer operand を左から右へ評価し、次に右辺を左から右へ評価して、それぞれの値を保持する
- その後、左辺への書き込みを左から右へ行い、直接の配列格納先に必要な bounds check は各書き込みの直前に行う
- 後続の格納先で bounds check が失敗しても、それ以前の書き込みは保持する
- 左辺の operand 内で行う配列読み取り（`*P[i]` の `P[i]` など）、`&A[i]` のアドレス取得、および右辺の配列読み取りは、その評価時に bounds check を行う
- compound assignment の左辺は `_` 以外の書き込み可能な式 1 個、右辺は式 1 個とし、左辺の index、pointer operand、bounds check、および現在値の読み取りを 1 回行ってから右辺を評価する
- `++`と`--`は後置statementの構文である
- 戻り値を持つ関数は、到達可能なすべての実行経路で宣言と一致する `return` に到達しなければならない
  - 無条件 `for {}` は、外へ到達する `break` を含まない場合に限り終端とみなす

## 文と制御構文
<!-- ond-spec-id: statements/control-flow -->

- 文はsemicolonで区切り、sourceの改行に[semicolon挿入規則](./lexical.md#semicolonと改行)を適用する
- block は `{}` で囲む
- `if`は式のconditionを取り、丸括弧は通常の式のグループ化として解釈する
- `if`、`else`、`for` は必ず block を伴う
- `if` と条件付き `for` の条件は `bool` でなければならない
- `for`は本章のEBNFで示す無条件、条件付き、3 clauseの形を持つ
- `break` と `continue` は最内の `for` を対象とする
- `return` は現在の関数を終了する
- `trap "reason"`は回復不能なCPU trapで現在の実行を終了する
  - 理由をdebug metadataに記録し、その場で停止する

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
