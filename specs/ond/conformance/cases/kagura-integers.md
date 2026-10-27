# Kagura上の整数実行vector
<!-- ond-spec-id: conformance/cases/kagura-integers -->

全caseは[共通case契約](./README.md#入力と期待結果の読み方)のproject manifestとbuildを使用し、sourceを`main.ond`に置く。\
Kagura実行は次の状態と操作で行う。\
このfamilyはcaller指定の配置論理値と正常終了命令を入力にした言語・命令の個別検査であり、ONDX file生成やCLIのlink.exit選択を操作へ含めない。\
以下のimageは[解決済みlink結果の論理値](../../artifacts.md#linkplanとlinkedimage)を指し、JSON保存成果物を入力としない。

この個別検査では、Text、Rodata、Data、Bssの順に、最初のTextをmemory baseへ、後続kindを直前kindの末尾から配置する。\
各kindとObjectの開始は`max(alignment, 4)`へ揃え、空kindは省略する。\
heap_baseは全section末尾を4へ切り上げ、stack_topはmemory baseとmemory sizeの和、stack_bottomはそこからstack sizeを引いた値とする。\
この配置入力はヘッダを含まない言語・命令検査の起点を指定し、ONDX fileの配置規則へ追加しない。

| 項目 | 初期状態・操作 |
| --- | --- |
| LinkPlan | `read_only=null`, memory base=`0x1000`, memory size=`0x800000`, stack size=`0x100000` |
| RAM | `[0x1000,0x801000)`をzero初期化し、配置済みsectionのbyte列を指定addressへ置き、initialized bytes以降からmemory_size末尾までzero初期化 |
| 計算領域 | `[0x20000000,0x20000200)`を読み書き可能なzero初期化RAMとしてmap |
| INPUT・OUTPUT | INPUT=`0x20000000`, OUTPUT=`0x20000100`、表の入力wordをINPUTから書き、OUTPUTから64wordを`0xdeadbeef`（SENTINEL）で初期化 |
| 追加mapping | `0xffff8050`から4byteのzero初期化RAM。上記RAMと明記したmapping以外はunmappedとし、address 0もunmapped |
| 終了通知 | `0xffff8040`から4byteを読み書き可能にmapし、初期u32=`0xdeadbeef` |
| CPU | PC=image.entry_point、r14/SP=`0x801000`、r7〜r11=`0xabcd0000 + register番号`、残りregisterは0 |
| 正常終了命令 | callerがmain return後にword列`0x0100ffff,0x2110000f,0x01100040,0xb0100000,0xc000ffff`をlittle-endianで実行し、終了通知へ0を書き込む |
| 観測 | 2,000,000命令以内に正常終了通知または表のfaultで停止。正常時はexit値0、SP初期値、r7〜r11初期値とOUTPUTを照合。fault時は終了通知SENTINELと記載OUTPUTを照合 |

命令数上限はcase実行の打切り条件であり、言語の性能要件ではない。\
入力wordは表のa/bの下位32bitである。

整数のwrap、型幅上のbitwise、符号付き商/剰余、count mod幅、比較のbool結果、runtime cast規則から期待を導く。\
規範は[整数演算](../../language/v1/expressions.md#integer-arithmetic)と[型変換](../../language/v1/types.md#型同一性)、fault停止は[machine](../../targets/kagura-v1/machine.md)に従う。\
期待列のhexはOUTPUTのu32 wordであり、InvalidInstructionはfaultとSENTINELの保持を表す。\


比較の式の結果型はboolであり、他演算とcastの結果型は表の演算対象型/cast先型である。\
観測wordのu32とsource上のresult型を区別する。

sourceのparameterは、Tを表の「演算対象型/cast先型」、Sを「aのsource型」とする。\
比較でもTはoperandの整数型であり、結果型boolとは区別する。\
Rはshiftの場合u32、それ以外はTとする。

EXPRESSIONは通常の二項演算なら`a OP b`、unary+なら`+a`、unary-なら`-a`、unary^なら`^a`、castなら`a as T`とし、OPは表の演算子で置換する。\
RESULTは比較演算の場合、次のsource断片で置換する。

```ond
var r:u32;if EXPRESSION{r=1};*(536871168 as u32 as *u32) = r
```

それ以外のRESULTは次のsource断片とする。

```ond
*(536871168 as u32 as *u32) = (EXPRESSION) as u32
```

`main.ond`は次のtemplateのS、R、RESULTを上記の規則で置換し、EXPRESSION内のT、OPも置換して構成する。\
これにより、比較のbool結果はu32の0/1、それ以外は整数値のu32変換としてOUTPUTへ書き込む。

```ond
package main
func main(){var addressA:u32=536870912;var addressB:u32=536870916;var pointerA = addressA as *u32;var pointerB = addressB as *u32;var a=(*pointerA) as S;var b=(*pointerB) as R;RESULT}
```

| Case ID | 要件 | 演算対象型/cast先型 | aのsource型 | 演算 | a | b | 仕様由来の期待word/停止 |
| --- | --- | --- | --- | --- | ---: | ---: | --- |
| <code>NUM-05.runtime.u8.add</code> | <code>NUM-05</code> | <code>u8</code> | <code>u8</code> | <code>+</code> | <code>6</code> | <code>3</code> | <code>0x00000009</code> |
| <code>NUM-05.runtime.u8.sub</code> | <code>NUM-05</code> | <code>u8</code> | <code>u8</code> | <code>-</code> | <code>6</code> | <code>3</code> | <code>0x00000003</code> |
| <code>NUM-05.runtime.u8.mul</code> | <code>NUM-05</code> | <code>u8</code> | <code>u8</code> | <code>*</code> | <code>6</code> | <code>3</code> | <code>0x00000012</code> |
| <code>NUM-04.runtime.u8.div</code> | <code>NUM-04</code> | <code>u8</code> | <code>u8</code> | <code>/</code> | <code>6</code> | <code>3</code> | <code>0x00000002</code> |
| <code>NUM-04.runtime.u8.rem</code> | <code>NUM-04</code> | <code>u8</code> | <code>u8</code> | <code>%</code> | <code>6</code> | <code>3</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.u8.and</code> | <code>NUM-05</code> | <code>u8</code> | <code>u8</code> | <code>&amp;</code> | <code>6</code> | <code>3</code> | <code>0x00000002</code> |
| <code>NUM-05.runtime.u8.or</code> | <code>NUM-05</code> | <code>u8</code> | <code>u8</code> | <code>&#124;</code> | <code>6</code> | <code>3</code> | <code>0x00000007</code> |
| <code>NUM-05.runtime.u8.xor</code> | <code>NUM-05</code> | <code>u8</code> | <code>u8</code> | <code>^</code> | <code>6</code> | <code>3</code> | <code>0x00000005</code> |
| <code>NUM-05.runtime.u8.andnot</code> | <code>NUM-05</code> | <code>u8</code> | <code>u8</code> | <code>&amp;^</code> | <code>6</code> | <code>3</code> | <code>0x00000004</code> |
| <code>NUM-05.runtime.u8.eq</code> | <code>NUM-05</code> | <code>u8</code> | <code>u8</code> | <code>==</code> | <code>6</code> | <code>3</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.u8.ne</code> | <code>NUM-05</code> | <code>u8</code> | <code>u8</code> | <code>!=</code> | <code>6</code> | <code>3</code> | <code>0x00000001</code> |
| <code>NUM-05.runtime.u8.lt</code> | <code>NUM-05</code> | <code>u8</code> | <code>u8</code> | <code>&lt;</code> | <code>6</code> | <code>3</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.u8.le</code> | <code>NUM-05</code> | <code>u8</code> | <code>u8</code> | <code>&lt;=</code> | <code>6</code> | <code>3</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.u8.gt</code> | <code>NUM-05</code> | <code>u8</code> | <code>u8</code> | <code>&gt;</code> | <code>6</code> | <code>3</code> | <code>0x00000001</code> |
| <code>NUM-05.runtime.u8.ge</code> | <code>NUM-05</code> | <code>u8</code> | <code>u8</code> | <code>&gt;=</code> | <code>6</code> | <code>3</code> | <code>0x00000001</code> |
| <code>NUM-05.runtime.u8.wrap_add</code> | <code>NUM-05</code> | <code>u8</code> | <code>u8</code> | <code>+</code> | <code>255</code> | <code>1</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.u8.wrap_sub</code> | <code>NUM-05</code> | <code>u8</code> | <code>u8</code> | <code>-</code> | <code>0</code> | <code>1</code> | <code>0x000000ff</code> |
| <code>NUM-05.runtime.u8.wrap_mul</code> | <code>NUM-05</code> | <code>u8</code> | <code>u8</code> | <code>*</code> | <code>255</code> | <code>2</code> | <code>0x000000fe</code> |
| <code>NUM-05.runtime.u8.positive</code> | <code>NUM-05</code> | <code>u8</code> | <code>u8</code> | <code>unary+</code> | <code>255</code> | <code>0</code> | <code>0x000000ff</code> |
| <code>NUM-05.runtime.u8.negative</code> | <code>NUM-05</code> | <code>u8</code> | <code>u8</code> | <code>unary-</code> | <code>1</code> | <code>0</code> | <code>0x000000ff</code> |
| <code>NUM-05.runtime.u8.not</code> | <code>NUM-05</code> | <code>u8</code> | <code>u8</code> | <code>unary^</code> | <code>0</code> | <code>0</code> | <code>0x000000ff</code> |
| <code>NUM-05.runtime.u8.boundary_eq</code> | <code>NUM-05</code> | <code>u8</code> | <code>u8</code> | <code>==</code> | <code>0</code> | <code>255</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.u8.boundary_ne</code> | <code>NUM-05</code> | <code>u8</code> | <code>u8</code> | <code>!=</code> | <code>0</code> | <code>255</code> | <code>0x00000001</code> |
| <code>NUM-05.runtime.u8.boundary_lt</code> | <code>NUM-05</code> | <code>u8</code> | <code>u8</code> | <code>&lt;</code> | <code>0</code> | <code>255</code> | <code>0x00000001</code> |
| <code>NUM-05.runtime.u8.boundary_le</code> | <code>NUM-05</code> | <code>u8</code> | <code>u8</code> | <code>&lt;=</code> | <code>0</code> | <code>255</code> | <code>0x00000001</code> |
| <code>NUM-05.runtime.u8.boundary_gt</code> | <code>NUM-05</code> | <code>u8</code> | <code>u8</code> | <code>&gt;</code> | <code>0</code> | <code>255</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.u8.boundary_ge</code> | <code>NUM-05</code> | <code>u8</code> | <code>u8</code> | <code>&gt;=</code> | <code>0</code> | <code>255</code> | <code>0x00000000</code> |
| <code>NUM-03.runtime.u8.left_0</code> | <code>NUM-03</code> | <code>u8</code> | <code>u8</code> | <code>&lt;&lt;</code> | <code>1</code> | <code>0</code> | <code>0x00000001</code> |
| <code>NUM-03.runtime.u8.right_0</code> | <code>NUM-03</code> | <code>u8</code> | <code>u8</code> | <code>&gt;&gt;</code> | <code>0</code> | <code>0</code> | <code>0x00000000</code> |
| <code>NUM-03.runtime.u8.left_7</code> | <code>NUM-03</code> | <code>u8</code> | <code>u8</code> | <code>&lt;&lt;</code> | <code>1</code> | <code>7</code> | <code>0x00000080</code> |
| <code>NUM-03.runtime.u8.right_7</code> | <code>NUM-03</code> | <code>u8</code> | <code>u8</code> | <code>&gt;&gt;</code> | <code>0</code> | <code>7</code> | <code>0x00000000</code> |
| <code>NUM-03.runtime.u8.left_8</code> | <code>NUM-03</code> | <code>u8</code> | <code>u8</code> | <code>&lt;&lt;</code> | <code>1</code> | <code>8</code> | <code>0x00000001</code> |
| <code>NUM-03.runtime.u8.right_8</code> | <code>NUM-03</code> | <code>u8</code> | <code>u8</code> | <code>&gt;&gt;</code> | <code>0</code> | <code>8</code> | <code>0x00000000</code> |
| <code>NUM-03.runtime.u8.left_9</code> | <code>NUM-03</code> | <code>u8</code> | <code>u8</code> | <code>&lt;&lt;</code> | <code>1</code> | <code>9</code> | <code>0x00000002</code> |
| <code>NUM-03.runtime.u8.right_9</code> | <code>NUM-03</code> | <code>u8</code> | <code>u8</code> | <code>&gt;&gt;</code> | <code>0</code> | <code>9</code> | <code>0x00000000</code> |
| <code>NUM-03.runtime.u8.left_4294967295</code> | <code>NUM-03</code> | <code>u8</code> | <code>u8</code> | <code>&lt;&lt;</code> | <code>1</code> | <code>4294967295</code> | <code>0x00000080</code> |
| <code>NUM-03.runtime.u8.right_4294967295</code> | <code>NUM-03</code> | <code>u8</code> | <code>u8</code> | <code>&gt;&gt;</code> | <code>0</code> | <code>4294967295</code> | <code>0x00000000</code> |
| <code>NUM-04.runtime.u8.div_zero</code> | <code>NUM-04</code> | <code>u8</code> | <code>u8</code> | <code>/</code> | <code>1</code> | <code>0</code> | <code>InvalidInstruction; 0xdeadbeef</code> |
| <code>NUM-04.runtime.u8.rem_zero</code> | <code>NUM-04</code> | <code>u8</code> | <code>u8</code> | <code>%</code> | <code>1</code> | <code>0</code> | <code>InvalidInstruction; 0xdeadbeef</code> |
| <code>NUM-05.runtime.i8.add</code> | <code>NUM-05</code> | <code>i8</code> | <code>i8</code> | <code>+</code> | <code>6</code> | <code>3</code> | <code>0x00000009</code> |
| <code>NUM-05.runtime.i8.sub</code> | <code>NUM-05</code> | <code>i8</code> | <code>i8</code> | <code>-</code> | <code>6</code> | <code>3</code> | <code>0x00000003</code> |
| <code>NUM-05.runtime.i8.mul</code> | <code>NUM-05</code> | <code>i8</code> | <code>i8</code> | <code>*</code> | <code>6</code> | <code>3</code> | <code>0x00000012</code> |
| <code>NUM-04.runtime.i8.div</code> | <code>NUM-04</code> | <code>i8</code> | <code>i8</code> | <code>/</code> | <code>6</code> | <code>3</code> | <code>0x00000002</code> |
| <code>NUM-04.runtime.i8.rem</code> | <code>NUM-04</code> | <code>i8</code> | <code>i8</code> | <code>%</code> | <code>6</code> | <code>3</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.i8.and</code> | <code>NUM-05</code> | <code>i8</code> | <code>i8</code> | <code>&amp;</code> | <code>6</code> | <code>3</code> | <code>0x00000002</code> |
| <code>NUM-05.runtime.i8.or</code> | <code>NUM-05</code> | <code>i8</code> | <code>i8</code> | <code>&#124;</code> | <code>6</code> | <code>3</code> | <code>0x00000007</code> |
| <code>NUM-05.runtime.i8.xor</code> | <code>NUM-05</code> | <code>i8</code> | <code>i8</code> | <code>^</code> | <code>6</code> | <code>3</code> | <code>0x00000005</code> |
| <code>NUM-05.runtime.i8.andnot</code> | <code>NUM-05</code> | <code>i8</code> | <code>i8</code> | <code>&amp;^</code> | <code>6</code> | <code>3</code> | <code>0x00000004</code> |
| <code>NUM-05.runtime.i8.eq</code> | <code>NUM-05</code> | <code>i8</code> | <code>i8</code> | <code>==</code> | <code>6</code> | <code>3</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.i8.ne</code> | <code>NUM-05</code> | <code>i8</code> | <code>i8</code> | <code>!=</code> | <code>6</code> | <code>3</code> | <code>0x00000001</code> |
| <code>NUM-05.runtime.i8.lt</code> | <code>NUM-05</code> | <code>i8</code> | <code>i8</code> | <code>&lt;</code> | <code>6</code> | <code>3</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.i8.le</code> | <code>NUM-05</code> | <code>i8</code> | <code>i8</code> | <code>&lt;=</code> | <code>6</code> | <code>3</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.i8.gt</code> | <code>NUM-05</code> | <code>i8</code> | <code>i8</code> | <code>&gt;</code> | <code>6</code> | <code>3</code> | <code>0x00000001</code> |
| <code>NUM-05.runtime.i8.ge</code> | <code>NUM-05</code> | <code>i8</code> | <code>i8</code> | <code>&gt;=</code> | <code>6</code> | <code>3</code> | <code>0x00000001</code> |
| <code>NUM-05.runtime.i8.wrap_add</code> | <code>NUM-05</code> | <code>i8</code> | <code>i8</code> | <code>+</code> | <code>127</code> | <code>1</code> | <code>0xffffff80</code> |
| <code>NUM-05.runtime.i8.wrap_sub</code> | <code>NUM-05</code> | <code>i8</code> | <code>i8</code> | <code>-</code> | <code>-128</code> | <code>1</code> | <code>0x0000007f</code> |
| <code>NUM-05.runtime.i8.wrap_mul</code> | <code>NUM-05</code> | <code>i8</code> | <code>i8</code> | <code>*</code> | <code>127</code> | <code>2</code> | <code>0xfffffffe</code> |
| <code>NUM-05.runtime.i8.positive</code> | <code>NUM-05</code> | <code>i8</code> | <code>i8</code> | <code>unary+</code> | <code>127</code> | <code>0</code> | <code>0x0000007f</code> |
| <code>NUM-05.runtime.i8.negative</code> | <code>NUM-05</code> | <code>i8</code> | <code>i8</code> | <code>unary-</code> | <code>-128</code> | <code>0</code> | <code>0xffffff80</code> |
| <code>NUM-05.runtime.i8.not</code> | <code>NUM-05</code> | <code>i8</code> | <code>i8</code> | <code>unary^</code> | <code>0</code> | <code>0</code> | <code>0xffffffff</code> |
| <code>NUM-05.runtime.i8.boundary_eq</code> | <code>NUM-05</code> | <code>i8</code> | <code>i8</code> | <code>==</code> | <code>-128</code> | <code>127</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.i8.boundary_ne</code> | <code>NUM-05</code> | <code>i8</code> | <code>i8</code> | <code>!=</code> | <code>-128</code> | <code>127</code> | <code>0x00000001</code> |
| <code>NUM-05.runtime.i8.boundary_lt</code> | <code>NUM-05</code> | <code>i8</code> | <code>i8</code> | <code>&lt;</code> | <code>-128</code> | <code>127</code> | <code>0x00000001</code> |
| <code>NUM-05.runtime.i8.boundary_le</code> | <code>NUM-05</code> | <code>i8</code> | <code>i8</code> | <code>&lt;=</code> | <code>-128</code> | <code>127</code> | <code>0x00000001</code> |
| <code>NUM-05.runtime.i8.boundary_gt</code> | <code>NUM-05</code> | <code>i8</code> | <code>i8</code> | <code>&gt;</code> | <code>-128</code> | <code>127</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.i8.boundary_ge</code> | <code>NUM-05</code> | <code>i8</code> | <code>i8</code> | <code>&gt;=</code> | <code>-128</code> | <code>127</code> | <code>0x00000000</code> |
| <code>NUM-03.runtime.i8.left_0</code> | <code>NUM-03</code> | <code>i8</code> | <code>i8</code> | <code>&lt;&lt;</code> | <code>1</code> | <code>0</code> | <code>0x00000001</code> |
| <code>NUM-03.runtime.i8.right_0</code> | <code>NUM-03</code> | <code>i8</code> | <code>i8</code> | <code>&gt;&gt;</code> | <code>-128</code> | <code>0</code> | <code>0xffffff80</code> |
| <code>NUM-03.runtime.i8.left_7</code> | <code>NUM-03</code> | <code>i8</code> | <code>i8</code> | <code>&lt;&lt;</code> | <code>1</code> | <code>7</code> | <code>0xffffff80</code> |
| <code>NUM-03.runtime.i8.right_7</code> | <code>NUM-03</code> | <code>i8</code> | <code>i8</code> | <code>&gt;&gt;</code> | <code>-128</code> | <code>7</code> | <code>0xffffffff</code> |
| <code>NUM-03.runtime.i8.left_8</code> | <code>NUM-03</code> | <code>i8</code> | <code>i8</code> | <code>&lt;&lt;</code> | <code>1</code> | <code>8</code> | <code>0x00000001</code> |
| <code>NUM-03.runtime.i8.right_8</code> | <code>NUM-03</code> | <code>i8</code> | <code>i8</code> | <code>&gt;&gt;</code> | <code>-128</code> | <code>8</code> | <code>0xffffff80</code> |
| <code>NUM-03.runtime.i8.left_9</code> | <code>NUM-03</code> | <code>i8</code> | <code>i8</code> | <code>&lt;&lt;</code> | <code>1</code> | <code>9</code> | <code>0x00000002</code> |
| <code>NUM-03.runtime.i8.right_9</code> | <code>NUM-03</code> | <code>i8</code> | <code>i8</code> | <code>&gt;&gt;</code> | <code>-128</code> | <code>9</code> | <code>0xffffffc0</code> |
| <code>NUM-03.runtime.i8.left_4294967295</code> | <code>NUM-03</code> | <code>i8</code> | <code>i8</code> | <code>&lt;&lt;</code> | <code>1</code> | <code>4294967295</code> | <code>0xffffff80</code> |
| <code>NUM-03.runtime.i8.right_4294967295</code> | <code>NUM-03</code> | <code>i8</code> | <code>i8</code> | <code>&gt;&gt;</code> | <code>-128</code> | <code>4294967295</code> | <code>0xffffffff</code> |
| <code>NUM-04.runtime.i8.div_zero</code> | <code>NUM-04</code> | <code>i8</code> | <code>i8</code> | <code>/</code> | <code>1</code> | <code>0</code> | <code>InvalidInstruction; 0xdeadbeef</code> |
| <code>NUM-04.runtime.i8.rem_zero</code> | <code>NUM-04</code> | <code>i8</code> | <code>i8</code> | <code>%</code> | <code>1</code> | <code>0</code> | <code>InvalidInstruction; 0xdeadbeef</code> |
| <code>NUM-04.runtime.i8.div_min</code> | <code>NUM-04</code> | <code>i8</code> | <code>i8</code> | <code>/</code> | <code>-128</code> | <code>-1</code> | <code>InvalidInstruction; 0xdeadbeef</code> |
| <code>NUM-04.runtime.i8.rem_min</code> | <code>NUM-04</code> | <code>i8</code> | <code>i8</code> | <code>%</code> | <code>-128</code> | <code>-1</code> | <code>0x00000000</code> |
| <code>NUM-04.runtime.i8.div_pp</code> | <code>NUM-04</code> | <code>i8</code> | <code>i8</code> | <code>/</code> | <code>7</code> | <code>3</code> | <code>0x00000002</code> |
| <code>NUM-04.runtime.i8.rem_pp</code> | <code>NUM-04</code> | <code>i8</code> | <code>i8</code> | <code>%</code> | <code>7</code> | <code>3</code> | <code>0x00000001</code> |
| <code>NUM-04.runtime.i8.div_np</code> | <code>NUM-04</code> | <code>i8</code> | <code>i8</code> | <code>/</code> | <code>-7</code> | <code>3</code> | <code>0xfffffffe</code> |
| <code>NUM-04.runtime.i8.rem_np</code> | <code>NUM-04</code> | <code>i8</code> | <code>i8</code> | <code>%</code> | <code>-7</code> | <code>3</code> | <code>0xffffffff</code> |
| <code>NUM-04.runtime.i8.div_pn</code> | <code>NUM-04</code> | <code>i8</code> | <code>i8</code> | <code>/</code> | <code>7</code> | <code>-3</code> | <code>0xfffffffe</code> |
| <code>NUM-04.runtime.i8.rem_pn</code> | <code>NUM-04</code> | <code>i8</code> | <code>i8</code> | <code>%</code> | <code>7</code> | <code>-3</code> | <code>0x00000001</code> |
| <code>NUM-04.runtime.i8.div_nn</code> | <code>NUM-04</code> | <code>i8</code> | <code>i8</code> | <code>/</code> | <code>-7</code> | <code>-3</code> | <code>0x00000002</code> |
| <code>NUM-04.runtime.i8.rem_nn</code> | <code>NUM-04</code> | <code>i8</code> | <code>i8</code> | <code>%</code> | <code>-7</code> | <code>-3</code> | <code>0xffffffff</code> |
| <code>NUM-05.runtime.u16.add</code> | <code>NUM-05</code> | <code>u16</code> | <code>u16</code> | <code>+</code> | <code>6</code> | <code>3</code> | <code>0x00000009</code> |
| <code>NUM-05.runtime.u16.sub</code> | <code>NUM-05</code> | <code>u16</code> | <code>u16</code> | <code>-</code> | <code>6</code> | <code>3</code> | <code>0x00000003</code> |
| <code>NUM-05.runtime.u16.mul</code> | <code>NUM-05</code> | <code>u16</code> | <code>u16</code> | <code>*</code> | <code>6</code> | <code>3</code> | <code>0x00000012</code> |
| <code>NUM-04.runtime.u16.div</code> | <code>NUM-04</code> | <code>u16</code> | <code>u16</code> | <code>/</code> | <code>6</code> | <code>3</code> | <code>0x00000002</code> |
| <code>NUM-04.runtime.u16.rem</code> | <code>NUM-04</code> | <code>u16</code> | <code>u16</code> | <code>%</code> | <code>6</code> | <code>3</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.u16.and</code> | <code>NUM-05</code> | <code>u16</code> | <code>u16</code> | <code>&amp;</code> | <code>6</code> | <code>3</code> | <code>0x00000002</code> |
| <code>NUM-05.runtime.u16.or</code> | <code>NUM-05</code> | <code>u16</code> | <code>u16</code> | <code>&#124;</code> | <code>6</code> | <code>3</code> | <code>0x00000007</code> |
| <code>NUM-05.runtime.u16.xor</code> | <code>NUM-05</code> | <code>u16</code> | <code>u16</code> | <code>^</code> | <code>6</code> | <code>3</code> | <code>0x00000005</code> |
| <code>NUM-05.runtime.u16.andnot</code> | <code>NUM-05</code> | <code>u16</code> | <code>u16</code> | <code>&amp;^</code> | <code>6</code> | <code>3</code> | <code>0x00000004</code> |
| <code>NUM-05.runtime.u16.eq</code> | <code>NUM-05</code> | <code>u16</code> | <code>u16</code> | <code>==</code> | <code>6</code> | <code>3</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.u16.ne</code> | <code>NUM-05</code> | <code>u16</code> | <code>u16</code> | <code>!=</code> | <code>6</code> | <code>3</code> | <code>0x00000001</code> |
| <code>NUM-05.runtime.u16.lt</code> | <code>NUM-05</code> | <code>u16</code> | <code>u16</code> | <code>&lt;</code> | <code>6</code> | <code>3</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.u16.le</code> | <code>NUM-05</code> | <code>u16</code> | <code>u16</code> | <code>&lt;=</code> | <code>6</code> | <code>3</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.u16.gt</code> | <code>NUM-05</code> | <code>u16</code> | <code>u16</code> | <code>&gt;</code> | <code>6</code> | <code>3</code> | <code>0x00000001</code> |
| <code>NUM-05.runtime.u16.ge</code> | <code>NUM-05</code> | <code>u16</code> | <code>u16</code> | <code>&gt;=</code> | <code>6</code> | <code>3</code> | <code>0x00000001</code> |
| <code>NUM-05.runtime.u16.wrap_add</code> | <code>NUM-05</code> | <code>u16</code> | <code>u16</code> | <code>+</code> | <code>65535</code> | <code>1</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.u16.wrap_sub</code> | <code>NUM-05</code> | <code>u16</code> | <code>u16</code> | <code>-</code> | <code>0</code> | <code>1</code> | <code>0x0000ffff</code> |
| <code>NUM-05.runtime.u16.wrap_mul</code> | <code>NUM-05</code> | <code>u16</code> | <code>u16</code> | <code>*</code> | <code>65535</code> | <code>2</code> | <code>0x0000fffe</code> |
| <code>NUM-05.runtime.u16.positive</code> | <code>NUM-05</code> | <code>u16</code> | <code>u16</code> | <code>unary+</code> | <code>65535</code> | <code>0</code> | <code>0x0000ffff</code> |
| <code>NUM-05.runtime.u16.negative</code> | <code>NUM-05</code> | <code>u16</code> | <code>u16</code> | <code>unary-</code> | <code>1</code> | <code>0</code> | <code>0x0000ffff</code> |
| <code>NUM-05.runtime.u16.not</code> | <code>NUM-05</code> | <code>u16</code> | <code>u16</code> | <code>unary^</code> | <code>0</code> | <code>0</code> | <code>0x0000ffff</code> |
| <code>NUM-05.runtime.u16.boundary_eq</code> | <code>NUM-05</code> | <code>u16</code> | <code>u16</code> | <code>==</code> | <code>0</code> | <code>65535</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.u16.boundary_ne</code> | <code>NUM-05</code> | <code>u16</code> | <code>u16</code> | <code>!=</code> | <code>0</code> | <code>65535</code> | <code>0x00000001</code> |
| <code>NUM-05.runtime.u16.boundary_lt</code> | <code>NUM-05</code> | <code>u16</code> | <code>u16</code> | <code>&lt;</code> | <code>0</code> | <code>65535</code> | <code>0x00000001</code> |
| <code>NUM-05.runtime.u16.boundary_le</code> | <code>NUM-05</code> | <code>u16</code> | <code>u16</code> | <code>&lt;=</code> | <code>0</code> | <code>65535</code> | <code>0x00000001</code> |
| <code>NUM-05.runtime.u16.boundary_gt</code> | <code>NUM-05</code> | <code>u16</code> | <code>u16</code> | <code>&gt;</code> | <code>0</code> | <code>65535</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.u16.boundary_ge</code> | <code>NUM-05</code> | <code>u16</code> | <code>u16</code> | <code>&gt;=</code> | <code>0</code> | <code>65535</code> | <code>0x00000000</code> |
| <code>NUM-03.runtime.u16.left_0</code> | <code>NUM-03</code> | <code>u16</code> | <code>u16</code> | <code>&lt;&lt;</code> | <code>1</code> | <code>0</code> | <code>0x00000001</code> |
| <code>NUM-03.runtime.u16.right_0</code> | <code>NUM-03</code> | <code>u16</code> | <code>u16</code> | <code>&gt;&gt;</code> | <code>0</code> | <code>0</code> | <code>0x00000000</code> |
| <code>NUM-03.runtime.u16.left_15</code> | <code>NUM-03</code> | <code>u16</code> | <code>u16</code> | <code>&lt;&lt;</code> | <code>1</code> | <code>15</code> | <code>0x00008000</code> |
| <code>NUM-03.runtime.u16.right_15</code> | <code>NUM-03</code> | <code>u16</code> | <code>u16</code> | <code>&gt;&gt;</code> | <code>0</code> | <code>15</code> | <code>0x00000000</code> |
| <code>NUM-03.runtime.u16.left_16</code> | <code>NUM-03</code> | <code>u16</code> | <code>u16</code> | <code>&lt;&lt;</code> | <code>1</code> | <code>16</code> | <code>0x00000001</code> |
| <code>NUM-03.runtime.u16.right_16</code> | <code>NUM-03</code> | <code>u16</code> | <code>u16</code> | <code>&gt;&gt;</code> | <code>0</code> | <code>16</code> | <code>0x00000000</code> |
| <code>NUM-03.runtime.u16.left_17</code> | <code>NUM-03</code> | <code>u16</code> | <code>u16</code> | <code>&lt;&lt;</code> | <code>1</code> | <code>17</code> | <code>0x00000002</code> |
| <code>NUM-03.runtime.u16.right_17</code> | <code>NUM-03</code> | <code>u16</code> | <code>u16</code> | <code>&gt;&gt;</code> | <code>0</code> | <code>17</code> | <code>0x00000000</code> |
| <code>NUM-03.runtime.u16.left_4294967295</code> | <code>NUM-03</code> | <code>u16</code> | <code>u16</code> | <code>&lt;&lt;</code> | <code>1</code> | <code>4294967295</code> | <code>0x00008000</code> |
| <code>NUM-03.runtime.u16.right_4294967295</code> | <code>NUM-03</code> | <code>u16</code> | <code>u16</code> | <code>&gt;&gt;</code> | <code>0</code> | <code>4294967295</code> | <code>0x00000000</code> |
| <code>NUM-04.runtime.u16.div_zero</code> | <code>NUM-04</code> | <code>u16</code> | <code>u16</code> | <code>/</code> | <code>1</code> | <code>0</code> | <code>InvalidInstruction; 0xdeadbeef</code> |
| <code>NUM-04.runtime.u16.rem_zero</code> | <code>NUM-04</code> | <code>u16</code> | <code>u16</code> | <code>%</code> | <code>1</code> | <code>0</code> | <code>InvalidInstruction; 0xdeadbeef</code> |
| <code>NUM-05.runtime.i16.add</code> | <code>NUM-05</code> | <code>i16</code> | <code>i16</code> | <code>+</code> | <code>6</code> | <code>3</code> | <code>0x00000009</code> |
| <code>NUM-05.runtime.i16.sub</code> | <code>NUM-05</code> | <code>i16</code> | <code>i16</code> | <code>-</code> | <code>6</code> | <code>3</code> | <code>0x00000003</code> |
| <code>NUM-05.runtime.i16.mul</code> | <code>NUM-05</code> | <code>i16</code> | <code>i16</code> | <code>*</code> | <code>6</code> | <code>3</code> | <code>0x00000012</code> |
| <code>NUM-04.runtime.i16.div</code> | <code>NUM-04</code> | <code>i16</code> | <code>i16</code> | <code>/</code> | <code>6</code> | <code>3</code> | <code>0x00000002</code> |
| <code>NUM-04.runtime.i16.rem</code> | <code>NUM-04</code> | <code>i16</code> | <code>i16</code> | <code>%</code> | <code>6</code> | <code>3</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.i16.and</code> | <code>NUM-05</code> | <code>i16</code> | <code>i16</code> | <code>&amp;</code> | <code>6</code> | <code>3</code> | <code>0x00000002</code> |
| <code>NUM-05.runtime.i16.or</code> | <code>NUM-05</code> | <code>i16</code> | <code>i16</code> | <code>&#124;</code> | <code>6</code> | <code>3</code> | <code>0x00000007</code> |
| <code>NUM-05.runtime.i16.xor</code> | <code>NUM-05</code> | <code>i16</code> | <code>i16</code> | <code>^</code> | <code>6</code> | <code>3</code> | <code>0x00000005</code> |
| <code>NUM-05.runtime.i16.andnot</code> | <code>NUM-05</code> | <code>i16</code> | <code>i16</code> | <code>&amp;^</code> | <code>6</code> | <code>3</code> | <code>0x00000004</code> |
| <code>NUM-05.runtime.i16.eq</code> | <code>NUM-05</code> | <code>i16</code> | <code>i16</code> | <code>==</code> | <code>6</code> | <code>3</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.i16.ne</code> | <code>NUM-05</code> | <code>i16</code> | <code>i16</code> | <code>!=</code> | <code>6</code> | <code>3</code> | <code>0x00000001</code> |
| <code>NUM-05.runtime.i16.lt</code> | <code>NUM-05</code> | <code>i16</code> | <code>i16</code> | <code>&lt;</code> | <code>6</code> | <code>3</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.i16.le</code> | <code>NUM-05</code> | <code>i16</code> | <code>i16</code> | <code>&lt;=</code> | <code>6</code> | <code>3</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.i16.gt</code> | <code>NUM-05</code> | <code>i16</code> | <code>i16</code> | <code>&gt;</code> | <code>6</code> | <code>3</code> | <code>0x00000001</code> |
| <code>NUM-05.runtime.i16.ge</code> | <code>NUM-05</code> | <code>i16</code> | <code>i16</code> | <code>&gt;=</code> | <code>6</code> | <code>3</code> | <code>0x00000001</code> |
| <code>NUM-05.runtime.i16.wrap_add</code> | <code>NUM-05</code> | <code>i16</code> | <code>i16</code> | <code>+</code> | <code>32767</code> | <code>1</code> | <code>0xffff8000</code> |
| <code>NUM-05.runtime.i16.wrap_sub</code> | <code>NUM-05</code> | <code>i16</code> | <code>i16</code> | <code>-</code> | <code>-32768</code> | <code>1</code> | <code>0x00007fff</code> |
| <code>NUM-05.runtime.i16.wrap_mul</code> | <code>NUM-05</code> | <code>i16</code> | <code>i16</code> | <code>*</code> | <code>32767</code> | <code>2</code> | <code>0xfffffffe</code> |
| <code>NUM-05.runtime.i16.positive</code> | <code>NUM-05</code> | <code>i16</code> | <code>i16</code> | <code>unary+</code> | <code>32767</code> | <code>0</code> | <code>0x00007fff</code> |
| <code>NUM-05.runtime.i16.negative</code> | <code>NUM-05</code> | <code>i16</code> | <code>i16</code> | <code>unary-</code> | <code>-32768</code> | <code>0</code> | <code>0xffff8000</code> |
| <code>NUM-05.runtime.i16.not</code> | <code>NUM-05</code> | <code>i16</code> | <code>i16</code> | <code>unary^</code> | <code>0</code> | <code>0</code> | <code>0xffffffff</code> |
| <code>NUM-05.runtime.i16.boundary_eq</code> | <code>NUM-05</code> | <code>i16</code> | <code>i16</code> | <code>==</code> | <code>-32768</code> | <code>32767</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.i16.boundary_ne</code> | <code>NUM-05</code> | <code>i16</code> | <code>i16</code> | <code>!=</code> | <code>-32768</code> | <code>32767</code> | <code>0x00000001</code> |
| <code>NUM-05.runtime.i16.boundary_lt</code> | <code>NUM-05</code> | <code>i16</code> | <code>i16</code> | <code>&lt;</code> | <code>-32768</code> | <code>32767</code> | <code>0x00000001</code> |
| <code>NUM-05.runtime.i16.boundary_le</code> | <code>NUM-05</code> | <code>i16</code> | <code>i16</code> | <code>&lt;=</code> | <code>-32768</code> | <code>32767</code> | <code>0x00000001</code> |
| <code>NUM-05.runtime.i16.boundary_gt</code> | <code>NUM-05</code> | <code>i16</code> | <code>i16</code> | <code>&gt;</code> | <code>-32768</code> | <code>32767</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.i16.boundary_ge</code> | <code>NUM-05</code> | <code>i16</code> | <code>i16</code> | <code>&gt;=</code> | <code>-32768</code> | <code>32767</code> | <code>0x00000000</code> |
| <code>NUM-03.runtime.i16.left_0</code> | <code>NUM-03</code> | <code>i16</code> | <code>i16</code> | <code>&lt;&lt;</code> | <code>1</code> | <code>0</code> | <code>0x00000001</code> |
| <code>NUM-03.runtime.i16.right_0</code> | <code>NUM-03</code> | <code>i16</code> | <code>i16</code> | <code>&gt;&gt;</code> | <code>-32768</code> | <code>0</code> | <code>0xffff8000</code> |
| <code>NUM-03.runtime.i16.left_15</code> | <code>NUM-03</code> | <code>i16</code> | <code>i16</code> | <code>&lt;&lt;</code> | <code>1</code> | <code>15</code> | <code>0xffff8000</code> |
| <code>NUM-03.runtime.i16.right_15</code> | <code>NUM-03</code> | <code>i16</code> | <code>i16</code> | <code>&gt;&gt;</code> | <code>-32768</code> | <code>15</code> | <code>0xffffffff</code> |
| <code>NUM-03.runtime.i16.left_16</code> | <code>NUM-03</code> | <code>i16</code> | <code>i16</code> | <code>&lt;&lt;</code> | <code>1</code> | <code>16</code> | <code>0x00000001</code> |
| <code>NUM-03.runtime.i16.right_16</code> | <code>NUM-03</code> | <code>i16</code> | <code>i16</code> | <code>&gt;&gt;</code> | <code>-32768</code> | <code>16</code> | <code>0xffff8000</code> |
| <code>NUM-03.runtime.i16.left_17</code> | <code>NUM-03</code> | <code>i16</code> | <code>i16</code> | <code>&lt;&lt;</code> | <code>1</code> | <code>17</code> | <code>0x00000002</code> |
| <code>NUM-03.runtime.i16.right_17</code> | <code>NUM-03</code> | <code>i16</code> | <code>i16</code> | <code>&gt;&gt;</code> | <code>-32768</code> | <code>17</code> | <code>0xffffc000</code> |
| <code>NUM-03.runtime.i16.left_4294967295</code> | <code>NUM-03</code> | <code>i16</code> | <code>i16</code> | <code>&lt;&lt;</code> | <code>1</code> | <code>4294967295</code> | <code>0xffff8000</code> |
| <code>NUM-03.runtime.i16.right_4294967295</code> | <code>NUM-03</code> | <code>i16</code> | <code>i16</code> | <code>&gt;&gt;</code> | <code>-32768</code> | <code>4294967295</code> | <code>0xffffffff</code> |
| <code>NUM-04.runtime.i16.div_zero</code> | <code>NUM-04</code> | <code>i16</code> | <code>i16</code> | <code>/</code> | <code>1</code> | <code>0</code> | <code>InvalidInstruction; 0xdeadbeef</code> |
| <code>NUM-04.runtime.i16.rem_zero</code> | <code>NUM-04</code> | <code>i16</code> | <code>i16</code> | <code>%</code> | <code>1</code> | <code>0</code> | <code>InvalidInstruction; 0xdeadbeef</code> |
| <code>NUM-04.runtime.i16.div_min</code> | <code>NUM-04</code> | <code>i16</code> | <code>i16</code> | <code>/</code> | <code>-32768</code> | <code>-1</code> | <code>InvalidInstruction; 0xdeadbeef</code> |
| <code>NUM-04.runtime.i16.rem_min</code> | <code>NUM-04</code> | <code>i16</code> | <code>i16</code> | <code>%</code> | <code>-32768</code> | <code>-1</code> | <code>0x00000000</code> |
| <code>NUM-04.runtime.i16.div_pp</code> | <code>NUM-04</code> | <code>i16</code> | <code>i16</code> | <code>/</code> | <code>7</code> | <code>3</code> | <code>0x00000002</code> |
| <code>NUM-04.runtime.i16.rem_pp</code> | <code>NUM-04</code> | <code>i16</code> | <code>i16</code> | <code>%</code> | <code>7</code> | <code>3</code> | <code>0x00000001</code> |
| <code>NUM-04.runtime.i16.div_np</code> | <code>NUM-04</code> | <code>i16</code> | <code>i16</code> | <code>/</code> | <code>-7</code> | <code>3</code> | <code>0xfffffffe</code> |
| <code>NUM-04.runtime.i16.rem_np</code> | <code>NUM-04</code> | <code>i16</code> | <code>i16</code> | <code>%</code> | <code>-7</code> | <code>3</code> | <code>0xffffffff</code> |
| <code>NUM-04.runtime.i16.div_pn</code> | <code>NUM-04</code> | <code>i16</code> | <code>i16</code> | <code>/</code> | <code>7</code> | <code>-3</code> | <code>0xfffffffe</code> |
| <code>NUM-04.runtime.i16.rem_pn</code> | <code>NUM-04</code> | <code>i16</code> | <code>i16</code> | <code>%</code> | <code>7</code> | <code>-3</code> | <code>0x00000001</code> |
| <code>NUM-04.runtime.i16.div_nn</code> | <code>NUM-04</code> | <code>i16</code> | <code>i16</code> | <code>/</code> | <code>-7</code> | <code>-3</code> | <code>0x00000002</code> |
| <code>NUM-04.runtime.i16.rem_nn</code> | <code>NUM-04</code> | <code>i16</code> | <code>i16</code> | <code>%</code> | <code>-7</code> | <code>-3</code> | <code>0xffffffff</code> |
| <code>NUM-05.runtime.u32.add</code> | <code>NUM-05</code> | <code>u32</code> | <code>u32</code> | <code>+</code> | <code>6</code> | <code>3</code> | <code>0x00000009</code> |
| <code>NUM-05.runtime.u32.sub</code> | <code>NUM-05</code> | <code>u32</code> | <code>u32</code> | <code>-</code> | <code>6</code> | <code>3</code> | <code>0x00000003</code> |
| <code>NUM-05.runtime.u32.mul</code> | <code>NUM-05</code> | <code>u32</code> | <code>u32</code> | <code>*</code> | <code>6</code> | <code>3</code> | <code>0x00000012</code> |
| <code>NUM-04.runtime.u32.div</code> | <code>NUM-04</code> | <code>u32</code> | <code>u32</code> | <code>/</code> | <code>6</code> | <code>3</code> | <code>0x00000002</code> |
| <code>NUM-04.runtime.u32.rem</code> | <code>NUM-04</code> | <code>u32</code> | <code>u32</code> | <code>%</code> | <code>6</code> | <code>3</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.u32.and</code> | <code>NUM-05</code> | <code>u32</code> | <code>u32</code> | <code>&amp;</code> | <code>6</code> | <code>3</code> | <code>0x00000002</code> |
| <code>NUM-05.runtime.u32.or</code> | <code>NUM-05</code> | <code>u32</code> | <code>u32</code> | <code>&#124;</code> | <code>6</code> | <code>3</code> | <code>0x00000007</code> |
| <code>NUM-05.runtime.u32.xor</code> | <code>NUM-05</code> | <code>u32</code> | <code>u32</code> | <code>^</code> | <code>6</code> | <code>3</code> | <code>0x00000005</code> |
| <code>NUM-05.runtime.u32.andnot</code> | <code>NUM-05</code> | <code>u32</code> | <code>u32</code> | <code>&amp;^</code> | <code>6</code> | <code>3</code> | <code>0x00000004</code> |
| <code>NUM-05.runtime.u32.eq</code> | <code>NUM-05</code> | <code>u32</code> | <code>u32</code> | <code>==</code> | <code>6</code> | <code>3</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.u32.ne</code> | <code>NUM-05</code> | <code>u32</code> | <code>u32</code> | <code>!=</code> | <code>6</code> | <code>3</code> | <code>0x00000001</code> |
| <code>NUM-05.runtime.u32.lt</code> | <code>NUM-05</code> | <code>u32</code> | <code>u32</code> | <code>&lt;</code> | <code>6</code> | <code>3</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.u32.le</code> | <code>NUM-05</code> | <code>u32</code> | <code>u32</code> | <code>&lt;=</code> | <code>6</code> | <code>3</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.u32.gt</code> | <code>NUM-05</code> | <code>u32</code> | <code>u32</code> | <code>&gt;</code> | <code>6</code> | <code>3</code> | <code>0x00000001</code> |
| <code>NUM-05.runtime.u32.ge</code> | <code>NUM-05</code> | <code>u32</code> | <code>u32</code> | <code>&gt;=</code> | <code>6</code> | <code>3</code> | <code>0x00000001</code> |
| <code>NUM-05.runtime.u32.wrap_add</code> | <code>NUM-05</code> | <code>u32</code> | <code>u32</code> | <code>+</code> | <code>4294967295</code> | <code>1</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.u32.wrap_sub</code> | <code>NUM-05</code> | <code>u32</code> | <code>u32</code> | <code>-</code> | <code>0</code> | <code>1</code> | <code>0xffffffff</code> |
| <code>NUM-05.runtime.u32.wrap_mul</code> | <code>NUM-05</code> | <code>u32</code> | <code>u32</code> | <code>*</code> | <code>4294967295</code> | <code>2</code> | <code>0xfffffffe</code> |
| <code>NUM-05.runtime.u32.positive</code> | <code>NUM-05</code> | <code>u32</code> | <code>u32</code> | <code>unary+</code> | <code>4294967295</code> | <code>0</code> | <code>0xffffffff</code> |
| <code>NUM-05.runtime.u32.negative</code> | <code>NUM-05</code> | <code>u32</code> | <code>u32</code> | <code>unary-</code> | <code>1</code> | <code>0</code> | <code>0xffffffff</code> |
| <code>NUM-05.runtime.u32.not</code> | <code>NUM-05</code> | <code>u32</code> | <code>u32</code> | <code>unary^</code> | <code>0</code> | <code>0</code> | <code>0xffffffff</code> |
| <code>NUM-05.runtime.u32.boundary_eq</code> | <code>NUM-05</code> | <code>u32</code> | <code>u32</code> | <code>==</code> | <code>0</code> | <code>4294967295</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.u32.boundary_ne</code> | <code>NUM-05</code> | <code>u32</code> | <code>u32</code> | <code>!=</code> | <code>0</code> | <code>4294967295</code> | <code>0x00000001</code> |
| <code>NUM-05.runtime.u32.boundary_lt</code> | <code>NUM-05</code> | <code>u32</code> | <code>u32</code> | <code>&lt;</code> | <code>0</code> | <code>4294967295</code> | <code>0x00000001</code> |
| <code>NUM-05.runtime.u32.boundary_le</code> | <code>NUM-05</code> | <code>u32</code> | <code>u32</code> | <code>&lt;=</code> | <code>0</code> | <code>4294967295</code> | <code>0x00000001</code> |
| <code>NUM-05.runtime.u32.boundary_gt</code> | <code>NUM-05</code> | <code>u32</code> | <code>u32</code> | <code>&gt;</code> | <code>0</code> | <code>4294967295</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.u32.boundary_ge</code> | <code>NUM-05</code> | <code>u32</code> | <code>u32</code> | <code>&gt;=</code> | <code>0</code> | <code>4294967295</code> | <code>0x00000000</code> |
| <code>NUM-03.runtime.u32.left_0</code> | <code>NUM-03</code> | <code>u32</code> | <code>u32</code> | <code>&lt;&lt;</code> | <code>1</code> | <code>0</code> | <code>0x00000001</code> |
| <code>NUM-03.runtime.u32.right_0</code> | <code>NUM-03</code> | <code>u32</code> | <code>u32</code> | <code>&gt;&gt;</code> | <code>0</code> | <code>0</code> | <code>0x00000000</code> |
| <code>NUM-03.runtime.u32.left_31</code> | <code>NUM-03</code> | <code>u32</code> | <code>u32</code> | <code>&lt;&lt;</code> | <code>1</code> | <code>31</code> | <code>0x80000000</code> |
| <code>NUM-03.runtime.u32.right_31</code> | <code>NUM-03</code> | <code>u32</code> | <code>u32</code> | <code>&gt;&gt;</code> | <code>0</code> | <code>31</code> | <code>0x00000000</code> |
| <code>NUM-03.runtime.u32.left_32</code> | <code>NUM-03</code> | <code>u32</code> | <code>u32</code> | <code>&lt;&lt;</code> | <code>1</code> | <code>32</code> | <code>0x00000001</code> |
| <code>NUM-03.runtime.u32.right_32</code> | <code>NUM-03</code> | <code>u32</code> | <code>u32</code> | <code>&gt;&gt;</code> | <code>0</code> | <code>32</code> | <code>0x00000000</code> |
| <code>NUM-03.runtime.u32.left_33</code> | <code>NUM-03</code> | <code>u32</code> | <code>u32</code> | <code>&lt;&lt;</code> | <code>1</code> | <code>33</code> | <code>0x00000002</code> |
| <code>NUM-03.runtime.u32.right_33</code> | <code>NUM-03</code> | <code>u32</code> | <code>u32</code> | <code>&gt;&gt;</code> | <code>0</code> | <code>33</code> | <code>0x00000000</code> |
| <code>NUM-03.runtime.u32.left_4294967295</code> | <code>NUM-03</code> | <code>u32</code> | <code>u32</code> | <code>&lt;&lt;</code> | <code>1</code> | <code>4294967295</code> | <code>0x80000000</code> |
| <code>NUM-03.runtime.u32.right_4294967295</code> | <code>NUM-03</code> | <code>u32</code> | <code>u32</code> | <code>&gt;&gt;</code> | <code>0</code> | <code>4294967295</code> | <code>0x00000000</code> |
| <code>NUM-04.runtime.u32.div_zero</code> | <code>NUM-04</code> | <code>u32</code> | <code>u32</code> | <code>/</code> | <code>1</code> | <code>0</code> | <code>InvalidInstruction; 0xdeadbeef</code> |
| <code>NUM-04.runtime.u32.rem_zero</code> | <code>NUM-04</code> | <code>u32</code> | <code>u32</code> | <code>%</code> | <code>1</code> | <code>0</code> | <code>InvalidInstruction; 0xdeadbeef</code> |
| <code>NUM-05.runtime.i32.add</code> | <code>NUM-05</code> | <code>i32</code> | <code>i32</code> | <code>+</code> | <code>6</code> | <code>3</code> | <code>0x00000009</code> |
| <code>NUM-05.runtime.i32.sub</code> | <code>NUM-05</code> | <code>i32</code> | <code>i32</code> | <code>-</code> | <code>6</code> | <code>3</code> | <code>0x00000003</code> |
| <code>NUM-05.runtime.i32.mul</code> | <code>NUM-05</code> | <code>i32</code> | <code>i32</code> | <code>*</code> | <code>6</code> | <code>3</code> | <code>0x00000012</code> |
| <code>NUM-04.runtime.i32.div</code> | <code>NUM-04</code> | <code>i32</code> | <code>i32</code> | <code>/</code> | <code>6</code> | <code>3</code> | <code>0x00000002</code> |
| <code>NUM-04.runtime.i32.rem</code> | <code>NUM-04</code> | <code>i32</code> | <code>i32</code> | <code>%</code> | <code>6</code> | <code>3</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.i32.and</code> | <code>NUM-05</code> | <code>i32</code> | <code>i32</code> | <code>&amp;</code> | <code>6</code> | <code>3</code> | <code>0x00000002</code> |
| <code>NUM-05.runtime.i32.or</code> | <code>NUM-05</code> | <code>i32</code> | <code>i32</code> | <code>&#124;</code> | <code>6</code> | <code>3</code> | <code>0x00000007</code> |
| <code>NUM-05.runtime.i32.xor</code> | <code>NUM-05</code> | <code>i32</code> | <code>i32</code> | <code>^</code> | <code>6</code> | <code>3</code> | <code>0x00000005</code> |
| <code>NUM-05.runtime.i32.andnot</code> | <code>NUM-05</code> | <code>i32</code> | <code>i32</code> | <code>&amp;^</code> | <code>6</code> | <code>3</code> | <code>0x00000004</code> |
| <code>NUM-05.runtime.i32.eq</code> | <code>NUM-05</code> | <code>i32</code> | <code>i32</code> | <code>==</code> | <code>6</code> | <code>3</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.i32.ne</code> | <code>NUM-05</code> | <code>i32</code> | <code>i32</code> | <code>!=</code> | <code>6</code> | <code>3</code> | <code>0x00000001</code> |
| <code>NUM-05.runtime.i32.lt</code> | <code>NUM-05</code> | <code>i32</code> | <code>i32</code> | <code>&lt;</code> | <code>6</code> | <code>3</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.i32.le</code> | <code>NUM-05</code> | <code>i32</code> | <code>i32</code> | <code>&lt;=</code> | <code>6</code> | <code>3</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.i32.gt</code> | <code>NUM-05</code> | <code>i32</code> | <code>i32</code> | <code>&gt;</code> | <code>6</code> | <code>3</code> | <code>0x00000001</code> |
| <code>NUM-05.runtime.i32.ge</code> | <code>NUM-05</code> | <code>i32</code> | <code>i32</code> | <code>&gt;=</code> | <code>6</code> | <code>3</code> | <code>0x00000001</code> |
| <code>NUM-05.runtime.i32.wrap_add</code> | <code>NUM-05</code> | <code>i32</code> | <code>i32</code> | <code>+</code> | <code>2147483647</code> | <code>1</code> | <code>0x80000000</code> |
| <code>NUM-05.runtime.i32.wrap_sub</code> | <code>NUM-05</code> | <code>i32</code> | <code>i32</code> | <code>-</code> | <code>-2147483648</code> | <code>1</code> | <code>0x7fffffff</code> |
| <code>NUM-05.runtime.i32.wrap_mul</code> | <code>NUM-05</code> | <code>i32</code> | <code>i32</code> | <code>*</code> | <code>2147483647</code> | <code>2</code> | <code>0xfffffffe</code> |
| <code>NUM-05.runtime.i32.positive</code> | <code>NUM-05</code> | <code>i32</code> | <code>i32</code> | <code>unary+</code> | <code>2147483647</code> | <code>0</code> | <code>0x7fffffff</code> |
| <code>NUM-05.runtime.i32.negative</code> | <code>NUM-05</code> | <code>i32</code> | <code>i32</code> | <code>unary-</code> | <code>-2147483648</code> | <code>0</code> | <code>0x80000000</code> |
| <code>NUM-05.runtime.i32.not</code> | <code>NUM-05</code> | <code>i32</code> | <code>i32</code> | <code>unary^</code> | <code>0</code> | <code>0</code> | <code>0xffffffff</code> |
| <code>NUM-05.runtime.i32.boundary_eq</code> | <code>NUM-05</code> | <code>i32</code> | <code>i32</code> | <code>==</code> | <code>-2147483648</code> | <code>2147483647</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.i32.boundary_ne</code> | <code>NUM-05</code> | <code>i32</code> | <code>i32</code> | <code>!=</code> | <code>-2147483648</code> | <code>2147483647</code> | <code>0x00000001</code> |
| <code>NUM-05.runtime.i32.boundary_lt</code> | <code>NUM-05</code> | <code>i32</code> | <code>i32</code> | <code>&lt;</code> | <code>-2147483648</code> | <code>2147483647</code> | <code>0x00000001</code> |
| <code>NUM-05.runtime.i32.boundary_le</code> | <code>NUM-05</code> | <code>i32</code> | <code>i32</code> | <code>&lt;=</code> | <code>-2147483648</code> | <code>2147483647</code> | <code>0x00000001</code> |
| <code>NUM-05.runtime.i32.boundary_gt</code> | <code>NUM-05</code> | <code>i32</code> | <code>i32</code> | <code>&gt;</code> | <code>-2147483648</code> | <code>2147483647</code> | <code>0x00000000</code> |
| <code>NUM-05.runtime.i32.boundary_ge</code> | <code>NUM-05</code> | <code>i32</code> | <code>i32</code> | <code>&gt;=</code> | <code>-2147483648</code> | <code>2147483647</code> | <code>0x00000000</code> |
| <code>NUM-03.runtime.i32.left_0</code> | <code>NUM-03</code> | <code>i32</code> | <code>i32</code> | <code>&lt;&lt;</code> | <code>1</code> | <code>0</code> | <code>0x00000001</code> |
| <code>NUM-03.runtime.i32.right_0</code> | <code>NUM-03</code> | <code>i32</code> | <code>i32</code> | <code>&gt;&gt;</code> | <code>-2147483648</code> | <code>0</code> | <code>0x80000000</code> |
| <code>NUM-03.runtime.i32.left_31</code> | <code>NUM-03</code> | <code>i32</code> | <code>i32</code> | <code>&lt;&lt;</code> | <code>1</code> | <code>31</code> | <code>0x80000000</code> |
| <code>NUM-03.runtime.i32.right_31</code> | <code>NUM-03</code> | <code>i32</code> | <code>i32</code> | <code>&gt;&gt;</code> | <code>-2147483648</code> | <code>31</code> | <code>0xffffffff</code> |
| <code>NUM-03.runtime.i32.left_32</code> | <code>NUM-03</code> | <code>i32</code> | <code>i32</code> | <code>&lt;&lt;</code> | <code>1</code> | <code>32</code> | <code>0x00000001</code> |
| <code>NUM-03.runtime.i32.right_32</code> | <code>NUM-03</code> | <code>i32</code> | <code>i32</code> | <code>&gt;&gt;</code> | <code>-2147483648</code> | <code>32</code> | <code>0x80000000</code> |
| <code>NUM-03.runtime.i32.left_33</code> | <code>NUM-03</code> | <code>i32</code> | <code>i32</code> | <code>&lt;&lt;</code> | <code>1</code> | <code>33</code> | <code>0x00000002</code> |
| <code>NUM-03.runtime.i32.right_33</code> | <code>NUM-03</code> | <code>i32</code> | <code>i32</code> | <code>&gt;&gt;</code> | <code>-2147483648</code> | <code>33</code> | <code>0xc0000000</code> |
| <code>NUM-03.runtime.i32.left_4294967295</code> | <code>NUM-03</code> | <code>i32</code> | <code>i32</code> | <code>&lt;&lt;</code> | <code>1</code> | <code>4294967295</code> | <code>0x80000000</code> |
| <code>NUM-03.runtime.i32.right_4294967295</code> | <code>NUM-03</code> | <code>i32</code> | <code>i32</code> | <code>&gt;&gt;</code> | <code>-2147483648</code> | <code>4294967295</code> | <code>0xffffffff</code> |
| <code>NUM-04.runtime.i32.div_zero</code> | <code>NUM-04</code> | <code>i32</code> | <code>i32</code> | <code>/</code> | <code>1</code> | <code>0</code> | <code>InvalidInstruction; 0xdeadbeef</code> |
| <code>NUM-04.runtime.i32.rem_zero</code> | <code>NUM-04</code> | <code>i32</code> | <code>i32</code> | <code>%</code> | <code>1</code> | <code>0</code> | <code>InvalidInstruction; 0xdeadbeef</code> |
| <code>NUM-04.runtime.i32.div_min</code> | <code>NUM-04</code> | <code>i32</code> | <code>i32</code> | <code>/</code> | <code>-2147483648</code> | <code>-1</code> | <code>InvalidInstruction; 0xdeadbeef</code> |
| <code>NUM-04.runtime.i32.rem_min</code> | <code>NUM-04</code> | <code>i32</code> | <code>i32</code> | <code>%</code> | <code>-2147483648</code> | <code>-1</code> | <code>0x00000000</code> |
| <code>NUM-04.runtime.i32.div_pp</code> | <code>NUM-04</code> | <code>i32</code> | <code>i32</code> | <code>/</code> | <code>7</code> | <code>3</code> | <code>0x00000002</code> |
| <code>NUM-04.runtime.i32.rem_pp</code> | <code>NUM-04</code> | <code>i32</code> | <code>i32</code> | <code>%</code> | <code>7</code> | <code>3</code> | <code>0x00000001</code> |
| <code>NUM-04.runtime.i32.div_np</code> | <code>NUM-04</code> | <code>i32</code> | <code>i32</code> | <code>/</code> | <code>-7</code> | <code>3</code> | <code>0xfffffffe</code> |
| <code>NUM-04.runtime.i32.rem_np</code> | <code>NUM-04</code> | <code>i32</code> | <code>i32</code> | <code>%</code> | <code>-7</code> | <code>3</code> | <code>0xffffffff</code> |
| <code>NUM-04.runtime.i32.div_pn</code> | <code>NUM-04</code> | <code>i32</code> | <code>i32</code> | <code>/</code> | <code>7</code> | <code>-3</code> | <code>0xfffffffe</code> |
| <code>NUM-04.runtime.i32.rem_pn</code> | <code>NUM-04</code> | <code>i32</code> | <code>i32</code> | <code>%</code> | <code>7</code> | <code>-3</code> | <code>0x00000001</code> |
| <code>NUM-04.runtime.i32.div_nn</code> | <code>NUM-04</code> | <code>i32</code> | <code>i32</code> | <code>/</code> | <code>-7</code> | <code>-3</code> | <code>0x00000002</code> |
| <code>NUM-04.runtime.i32.rem_nn</code> | <code>NUM-04</code> | <code>i32</code> | <code>i32</code> | <code>%</code> | <code>-7</code> | <code>-3</code> | <code>0xffffffff</code> |
| <code>CAST-02.runtime.u8.u8.min</code> | <code>CAST-02</code> | <code>u8</code> | <code>u8</code> | <code>cast</code> | <code>0</code> | <code>0</code> | <code>0x00000000</code> |
| <code>CAST-02.runtime.u8.u8.max</code> | <code>CAST-02</code> | <code>u8</code> | <code>u8</code> | <code>cast</code> | <code>255</code> | <code>0</code> | <code>0x000000ff</code> |
| <code>CAST-02.runtime.u8.i8.min</code> | <code>CAST-02</code> | <code>i8</code> | <code>u8</code> | <code>cast</code> | <code>0</code> | <code>0</code> | <code>0x00000000</code> |
| <code>CAST-02.runtime.u8.i8.max</code> | <code>CAST-02</code> | <code>i8</code> | <code>u8</code> | <code>cast</code> | <code>255</code> | <code>0</code> | <code>0xffffffff</code> |
| <code>CAST-02.runtime.u8.u16.min</code> | <code>CAST-02</code> | <code>u16</code> | <code>u8</code> | <code>cast</code> | <code>0</code> | <code>0</code> | <code>0x00000000</code> |
| <code>CAST-02.runtime.u8.u16.max</code> | <code>CAST-02</code> | <code>u16</code> | <code>u8</code> | <code>cast</code> | <code>255</code> | <code>0</code> | <code>0x000000ff</code> |
| <code>CAST-02.runtime.u8.i16.min</code> | <code>CAST-02</code> | <code>i16</code> | <code>u8</code> | <code>cast</code> | <code>0</code> | <code>0</code> | <code>0x00000000</code> |
| <code>CAST-02.runtime.u8.i16.max</code> | <code>CAST-02</code> | <code>i16</code> | <code>u8</code> | <code>cast</code> | <code>255</code> | <code>0</code> | <code>0x000000ff</code> |
| <code>CAST-02.runtime.u8.u32.min</code> | <code>CAST-02</code> | <code>u32</code> | <code>u8</code> | <code>cast</code> | <code>0</code> | <code>0</code> | <code>0x00000000</code> |
| <code>CAST-02.runtime.u8.u32.max</code> | <code>CAST-02</code> | <code>u32</code> | <code>u8</code> | <code>cast</code> | <code>255</code> | <code>0</code> | <code>0x000000ff</code> |
| <code>CAST-02.runtime.u8.i32.min</code> | <code>CAST-02</code> | <code>i32</code> | <code>u8</code> | <code>cast</code> | <code>0</code> | <code>0</code> | <code>0x00000000</code> |
| <code>CAST-02.runtime.u8.i32.max</code> | <code>CAST-02</code> | <code>i32</code> | <code>u8</code> | <code>cast</code> | <code>255</code> | <code>0</code> | <code>0x000000ff</code> |
| <code>CAST-02.runtime.i8.u8.min</code> | <code>CAST-02</code> | <code>u8</code> | <code>i8</code> | <code>cast</code> | <code>-128</code> | <code>0</code> | <code>0x00000080</code> |
| <code>CAST-02.runtime.i8.u8.max</code> | <code>CAST-02</code> | <code>u8</code> | <code>i8</code> | <code>cast</code> | <code>127</code> | <code>0</code> | <code>0x0000007f</code> |
| <code>CAST-02.runtime.i8.i8.min</code> | <code>CAST-02</code> | <code>i8</code> | <code>i8</code> | <code>cast</code> | <code>-128</code> | <code>0</code> | <code>0xffffff80</code> |
| <code>CAST-02.runtime.i8.i8.max</code> | <code>CAST-02</code> | <code>i8</code> | <code>i8</code> | <code>cast</code> | <code>127</code> | <code>0</code> | <code>0x0000007f</code> |
| <code>CAST-02.runtime.i8.u16.min</code> | <code>CAST-02</code> | <code>u16</code> | <code>i8</code> | <code>cast</code> | <code>-128</code> | <code>0</code> | <code>0x0000ff80</code> |
| <code>CAST-02.runtime.i8.u16.max</code> | <code>CAST-02</code> | <code>u16</code> | <code>i8</code> | <code>cast</code> | <code>127</code> | <code>0</code> | <code>0x0000007f</code> |
| <code>CAST-02.runtime.i8.i16.min</code> | <code>CAST-02</code> | <code>i16</code> | <code>i8</code> | <code>cast</code> | <code>-128</code> | <code>0</code> | <code>0xffffff80</code> |
| <code>CAST-02.runtime.i8.i16.max</code> | <code>CAST-02</code> | <code>i16</code> | <code>i8</code> | <code>cast</code> | <code>127</code> | <code>0</code> | <code>0x0000007f</code> |
| <code>CAST-02.runtime.i8.u32.min</code> | <code>CAST-02</code> | <code>u32</code> | <code>i8</code> | <code>cast</code> | <code>-128</code> | <code>0</code> | <code>0xffffff80</code> |
| <code>CAST-02.runtime.i8.u32.max</code> | <code>CAST-02</code> | <code>u32</code> | <code>i8</code> | <code>cast</code> | <code>127</code> | <code>0</code> | <code>0x0000007f</code> |
| <code>CAST-02.runtime.i8.i32.min</code> | <code>CAST-02</code> | <code>i32</code> | <code>i8</code> | <code>cast</code> | <code>-128</code> | <code>0</code> | <code>0xffffff80</code> |
| <code>CAST-02.runtime.i8.i32.max</code> | <code>CAST-02</code> | <code>i32</code> | <code>i8</code> | <code>cast</code> | <code>127</code> | <code>0</code> | <code>0x0000007f</code> |
| <code>CAST-02.runtime.u16.u8.min</code> | <code>CAST-02</code> | <code>u8</code> | <code>u16</code> | <code>cast</code> | <code>0</code> | <code>0</code> | <code>0x00000000</code> |
| <code>CAST-02.runtime.u16.u8.max</code> | <code>CAST-02</code> | <code>u8</code> | <code>u16</code> | <code>cast</code> | <code>65535</code> | <code>0</code> | <code>0x000000ff</code> |
| <code>CAST-02.runtime.u16.i8.min</code> | <code>CAST-02</code> | <code>i8</code> | <code>u16</code> | <code>cast</code> | <code>0</code> | <code>0</code> | <code>0x00000000</code> |
| <code>CAST-02.runtime.u16.i8.max</code> | <code>CAST-02</code> | <code>i8</code> | <code>u16</code> | <code>cast</code> | <code>65535</code> | <code>0</code> | <code>0xffffffff</code> |
| <code>CAST-02.runtime.u16.u16.min</code> | <code>CAST-02</code> | <code>u16</code> | <code>u16</code> | <code>cast</code> | <code>0</code> | <code>0</code> | <code>0x00000000</code> |
| <code>CAST-02.runtime.u16.u16.max</code> | <code>CAST-02</code> | <code>u16</code> | <code>u16</code> | <code>cast</code> | <code>65535</code> | <code>0</code> | <code>0x0000ffff</code> |
| <code>CAST-02.runtime.u16.i16.min</code> | <code>CAST-02</code> | <code>i16</code> | <code>u16</code> | <code>cast</code> | <code>0</code> | <code>0</code> | <code>0x00000000</code> |
| <code>CAST-02.runtime.u16.i16.max</code> | <code>CAST-02</code> | <code>i16</code> | <code>u16</code> | <code>cast</code> | <code>65535</code> | <code>0</code> | <code>0xffffffff</code> |
| <code>CAST-02.runtime.u16.u32.min</code> | <code>CAST-02</code> | <code>u32</code> | <code>u16</code> | <code>cast</code> | <code>0</code> | <code>0</code> | <code>0x00000000</code> |
| <code>CAST-02.runtime.u16.u32.max</code> | <code>CAST-02</code> | <code>u32</code> | <code>u16</code> | <code>cast</code> | <code>65535</code> | <code>0</code> | <code>0x0000ffff</code> |
| <code>CAST-02.runtime.u16.i32.min</code> | <code>CAST-02</code> | <code>i32</code> | <code>u16</code> | <code>cast</code> | <code>0</code> | <code>0</code> | <code>0x00000000</code> |
| <code>CAST-02.runtime.u16.i32.max</code> | <code>CAST-02</code> | <code>i32</code> | <code>u16</code> | <code>cast</code> | <code>65535</code> | <code>0</code> | <code>0x0000ffff</code> |
| <code>CAST-02.runtime.i16.u8.min</code> | <code>CAST-02</code> | <code>u8</code> | <code>i16</code> | <code>cast</code> | <code>-32768</code> | <code>0</code> | <code>0x00000000</code> |
| <code>CAST-02.runtime.i16.u8.max</code> | <code>CAST-02</code> | <code>u8</code> | <code>i16</code> | <code>cast</code> | <code>32767</code> | <code>0</code> | <code>0x000000ff</code> |
| <code>CAST-02.runtime.i16.i8.min</code> | <code>CAST-02</code> | <code>i8</code> | <code>i16</code> | <code>cast</code> | <code>-32768</code> | <code>0</code> | <code>0x00000000</code> |
| <code>CAST-02.runtime.i16.i8.max</code> | <code>CAST-02</code> | <code>i8</code> | <code>i16</code> | <code>cast</code> | <code>32767</code> | <code>0</code> | <code>0xffffffff</code> |
| <code>CAST-02.runtime.i16.u16.min</code> | <code>CAST-02</code> | <code>u16</code> | <code>i16</code> | <code>cast</code> | <code>-32768</code> | <code>0</code> | <code>0x00008000</code> |
| <code>CAST-02.runtime.i16.u16.max</code> | <code>CAST-02</code> | <code>u16</code> | <code>i16</code> | <code>cast</code> | <code>32767</code> | <code>0</code> | <code>0x00007fff</code> |
| <code>CAST-02.runtime.i16.i16.min</code> | <code>CAST-02</code> | <code>i16</code> | <code>i16</code> | <code>cast</code> | <code>-32768</code> | <code>0</code> | <code>0xffff8000</code> |
| <code>CAST-02.runtime.i16.i16.max</code> | <code>CAST-02</code> | <code>i16</code> | <code>i16</code> | <code>cast</code> | <code>32767</code> | <code>0</code> | <code>0x00007fff</code> |
| <code>CAST-02.runtime.i16.u32.min</code> | <code>CAST-02</code> | <code>u32</code> | <code>i16</code> | <code>cast</code> | <code>-32768</code> | <code>0</code> | <code>0xffff8000</code> |
| <code>CAST-02.runtime.i16.u32.max</code> | <code>CAST-02</code> | <code>u32</code> | <code>i16</code> | <code>cast</code> | <code>32767</code> | <code>0</code> | <code>0x00007fff</code> |
| <code>CAST-02.runtime.i16.i32.min</code> | <code>CAST-02</code> | <code>i32</code> | <code>i16</code> | <code>cast</code> | <code>-32768</code> | <code>0</code> | <code>0xffff8000</code> |
| <code>CAST-02.runtime.i16.i32.max</code> | <code>CAST-02</code> | <code>i32</code> | <code>i16</code> | <code>cast</code> | <code>32767</code> | <code>0</code> | <code>0x00007fff</code> |
| <code>CAST-02.runtime.u32.u8.min</code> | <code>CAST-02</code> | <code>u8</code> | <code>u32</code> | <code>cast</code> | <code>0</code> | <code>0</code> | <code>0x00000000</code> |
| <code>CAST-02.runtime.u32.u8.max</code> | <code>CAST-02</code> | <code>u8</code> | <code>u32</code> | <code>cast</code> | <code>4294967295</code> | <code>0</code> | <code>0x000000ff</code> |
| <code>CAST-02.runtime.u32.i8.min</code> | <code>CAST-02</code> | <code>i8</code> | <code>u32</code> | <code>cast</code> | <code>0</code> | <code>0</code> | <code>0x00000000</code> |
| <code>CAST-02.runtime.u32.i8.max</code> | <code>CAST-02</code> | <code>i8</code> | <code>u32</code> | <code>cast</code> | <code>4294967295</code> | <code>0</code> | <code>0xffffffff</code> |
| <code>CAST-02.runtime.u32.u16.min</code> | <code>CAST-02</code> | <code>u16</code> | <code>u32</code> | <code>cast</code> | <code>0</code> | <code>0</code> | <code>0x00000000</code> |
| <code>CAST-02.runtime.u32.u16.max</code> | <code>CAST-02</code> | <code>u16</code> | <code>u32</code> | <code>cast</code> | <code>4294967295</code> | <code>0</code> | <code>0x0000ffff</code> |
| <code>CAST-02.runtime.u32.i16.min</code> | <code>CAST-02</code> | <code>i16</code> | <code>u32</code> | <code>cast</code> | <code>0</code> | <code>0</code> | <code>0x00000000</code> |
| <code>CAST-02.runtime.u32.i16.max</code> | <code>CAST-02</code> | <code>i16</code> | <code>u32</code> | <code>cast</code> | <code>4294967295</code> | <code>0</code> | <code>0xffffffff</code> |
| <code>CAST-02.runtime.u32.u32.min</code> | <code>CAST-02</code> | <code>u32</code> | <code>u32</code> | <code>cast</code> | <code>0</code> | <code>0</code> | <code>0x00000000</code> |
| <code>CAST-02.runtime.u32.u32.max</code> | <code>CAST-02</code> | <code>u32</code> | <code>u32</code> | <code>cast</code> | <code>4294967295</code> | <code>0</code> | <code>0xffffffff</code> |
| <code>CAST-02.runtime.u32.i32.min</code> | <code>CAST-02</code> | <code>i32</code> | <code>u32</code> | <code>cast</code> | <code>0</code> | <code>0</code> | <code>0x00000000</code> |
| <code>CAST-02.runtime.u32.i32.max</code> | <code>CAST-02</code> | <code>i32</code> | <code>u32</code> | <code>cast</code> | <code>4294967295</code> | <code>0</code> | <code>0xffffffff</code> |
| <code>CAST-02.runtime.i32.u8.min</code> | <code>CAST-02</code> | <code>u8</code> | <code>i32</code> | <code>cast</code> | <code>-2147483648</code> | <code>0</code> | <code>0x00000000</code> |
| <code>CAST-02.runtime.i32.u8.max</code> | <code>CAST-02</code> | <code>u8</code> | <code>i32</code> | <code>cast</code> | <code>2147483647</code> | <code>0</code> | <code>0x000000ff</code> |
| <code>CAST-02.runtime.i32.i8.min</code> | <code>CAST-02</code> | <code>i8</code> | <code>i32</code> | <code>cast</code> | <code>-2147483648</code> | <code>0</code> | <code>0x00000000</code> |
| <code>CAST-02.runtime.i32.i8.max</code> | <code>CAST-02</code> | <code>i8</code> | <code>i32</code> | <code>cast</code> | <code>2147483647</code> | <code>0</code> | <code>0xffffffff</code> |
| <code>CAST-02.runtime.i32.u16.min</code> | <code>CAST-02</code> | <code>u16</code> | <code>i32</code> | <code>cast</code> | <code>-2147483648</code> | <code>0</code> | <code>0x00000000</code> |
| <code>CAST-02.runtime.i32.u16.max</code> | <code>CAST-02</code> | <code>u16</code> | <code>i32</code> | <code>cast</code> | <code>2147483647</code> | <code>0</code> | <code>0x0000ffff</code> |
| <code>CAST-02.runtime.i32.i16.min</code> | <code>CAST-02</code> | <code>i16</code> | <code>i32</code> | <code>cast</code> | <code>-2147483648</code> | <code>0</code> | <code>0x00000000</code> |
| <code>CAST-02.runtime.i32.i16.max</code> | <code>CAST-02</code> | <code>i16</code> | <code>i32</code> | <code>cast</code> | <code>2147483647</code> | <code>0</code> | <code>0xffffffff</code> |
| <code>CAST-02.runtime.i32.u32.min</code> | <code>CAST-02</code> | <code>u32</code> | <code>i32</code> | <code>cast</code> | <code>-2147483648</code> | <code>0</code> | <code>0x80000000</code> |
| <code>CAST-02.runtime.i32.u32.max</code> | <code>CAST-02</code> | <code>u32</code> | <code>i32</code> | <code>cast</code> | <code>2147483647</code> | <code>0</code> | <code>0x7fffffff</code> |
| <code>CAST-02.runtime.i32.i32.min</code> | <code>CAST-02</code> | <code>i32</code> | <code>i32</code> | <code>cast</code> | <code>-2147483648</code> | <code>0</code> | <code>0x80000000</code> |
| <code>CAST-02.runtime.i32.i32.max</code> | <code>CAST-02</code> | <code>i32</code> | <code>i32</code> | <code>cast</code> | <code>2147483647</code> | <code>0</code> | <code>0x7fffffff</code> |
