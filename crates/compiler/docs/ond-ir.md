# Ond Compiler IR

本書は `ond` compiler の内部表現と phase 境界を定義する。compiler pipeline は `AST -> HIR -> MIR -> SSA IR -> Target Machine IR -> Object -> link -> package` とする。

本書に書かれていない内部表現、最適化段階、公開 object format は v1 では持たない。

## 1. Scope

- `AST` は source syntax を表す。
- `HIR` は名前解決済み・型付きの高水準意味表現を表す。
- `MIR` は関数単位の control-flow graph を持つ低水準表現を表す。
- `Object` は codegen 後に linker へ渡す package 単位の成果物を表す。
- package graph、diagnostic collection は compiler の補助データであり、IR には含めない。

## 2. Common Rules

- すべての IR node は source に対応する `Span` を持つ。compiler 内部生成 node は最も近い user source span を持ち、対応 source がない場合だけ synthetic span を使う。
- `HIR` 以降の式、文、local、field、function、type には stable な内部 ID を与える。
- `HIR` と `MIR` の型は [Ond Language v1](../../../specs/ond/v1/language.md) に定義した guest type だけを表す。host language 独自型は混在させない。
- `MIR` は非 SSA、optimization 用 `SSA IR` は別形式とする。
- 実行順序は source の左から右という言語規則を保持しなければならない。

## 3. AST

### 3.1 Role

- parser の出力とする。
- source file の構文構造、token 順序、明示された syntactic form を保持する。
- name resolution、type checking、const evaluation は行わない。

### 3.2 Unit of Ownership

- `AstProject`
- `AstPackage`
- `AstFile`

`AstProject` は project root 以下で発見した source file 群を package ごとに束ねる。import graph の妥当性はまだ保証しない。

### 3.3 AST Must Contain

- package clause、import decl、top-level decl
- source-level の `var`、`const`、`type`、`func`
- statement の原形
- expression の原形
- type syntax の原形
- literal の字面に対応する値
- source 上の identifier 文字列
- source 上の field 名、package-qualified name、composite literal の key
- multi-value call、`:=`、`for` の 3 形態、`if init; cond` のような syntactic sugar

### 3.4 AST Must Not Contain

- 解決済み symbol 参照
- 推論済み型
- 定数 folding 結果
- bounds check や helper call のような compiler-generated 処理
- CFG

### 3.5 AST Notes

- comment は v1 では保持しなくてよい。
- semicolon insertion は parser の入力規則で解決済みとし、AST に semicolon node を持たない。
- parse error 後の recovery 形は implementation-defined とするが、以降の phase に渡す AST は整合した木でなければならない。

## 4. HIR

### 4.1 Role

- name resolution と type checking の出力とする。
- package 境界をまたぐ symbol 解決、visibility 判定、defined type の区別、定数式評価をここで確定する。
- source-level の高水準構造は残しつつ、以降の lowering に必要な意味情報をすべて持つ。

### 4.2 Unit of Ownership

- `HirProject`
- `HirPackage`
- `HirFile`
- `HirFunction`

`HirProject` は import graph が閉じた全 package を保持する。

`HirProject` は project-wide な型定義 table も所有する。HIR/MIR 内の型は recursive な値として埋め込まず `TypeId` で参照し、defined type、struct、pointer recursive type を有限な graph として保持する。

### 4.3 HIR Must Contain

- 解決済み package graph
- top-level item ID と canonical symbol name
- local binding ID
- 各 expression と statement の確定型
- compile-time 定数値
- package-qualified name を解決した item 参照
- struct field name を解決した field 参照
- function call の callee 種別
- function pointer call と direct call の区別
- multi-value function signature
- package-level `var` 初期化順序

### 4.4 HIR Must Not Contain

- 未解決 identifier
- 暗黙型変換
- source-level の `:=`
- parser 都合の曖昧な構文情報
- machine register

### 4.5 HIR Lowering Rules

- `:=` は通常の local `var` 宣言へ lower する。
- const 式は型付き constant value として保持し、再評価しない。
- defined type と underlying type の区別を残す。
- `if`、`for`、`&&`、`||`、composite literal、多値返却は高水準のまま保持してよい。
- 配列index、field access、`*pointer`はまだtarget命令列へ分解しない。文字列literalはbyte配列として扱う。

### 4.6 Recommended HIR Shape

- item level: `Const`, `Var`, `Type`, `Func`
- statement level: `Block`, `VarDecl`, `Assign`, `ExprStmt`, `If`, `For`, `Break`, `Continue`, `Return`
- expression level: `Literal`, `Name`, `Call`, `Methodless Selector`, `Index`, `Deref`, `AddrOf`, `Unary`, `Binary`, `Cast`, `Composite`

v1 では method を持たないため receiver 付き declaration node は存在しない。

移行期間中、まだ型付き HIR node が実装されていない構文を含む function body は明示的に `Pending` とする。`Pending` body を意味解析済みとして扱ってはならず、対応構文の追加とともに段階的に解消する。

