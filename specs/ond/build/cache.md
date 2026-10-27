# ビルドキャッシュ契約
<!-- ond-spec-id: build/cache -->

## 成果物とキャッシュ
<!-- ond-spec-id: build/artifacts-and-cache -->

PackageArtifactとmanifestのfieldは[成果物schema](../artifacts.md#packageartifactとcompilationmanifest)に従う。

package build_id、content_hash、image build_idは以下の公開識別子規則に従う。\
再利用の可否はsource、設定、依存、具体化した意味型・定数値と公開interfaceを反映する。

### Canonical JSON
<!-- ond-spec-id: build/canonical-json -->

公開JSONのwriterは、[wire値型](../artifacts.md#json-wire形式)の値を次の一意なbyte列へ変換する。\
UTF-8を使い、BOM、値間のwhitespace、末尾LFを付けない。\
objectはkeyをUnicode scalar値の辞書順に並べ、arrayは各契約が定める順序を保つ。\
整数はwire形式の最短10進表記、boolとnullは小文字で書く。

stringでは`"`とbackslashをそれぞれJSONのquote escapeとbackslash escapeで書き、BS、TAB、LF、FF、CRはそれぞれ`\b`、`\t`、`\n`、`\f`、`\r`とする。\
それ以外のU+0000〜U+001Fは小文字hexの`\u00xx`、`/`とその他のUnicode scalarは元のUTF-8で書く。\
Unicode正規化は行わない。

### 公開識別子
<!-- ond-spec-id: build/public-identifiers -->

SHA-256の出力stringは、digestのbyte順に各byteを小文字hex2桁へ変換した64桁とする。\
`compiler_id`は実行compiler executableの全file bytes、`content_hash`は保存されたPackageArtifact fileの全bytesのSHA-256である。\
content_hashの照合時は入力file bytesをそのままhashし、再encodeしたJSONのhashへ置き換えない。

image build_idは、出力する実行可能バイナリの全bytesのSHA-256とする。\
externalは標準実行バイナリ全体、selfは前置起動処理を含む出力全体がhash入力である。\
debug sidecarのbuild_idにはこの識別子を使い、sidecar bytesをhash入力へ加えない。\
sidecar利用者は対象バイナリの全bytesから識別子を求め、不一致のsidecar全体を使用拒否する。

package build_idは次のPackageBuildInputをcanonical JSON化したbytesのSHA-256とする。\
以下のobjectも閉schemaとし、PackageBuildInput.rolesだけを後述の条件付きfieldとする。\
他の全fieldは必須であり、mapのentry集合は対応する規則に従う。

| 型 | Field |
| --- | --- |
| PackageBuildInput | `contract: Contract`, `compiler_id: string`, `manifest_hash: string`, `package: string`, `sources: SourceHash[]`, `dependencies: map<string,string>`, `generation: Generation`, 条件付き`roles: RoleBuildInput` |
| RoleBuildInput | `names: string[]`, `selected: string or null` |
| SourceHash | `path: string`, `content_hash: string` |
| Generation | `instances: Instance[]`, `types: TypeExpr[]`, `nominals: Nominal[]`, `constants: OwnedConstant[]`, `interface: NormalizedInterface` |

manifest_hashは全packageに共通して今回compileするproject rootのond.tomlの全file bytesのSHA-256、SourceHash.content_hashはsource fileの全bytesのSHA-256とする。\
BOMと改行を含む入力bytesを保持し、pathはpackageを構成するsource directory（依存packageもそのpackage directory）からの相対pathを`/`で区切る。\
sourcesはpathのUnicode scalar辞書順、dependenciesは直接import先の論理package pathからそのbuild_idへのmapとする。

rolesは当該packageの所属projectが[roles設定](./roles.md#manifest設定)を持つ場合に必須とし、roles tableを省略したprojectではfieldを省略する。\
roles field全体のnullは受理しない。\
namesは当該projectの宣言済み追加role集合をUnicode scalar辞書順に並べ、selectedは[compile時の選択](./roles.md#compile時の選択)がrole未指定の構成ならnull、名前付きroleならその名前のstringとする。\
依存packageのnamesも自身のprojectの設定値とし、selectedは[依存edgeの選択](./roles.md#依存edgeのrole選択)を同じ表現へ変換する。\
roles以外の既存入力とcanonical JSON規則を維持し、roles table省略projectの入力へ追加fieldを挿入しない。\
異なる選択roleは選択後の生成集合が同じ場合も異なるPackageBuildInputとなる。\
roles fieldはhash計算用であり、PackageArtifact、Object、CompilationManifest、Contractのfield集合と版識別は[成果物schema](../artifacts.md#版識別と互換条件)に従う。

### 意味型・具体化・定数の正規化
<!-- ond-spec-id: build/normalized-generation -->

本節のsource、宣言、式、importの母集団は、各所属projectで[role選択](../language/v1/roles.md#対象集合と選択)を行った後の集合とする。\
全nongeneric宣言、全source式とruntime conditionの両branchという記述も、この母集団内へ適用する。\
source hashは選択前の原文全bytesを対象とし、非選択部分の編集もsource内容の変更として扱う。

TypeExprは次の外部tag表現とし、構成要素の順序を型同一性の規則に対応づける。\
primitiveは`"Bool"`、`"U8"`、`"I8"`、`"U16"`、`"I16"`、`"U32"`、`"I32"`、`"F32"`とする。

| TypeExpr | 意味 |
| --- | --- |
| `{"Pointer":TypeExpr}` | 参照先型 |
| `{"Array":{"length":u32,"element":TypeExpr}}` | 長さと要素型 |
| `{"Function":NormalizedSignature}` | source-visible signature |
| `{"Struct":NormalizedField[]}` | 宣言順field列 |
| `{"Defined":NominalId}` | sourceで宣言したすべての名前付き型のidentity参照 |

| 型 | Field |
| --- | --- |
| NormalizedSignature | `parameters: TypeExpr[]`, `returns: TypeExpr[]` |
| NormalizedField | `name: string`, `ty: TypeExpr`, `private_owner: string or null` |
| NominalId | `package: string`, `name: string`, `arguments: TypeExpr[]` |
| Nominal | `identity: NominalId`, `underlying: TypeExpr`, `methods: NormalizedMethod[]` |
| NormalizedMethod | `name: string`, `signature: NormalizedSignature`, `pointer_receiver: bool`, `identity: InstanceId` |
| InstanceId | `declaration: DeclarationId`, `receiver_arguments: TypeExpr[]`, `type_arguments: TypeExpr[]` |
| Instance | `identity: InstanceId`, `signature: NormalizedSignature` |
| OwnedConstant | `owner: InstanceId`, `value: NormalizedValue` |
| NormalizedValue | `ty: TypeExpr`, `kind: NormalizedValueKind` |
| NormalizedInterface | `package: string`, `exports: map<string,NormalizedExport>` |

DeclarationIdは次の単一keyの外部tag objectである。\
Pは論理package path、N/M/receiverはsource宣言名とする。

| variant | payload field |
| --- | --- |
| Function | `package: string`, `name: string` |
| Method | `package: string`, `receiver: string`, `name: string`, `pointer_receiver: bool` |
| Operator | `package: string`, `receiver: string`, `token: string`, `operand_count: u32`, `pointer_receiver: bool` |
| Global、Constant | `package: string`, `name: string` |

operator tokenとreceiverを含むoperand_countは[operator宣言](../language/v1/expressions.md#user-defined-operator)に従う。\
NominalId.argumentsは型宣言の型parameter順とし、nongeneric型では空arrayとする。\
Functionのreceiver_argumentsは空、type_argumentsはtop-level generic関数の型適用で明示した型argumentを宣言順に並べた列とする。\
Method/Operatorのreceiver_argumentsはreceiver基底型の具体的な型argumentを宣言順に並べ、type_argumentsは空arrayとする。\
Global/Constantとnongeneric Functionは両argument列を空arrayとする。\
signature.parametersはreceiverを除くsource-visible列である。

Generation.instancesは今回compileに含まれる全source packageの、静的検査対象となる全nongeneric宣言内のsource使用を根とする。\
generic function/method/operatorの使用から[型parameterの具体化経路の成立条件](../language/v1/types.md#型parameter)に従って本体内の使用へ推移閉包を取り、宣言owner packageごとに各InstanceIdを1個収録する。\
Generation.typesの母集団は当該packageの全nongeneric宣言とinstancesの全具体化とし、その宣言型、function signature自体のFunction型、parameter/result、receiver、field、local宣言、各source式に確定した型を集合として収録する。\
この集合をPointerの参照先、Arrayの要素、Functionのparameter/result、Structのfield、Definedの型argumentとunderlyingの構成型へ再帰的に閉じる。\
言語の明示変換とreceiver調整後の型も含める。\
型構文内の配列長式は確定したArray.lengthにだけ反映し、その式のoperand/演算結果型は独立したtypes要素へ加えない。\
NominalIdを参照する位置ではunderlyingを展開せず、Generation.nominalsへsignature、本体型、公開interfaceから到達するnominalの閉包を各identityにつき1個収録する。\
すべての名前付き型はDefined参照で表し、underlyingはNominal表だけで展開する。\
nongeneric宣言型それ自体とそこで使う構成型もtypes/nominalsへ収録し、type宣言内の配列長等は型構造に含めてconstantsへ重複収録しない。\
これによりpointer経由の再帰も有限な参照で表す。\
nominalのmethodsは、その型の公開nongeneric methodと公開generic methodの収録済み具体化をname順、同名ならidentityのcanonical JSON bytes辞書順に並べ、物理symbol名をNormalizedMethod.identityへ置換する。

Generation.constantsは全nongeneric function/method/operator/global/const宣言とinstancesの具体化を母集団とし、次のsource値抽出で得る最大の定数部分式と静的aggregate/文字列実体を、ownerと型付き確定値の組として収録する。\
型構文内の配列長式を除くsource値式の親子関係を用い、他の値式を親に持たない各式を根とする。\
callのcallee/argumentはcall式の子、各operandは演算式の子として再帰し、未使用nongeneric宣言の本体とruntime conditionの両branchも含める。\
ここで定数部分式とは、元の型文脈を保ってconst initializerへ用いたときに言語のconst適格性を満たすsource式である。\
式全体が定数ならその型付き値を1個収録し、定数でない式はsourceのoperandへ再帰して同じ処理を行う。\
したがって`x+1`の`1`は収録し、`(1+2)*3`は結果`9`だけを収録する。\
短絡の値評価経路は[短絡規則](../language/v1/expressions.md#式と評価順)に従い、定数falseの&&右辺と定数trueの||右辺の値抽出を行う経路は左辺で終了する。\
同じ値評価経路にあるnestedの定数aggregateと文字列literalは静的実体として追加し、途中のscalar定数部分式は最大式へまとめる。\
静的実体もこの経路で到達したsourceだけから収録し、hashのための追加の値評価を行わない。\
型専用の配列長は型構造へ含め、型抽出は名前・型検査の対象全体に適用する。\
package-level global/constのownerはGlobal/Constant DeclarationIdを持ち両argument列が空のInstanceIdとし、instances列への所属を要求しない。\
function/method/operator本体の値と静的実体はそのInstanceIdをownerとする。

NormalizedInterface.exportsのbinding集合は[公開interface](../artifacts.md#公開interface)と同じ非generic公開宣言集合とする。\
NormalizedExportは同節のExportと同じ外部tagを使い、TypeRefをTypeExpr、ValueをNormalizedValueへ置換する。\
NormalizedValueKindはwire ValueKindと同じunit/scalar variantを持ち、Compositeだけ`{"Composite":[[u32,NormalizedValue],...]}`として再帰する。\
Compositeの対はindex昇順、zero valueの要素は省略し、全要素がzeroならkindを`"Zero"`に統一する。\
整数0、false、typed nil、f32の+0も指定型のzero表現ならZeroへ統一し、f32の-0はFloat32 payload `2147483648`で保持する。\
scalar payloadと型の整合は[型付きwire Value](../artifacts.md#公開interface)と同じ成立条件に従う。

Generationの各集合列はelementのcanonical JSON bytesのbyte辞書順に並べ、同一elementを重複収録しない。\
nominalsはidentityのcanonical bytes順とする。\
signature、struct fields、配列要素、型argumentの列順はそれぞれの意味順を保つ。\
内部型番号、source位置、要求回数、物理的なstorage共有、registerやbranch/slot offsetの定数は正規化入力へ加えない。

### 再利用と公開
<!-- ond-spec-id: build/cache-publication -->

全artifactの参照先、checksum、package集合、依存関係を検証できた場合だけfrontendを省略できる。\
sourceが変われば具体化集合、型構造、確定値を再確認し、package build_idが変化したObjectを生成する。\
壊れた索引・checksum不一致・不適合なcacheはmissとして再生成し、明示link入力の破損または不一致はlink errorとなる。\
compile失敗時は以前の成功成果物を保持し、公開するfileは同じdirectory内の一時fileの書込み完了後renameする。

公開するpackage成果物のfile名は`<build_id>-<content_hash>.ondpkg`とする。\
private cache keyのbyte表現・hash方式・保存形式・file名・保存範囲・回収時点は実装裁量とする。\
再利用の観測結果は以下の検証条件を満たす。

## 検証
<!-- ond-spec-id: build/verification -->

再利用の前提はsource内容、manifest、compiler_id、contract、import graph、依存build_idとartifact checksumの一致である。\
source読込、構文解析、依存graph収集はcompileごとに行う。\
[登録依存graphの成立条件](./roles.md#登録依存graphの成立条件)はcacheの一致によらず検証する。\
全package artifactの一致を検証できた場合は型付きfrontendとObject生成を省略できる。\
一つでも不一致ならproject全体に型付きfrontendを実行し、Object生成は再計算したpackage build_idの変化に応じて行う。\
`frontend_ran`は型付きfrontendを実行したことを示す。

検査はcold/warm、diamond依存、実装だけの変更、generic具体化の追加、sourceの追加・削除・rename、manifest/compiler変更、仮想sourceとの衝突を入力として再利用範囲を照合する。\
壊れたcacheからの再生成、明示link入力の破損error、fresh linkとのimage一致、診断位置、既存出力の保持も外部の観測結果である。

## process内意味解析キャッシュ
<!-- ond-spec-id: build/process-semantic-cache -->

同一process内の解析結果再利用は、以下の再利用・失効・保持の規則に従う。

### 意味解析cache
<!-- ond-spec-id: build/semantic-cache -->

意味解析結果を再利用するキャッシュは、packageを単位として扱う。\
ここでの再利用は同一processまたは解析session内の結果に適用する。\
Object生成の永続キャッシュは[パッケージ成果物・独立コンパイラ契約](./packages.md)に従う。

### 参照環境
<!-- ond-spec-id: build/semantic-cache-visibility -->

意味解析は、自packageとimportした依存packageのsourceを参照環境とする。\
名前の公開範囲と同一packageのprivate declarationへのアクセスは[名前規則](../language/v1/names-and-scopes.md#scopeとshadowing)に従う。

各参照環境には[role可視性](../language/v1/roles.md#名前解決とscope)を併せて適用し、依存境界は[project境界規則](./roles.md#project境界)に従う。

function、method、operatorの呼出しとgeneric宣言の特殊化では、定義元の参照環境を維持する。\
型、constant、globalの解決にも同じ規則を使う。

### 同一性と失効
<!-- ond-spec-id: build/semantic-cache-invalidation -->

再利用の判定にはsourceの内容とidentity、および参照するsourceの依存関係を用いる。\
同じ内容と参照環境を再利用条件とする。\
依存関係には型、constant、generic定義と同一packageのprivate declarationを含め、それらすべての内容と参照環境の一致を再利用条件とする。

参照環境のfinal・role設定、依存edgeの明示roleと解決した選択role、source位置のrole文脈の変更は[role cache入力](./roles.md#cache入力)に従って失効へ反映する。\
永続cache、frontend省略を判断する索引、LSPの名前索引と失敗診断にも同じ一致条件を適用する。

sourceが変わった場合、そのpackageの解析結果、および変更されたsourceを参照する解析結果を失効させる。\
importの追加・削除、fileの追加・削除・renameでも、変更後の参照環境に基づいて再利用を判定する。

source列挙順や解析snapshotが変化した後も、再利用した診断のprimary labelと関連labelを現在のsourceへ対応づける。\
失敗した解析結果も同じ失効規則を適用し、修正後は現在のsourceに基づく結果と診断を返す。

### 保持期間
<!-- ond-spec-id: build/semantic-cache-lifetime -->

意味解析キャッシュの保持期間は、それを所有するprocessまたは解析sessionの範囲内とする。\
キャッシュを破棄した場合も、再解析によって同じ意味解析結果を得られなければならない。\
LSPでは連続する編集の検証間でpackageごとの解析結果を再利用する。\
再利用と再解析は同じ可視性、診断、生成物の意味を与える。
