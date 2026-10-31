# Inline assembly
<!-- ond-spec-id: language/v1-inline-assembly -->

本章はOnd v1の`asm`文の構文、binding、評価、値の受渡し、および制御移行を定義する。\
命令本文は[Ond for Kagura v1 inline assembly](../../targets/kagura-v1/inline-assembly.md)に従い、Kagura v1 targetでのみ成立する。

## 構文
<!-- ond-spec-id: language/v1-inline-assembly-grammar -->

文のEBNFで用いる`AsmInstructionText`は、Kagura v1が定義するasm命令列である。\
その字句・構文は[Target命令構文](../../targets/kagura-v1/inline-assembly.md#命令構文)を定義元とする。

```ebnf
InlineAssembly = "asm" [ "noreturn" ] [ AsmBindings ] AsmBlock .
AsmBindings  = "(" [ AsmBinding { "," AsmBinding } [ "," ] ] ")" .
AsmBinding   = Register AsmDirection Expression .
AsmDirection = "<-" | "->" | "<->" .
Register     = "r0" | "r1" | "r2" | "r3" | "r4" | "r5" | "r6" | "r7"
             | "r8" | "r9" | "r10" | "r11" | "r12" | "r13" | "r14" | "r15" .
AsmBlock     = "{" AsmInstructionText "}" .
```

括弧を省略した場合はbindingがない。`()`は空のbinding列を表す。\
専用構文としての認識はStatement開始位置に限り、`asm`、optionalな`noreturn`、optionalなbindings、`{`の順に一致する場合に成立する。\
`noreturn`単独はstatement構文ではない。\
この形に一致しない`asm`は通常のidentifierとして名前解決し、通常の式およびcall構文に従う。

`<-`、`->`、`<->`は各bindingのRegister直後でdirectionを読む位置に限り方向tokenとして認識する。\
その位置で最長一致を用い、`<->`を`<-`と`>`へ分割しない。\
binding右辺は通常の式であり、`a < -b`と`a<-b`の双方を既存の`<`と単項`-`として解釈する。\
asm bindingのdirection位置以外では既存のtoken集合と式文法を適用し、矢印を新たな式演算子として扱わない。

## Bindingと型
<!-- ond-spec-id: language/v1-inline-assembly-bindings -->

`Register`はEBNFに列挙した綴りだけを受理する。\
register名はグローバル予約語ではなく、register番号として解釈するのはasm bindingの左辺と命令本文のregister operandに限る。

bindingの右辺は通常のOnd式または左辺への書込み対象名として名前解決する。\
従って`r1 <- r1`では左の`r1`はregister、右の`r1`は通常のOnd bindingである。\
`<-`の右辺は単一値式であり、`->`と`<->`の右辺は書込み可能なscalar変数identifierである。\
field、index、参照外し、blank identifier、およびその他の式は出力先として成立しない。

bindingの左辺register番号は一つのasm文内で重複してはならない。\
`->`または`<->`の出力先変数は一つのasm文内で重複してはならない。\
通常の`asm`は`r1`から`r13`をbindingに使用できる。\
`asm noreturn`は入力bindingに限り`r1`から`r15`を使用でき、出力bindingを持たない。\
`r0`はbinding対象にできない。

`<-`と`<->`はinput binding、`->`と`<->`はoutput bindingである。\
binding型は整数型、`bool`、`f32`、raw pointer、function pointer、およびそれらのdefined typeとする。\
array型とstruct型はbinding値として扱わず、そのaddressをraw pointerとして渡す。

input valueは[ABIのDirect-1値分類](../../targets/kagura-v1/abi.md#3-value-classification)が定めるbit representationでregister幅に配置する。\
output integerはregister下位の型幅bitを型の符号で解釈し、boolはzeroをfalse、それ以外をtrue、f32はbit pattern、pointerとfunction pointerはaddressとして解釈する。\
defined typeはunderlying typeと同じrepresentationを持つ。

## 評価と値の受渡し
<!-- ond-spec-id: language/v1-inline-assembly-evaluation -->

compilerはinput bindingの式をheaderに記した順に各1回評価し、その評価結果をsnapshotとして保持する。\
すべてのinput評価が完了した後に、そのsnapshot値を各指定registerへ配置する。\
評価中に呼出しが発生しても、先に取得したsnapshot値は保たれる。\
`<-`で指定したregisterの命令本文による変更は、binding自体の効果として元のOnd変数へ書き戻さない。\
命令本文がその変数のalias先へ明示的にmemory storeを行った場合は、そのstoreの結果を保持する。

`->`で指定したregisterはasm block entry時に未定義である。\
`<->`のregisterはentry時にinput値で定義される。\
blockが正常にOnd継続へ到達した場合、compilerは全output register値を取り出した後、出力先変数へbinding記載順に代入する。\
出力代入操作は出力先の既存値を読み取らずに新しい値を格納する。\
`<->`ではinput評価においてasm開始前の値を読む。

KaguraのCPU faultまたは停止しない実行ではasm文はOnd継続へ到達せず、outputを出力先へ代入しない。\
input式評価中に完了した副作用は保持される。

## 通常継続と外部移行
<!-- ond-spec-id: language/v1-inline-assembly-control -->

通常`asm`は命令本文が実行時にloopまたはbranchを含む場合も、通常完了時には直後のOnd文へ継続する文として扱う。\
内部control-flowからnoreturn性を推論しない。\
`asm noreturn`はOndの関数制御フロー上で終端経路であり、その後の文へ継続しない。

`asm noreturn`のinput式評価は現在のstack pointerで完了する。\
input bindingが`r14`を指定する場合、命令本文開始時の`r14`はそのsnapshot値である。\
`r14` inputがない場合、命令本文開始時の`r14`はasm文直前の値を保持する。\
すべてのinput値を設定し、compilerによるstack accessを完了した後に、命令本文開始時の`r14`を確定する。\
その後、命令本文の明示命令列にcompilerは退避・復元その他の命令を挿入しない。\
外部移行時に保証するregister値は、inputで指定された値、input未指定時に保持される`r14`、および常時zeroの`r0`である。\
命令本文が明示的に定義したregister値は、その定義後にだけ保証される。

external移行は現在の関数を正常終了させない。\
移行前に`defer`を実行せず、関数epilogue、callee-save復元、stack pointer復元を行わない。\
callerの制御フロー型には`noreturn`属性を伝播しない。\
移行先のentry、stack pointer、alignment、memory、引数契約は利用側と移行先が定める。\
暗黙のstate cleanupまたはresetは行わない。
