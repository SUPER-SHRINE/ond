# Kagura v1 inline assembly
<!-- ond-spec-id: targets/kagura-v1-inline-assembly -->

本書はKagura v1をtargetとするOnd inline assemblyの命令本文、control-flow検査、memory効果、faultおよびregister保持契約を定義する。\
binding、型、入力評価と出力代入は[Ond inline assembly](../../language/v1/inline-assembly.md)を定義元とする。\
命令のencoding、architectural state transition、faultは[Kagura CPU規範snapshot](./README.md#参照するkagura規範)のCPU ISAに従う。

## 命令構文
<!-- ond-spec-id: targets/kagura-v1-inline-assembly-syntax -->

本文は空、または命令・label定義を改行若しくはsemicolonで区切った列とする。\
`source_line_break`はLFまたはCRLFであり、line comment内の改行とblock comment内の各改行も含む。\
spaceとtabはtoken間の空白だが、命令区切りではない。その他の空白、`//` line comment、`/* ... */` block commentは[Ond v1のsource形式](../../language/v1/lexical.md#source形式)に従い、本文内で`}`まで読み取る。\
本文内のsemicolonは命令区切りであり、外側のOnd文に適用するsemicolon挿入は本文中で行わない。

```ebnf
AsmInstructionText = { Separator } [ AsmItem { Separator { Separator } AsmItem } { Separator } ] .
AsmItem             = LabelDef | AsmInstruction .
LabelDef            = identifier ":" .
Separator           = ";" | source_line_break .
AsmInstruction      = Arithmetic | Memory | Compare | LocalBranch | ExternalBranch .
Arithmetic          = ("ADD" | "NAND" | "SHIFT" | "MUL") Register "," Register "," Register "," AsmImmediate .
Memory              = ("LDB" | "LDH" | "LDW" | "STB" | "STH" | "STW") Register "," Register "," Register "," AsmImmediate .
Compare             = "CMP" Register "," Register "," Register "," AsmImmediate .
LocalBranch         = "JZ" "r0" "," Register "," identifier .
ExternalBranch      = "JZ" "r0" "," "r0" "," Register "," "0" .
AsmImmediate        = [ "+" | "-" ] int_lit .
```

mnemonicは大文字表記、register markerは小文字`r`表記とする。\
register operandは[Ond inline assemblyのRegister](../../language/v1/inline-assembly.md#構文)productionの綴りを使う。\
各命令はCPU ISAで示す4 operand形式`rd, rs1, rs2, imm`を持つが、local branchだけは3 operand形式、external branchだけは4 operand形式とする。\
4 operand命令の`imm`はOnd integer literalで、数学値が-32768から32767まででなければならない。\
この符号付き16-bit範囲条件はCPU ISAがimm fieldを無視する命令にも適用する。\
imm fieldには数学値を2^16で剰余した値を格納する。

命令operandの値域はCPU ISA encodingの静的成立条件を満たさなければならない。\
`CMP`のimmは`0`、`1`、`2`、`4`、`5`、`6`のいずれかであり、基本比較種別00、01、10と反転bitの組合せを表す。\
`SHIFT`のimmは0から31、-32768から-32737、または-16384から-16353までであり、それぞれleft、logical right、arithmetic right shiftのencodingを表す。\
これら以外の値、および命令に定義されないreserved bitsを要求する値はcompile errorとなる。

## Register、命令意味、control flow
<!-- ond-spec-id: targets/kagura-v1-inline-assembly-flow -->

命令意味はCPU ISA snapshotに従い、以下の表のread/write/control/memory分類を検査に用いる。\
operand欄はKagura ISAの`rd, rs1, rs2, imm`位置を示す。

| 命令群 | 読むregister | 書くregister | control/memory effect |
| --- | --- | --- | --- |
| `ADD`, `NAND`, `SHIFT`, `MUL`, `CMP` | `rs1`, `rs2` | `rd` | control移行なし、memory accessなし |
| `LDB`, `LDH`, `LDW` | `rs1`, `rs2` | `rd` | 指定幅のmemory read |
| `STB`, `STH`, `STW` | `rd`, `rs1`, `rs2` | なし | 指定幅のmemory write |
| local `JZ r0, condition, label` | `condition` | taken時に`r0`のみ | 条件がzeroならlabelへ分岐 |
| external `JZ r0, r0, target, 0` | `target` | `r0`のみ | asm block外へ間接移行 |

`r0`のreadはzero、writeは破棄される。\
`JZ` local formの条件registerがzeroの場合にのみ分岐し、非zeroなら次の命令へ進む。\
conditional branchのanalysisは、conditionがzeroとなるedgeとnonzeroとなるedgeの双方を含める。ただしconditionが`r0`ならzero edgeだけを含める。\
定数計算によりcondition値が予測できる場合もedge集合を削除しない。\
labelは同一asm block内の命令開始位置を指し、命令列の末尾に置かれたlabelは正常fallthrough位置を指す。\
label名はblock scopeで一意であり、Ond変数名とは独立する。重複labelと未定義labelはcompile errorとなる。

local branchの符号付きPC相対offsetは、命令addressを`P`、labelの命令addressを`T`として`(T - (P + 4)) / 4`で求める。\
差が4の倍数であり、得られた値が-32768から32767命令位置の範囲内でなければcompile errorとなる。\
compilerはこの制約を満たすための暗黙thunk、scratch register使用、命令挿入を行わない。\
raw数値のPC相対branch、direct symbol branch、通常asm内のexternal jump、call、return opcodeは成立しない。\
`asm noreturn`では上記に加えて`JZ r0, r0, rN, 0`形式を許可し、`rN`は`r1`から`r15`とする。\
この形式は常にregister間接移行し、移行先のalignmentと実行可能性はKagura CPU ISAに従う。

## 定義到達性とregister制約
<!-- ond-spec-id: targets/kagura-v1-inline-assembly-definitions -->

通常asmのentryではinput binding registerだけが定義済みであり、それ以外のregisterは未定義である。\
`asm noreturn`ではinput registerと`r14`が定義済みであり、input bindingによる`r14`指定がなければ`r14`はasm直前のSP値を保持する。\
命令列上のscratch registerは、readに至るすべてのcontrol-flow経路で先行writeにより定義されなければならない。\
loop headerへのbackedgeはその経路上の明示writeだけを定義として扱い、loop前の値を新たな定義として補わない。\
output registerは正常なblock退出に至るすべての経路で定義済みでなければならない。\
`r0`は常に定義済みzeroである。

通常`asm`本文は`r14`と`r15`をregister operandまたは命令fieldのいずれにも使用できない。\
通常`asm`の`r1`から`r13`はbindingなしでscratchとして使用できる。\
`asm noreturn`の命令本文では`r1`から`r15`を使用できる。\
block間でbindingなしregisterの値を引き継ぐ保証はない。\
通常asmではcompilerは実行前後に生存するOnd値を保持し、通常の関数呼出しに対するcallee-save契約を満たす。\
命令本文の明示storeとoutput bindingの代入はそれぞれの定義済み効果を持つ。\
compiler専用退避領域への推測addressによる書込みは、値保持の保証対象としない。

## Effect、順序、停止
<!-- ond-spec-id: targets/kagura-v1-inline-assembly-effects -->

すべてのasm文はobservable side effectを持つ。\
全命令のsource順序を保ち、memory accessの回数、幅、順序を変更しない。\
memory effectはraw pointer accessと同じbus address spaceに作用し、aliasingを考慮して前後のobservable effectとの順序を保つ。\
asm memory accessは[raw pointer access](./machine.md#6-raw-pointer-access)の有効address、alignment、bus規則に従う。\
asm命令のfault/debug range operationは[debug metadata](../../debug-metadata/v1.md#2-sourceとfault-range)に従う。\
Kagura CPU faultは[CPU fault処理](./README.md#参照するkagura規範)に従い、fault命令自身はregister、PC、bus deviceへの副作用を残さず停止する。\
fault前に完了したstoreは保持され、faultまたは非終了時にOnd output bindingの書戻しは行われない。

通常asmの正常末尾fallthroughはOnd code continuationへ戻る。\
`asm noreturn` blockの有限な正常退出経路はexternal branchだけであり、block末尾へ到達する経路がある場合はcompile errorとなる。\
asm内部の循環は許可し、循環する実行はOnd側への継続を持たない実行として扱う。\
external移行後に制御が偶然Ond compiler-generated codeへ戻ることを利用するsourceは、`asm noreturn`の移行先契約を満たさない。