## 5. MIR

### 5.1 Role

- target-neutral な low-level IR とする。
- unit は function 単位とし、各 function は CFG を持つ。
- source-level control structure と sugar を取り除き、実行順序、temporary、branch、call、memory access を明示する。
- target register、ABI、instruction encoding、pointer width、section、relocation は含めない。

### 5.2 Unit of Ownership

- `MirPackage`
- `MirFunction`
- `BasicBlock`

package-level `var` 初期化関数も通常の `MirFunction` として表す。

### 5.3 MIR Must Contain

- entry block と終端 block を含む CFG
- basic block ごとの instruction 列と terminator
- function parameter local と logical return type
- mutable local、immutable value、place
- 各 local と value の確定した Ond type
- direct call target または indirect call operand
- branch condition
- bounds、division などの明示的な runtime check
- memory effect、fault、call の分類に必要な operation 情報
- ordered raw pointer access
- operation ごとの overflow behavior

### 5.4 MIR Must Not Contain

- source-level `if`、`for`
- source-level `&&`、`||`
- source-level composite literal
- source-level 多値代入の曖昧な評価順
- 未解決型
- parser 由来の sugar
- target machine register
- target ABI の argument location
- target 固有の address、field offset、instruction width

### 5.5 MIR Lowering Rules

- `if` と `for` は basic block と branch へ lower する。
- `&&` と `||` は short-circuit を保つ branch へ lower する。
- 多値返却は複数 return slot と複数 destination へ lower する。
- 代入は右辺をすべて評価してから左辺へ書く順序を MIR 上で明示する。
- 配列indexのbounds checkは明示instructionとして生成する。文字列literalも同じarray経路を使う。
- `pointer[index]` は unchecked place projection と ordered load/store へ lower する。
- struct field access は field projection として保持し、target byte offset は Machine IR lowering で決定する。
- `f32` の算術、比較、整数変換は Ond semantics を持つ operation として保持する。runtime helper の選択は target backend が行う。
- integer `/` と `%` は明示 check と semantic operation として保持する。runtime helper の選択は target backend が行う。
- integer 比較は signed/unsigned と operand type を保持し、target instruction を仮定しない。

### 5.6 Recommended MIR Shape

`MirFunction` は少なくとも次の論理情報を持つ。

| Field | Meaning |
| --- | --- |
| `name` | canonical symbol name または local runtime name |
| `parameters` | parameter local ID 列 |
| `returns` | logical return type 列 |
| `locals` | mutable local storage 列 |
| `values` | immutable value 列 |
| `blocks` | `BasicBlock` 列 |
| `entry` | entry block ID |

`BasicBlock` は少なくとも次を持つ。

| Field | Meaning |
| --- | --- |
| `id` | block ID |
| `parameters` | SSA IR が使用する block parameter 列 |
| `instructions` | source evaluation order に並ぶ instruction |
| `terminator` | block の終了動作 |

instruction は少なくとも次を表せればよい。

- constant、unary、binary、cast
- address-of と place projection
- load
- store
- aggregate copy
- runtime check
- direct / indirect call

terminator は少なくとも次を表せればよい。

- `Jump`
- `Branch`
- `Return`
- `Trap`
- `Unreachable`

### 5.7 Call Representation

- direct call は target symbol を持つ。
- indirect call は callee operand を持つ。
- call site は logical function signature、argument value 列、result value 列を持つ。
- helper symbol と ABI は MIR より後の target lowering が決定する。
- 未解析の call は全 memory を読み書きでき、fault しうる barrier として扱う。

### 5.8 Detailed Contract

MIR の型、effect、validation、SSA conversion、optimization policy の詳細は [Ond MIR Design](../../compiler-core/docs/mir.md) に従う。

## 6. Object Boundary

### 6.1 Role

- `MIR` から machine code、static data、symbol、relocation を生成した package 単位の成果物とする。
- linker は `Object` だけを読み、`AST`、`HIR`、`MIR` を読まない。

### 6.2 Object Must Contain

- [Ond Linker v1](./ond-linker.md) で定義した `sections`
- `symbols`
- `relocations`
- `imports`
- `init_symbol`
- `main_symbol`

### 6.3 Object Must Not Contain

- source span
- 型検査用情報
- CFG
- source-level identifier

## 7. Phase Boundaries

- parse 成功後にだけ `AST` を生成する。
- name resolution と type checking の両方が成功した package だけ `HIR` を生成する。
- `HIR` から `MIR` への lowering は package 単位で行う。
- codegen は `MirPackage` 1 個から `Object` 1 個を生成する。
- linker は `main` package から到達可能な全 `Object` と runtime support object を入力とする。

## 8. v1 Non-Goals

- generic IR
- method dispatch IR
- exception handling IR
- Memory SSA
- stable serialized MIR / SSA format
- debug info format
- public `.o` file format
- incremental compilation cache format
