# Ond成果物と公開interface
<!-- ond-spec-id: artifacts/v1 -->

## JSON wire形式
<!-- ond-spec-id: artifacts/json-wire-format -->

本書と[debug](./debug-metadata/v1.md)、[test sidecar](./build/test-build.md)の公開JSONは、BOMのないUTF-8で表す。\
読出し時にはJSONの値間の空白を許可し、文字列はUnicode scalar列として解釈する。\
すべてのobjectは表で定義したfield集合だけを持つ閉schemaとし、未知fieldと重複keyはerrorとなる。\
keyの同一性はescapeをdecodeしたstringで比較し、Unicode正規化は行わない。\
`map<string,T>`は任意の相異なるstring keyからTへのobjectであり、mapの値にもTのschemaを適用する。

各fieldは必須であり、型に`null`を含むfieldは明示的なnullまたは指定型の値を持つ。\
省略できるfieldはPackageArtifactとCompilationManifestの`tests`だけであり、`tests`の明示nullはerrorとなる。\
配列の長さは指定がある場合その長さに一致し、tuple `(A,B)`は長さ2のJSON array `[A,B]`で表す。

| 値型 | JSON表現・成立条件 |
| --- | --- |
| string | Unicode scalar列のJSON string。不正なUTF-8と孤立surrogateはerror |
| bool | `true`または`false` |
| u32 | `0`または非zero数字で始まる10進数字列、値範囲0〜4294967295 |
| u64 | u32と同じ非負整数表記、値範囲0〜18446744073709551615 |
| i32 | `0`または正の10進数字列、または`-`に正の10進数字列を続けた表記、値範囲-2147483648〜2147483647 |
| bytes | 下記の標準Base64 string |
| T[] | TのJSON array。順序は各契約の列順に従う |

整数fieldのfraction、exponent、先頭zero、`-0`はerrorとなる。\
byte列はalphabet `A-Z a-z 0-9 + /`を使う標準Base64とし、末尾の不足6bit群をzeroで埋め、必要な`=`paddingを付ける。\
空byte列は空string、非空列の文字数は4の倍数で、最終quartet以外のpadding、whitespace、URL-safe alphabet、非zero pad bitsはerrorとなる。

unit enum variantはvariant名のJSON string、payload variantはvariant名を唯一のkeyとするobjectで表す。\
variant名は大文字・小文字を含め以下の綴りと完全一致し、表の集合に属することを成立条件とする。

| enum | 表現 |
| --- | --- |
| SectionKind | `"Text"`, `"Rodata"`, `"Data"`, `"Bss"` |
| SymbolBinding | `"Local"`, `"Exported"`, `"Imported"` |
| SymbolKind | `"Function"`, `"Object"`, `"Section"`, `"Runtime"` |
| RelocationKind | `"Abs32"`, `"Call16"`, `"Jump16"`, `"CallSlot"`, `"JumpSlot"` |
| Kind unit | `"Bool"`, `"U8"`, `"I8"`, `"U16"`, `"I16"`, `"U32"`, `"I32"`, `"F32"` |
| Kind payload | `{"Pointer": TypeRef}`, `{"Array":{"length":u32,"element":TypeRef}}`, `{"Function":Signature}`, `{"Struct":Field[]}`, `{"Defined":TypeRef}` |
| Export | `{"Type":TypeRef}`, `{"Constant":Value}`, `{"Global":TypeRef}`, `{"Function":Signature}` |
| ValueKind unit | `"Zero"`, `"Nil"` |
| ValueKind payload | `{"Integer":u64}`, `{"Float32":u32}`, `{"Bool":bool}`, `{"Composite":[[u32,Value],...]}` |

compiler_id、build_id、content_hashの保存値は[SHA-256識別子](./build/cache.md#公開識別子)の小文字hex64桁とする。

schema違反は公開入力のerrorとなり、compile/link CLIでは[失敗契約](./build/commands.md#入出力と失敗時)を適用する。\
公開JSONの生成byte列とhashは[canonical JSONと識別子](./build/cache.md#canonical-json)に従う。

## 公開interface
<!-- ond-spec-id: build/public-metadata-revision-3 -->

成果物の外層contractはrevision 6とし、[版識別と互換条件](#版識別と互換条件)を適用する。

`interface`はpackage、types（型定義の配列）、exports（公開名→宣言）からなる。\
target-neutralな型・公開宣言情報であり、型参照はこの成果物localのtypes配列indexで表す。

exportsは非genericのsource公開bindingを名前ごとに収録する。\
generic型・関数・method・operatorの使用はsource宣言から具体化し、具体化したcodeを収録するObjectは宣言を所有するpackageのObjectとする。\
packageを跨ぐ具体化codeへの参照に必要なObject symbolはExportedとして公開でき、interface.exportsのsource bindingとは別の集合を形成する。

| 型 | Fieldまたはvariant | 意味 |
| --- | --- | --- |
| Interface | `package: string`, `types: Type[]`, `exports: map<string,Export>` | 論理package path、型配列、ASCII大文字で始まる公開名からExportへのmap |
| Type | `name: string or null`, `kind: Kind`, `methods: ConcreteMethod[]` | 下記のnominal名または無名型のnull、型構造、公開concrete method |
| Kind | `Bool`, `U8`, `I8`, `U16`, `I16`, `U32`, `I32`, `F32` | primitive型 |
| Kind | `Pointer(TypeRef)`, `Array {length: u32, element: TypeRef}` | 参照先型、配列長と要素型 |
| Kind | `Function(Signature)`, `Struct(Field[])`, `Defined(TypeRef)` | 関数型、宣言順field列、underlying型 |
| Signature | `parameters: TypeRef[]`, `returns: TypeRef[]` | source-visibleな引数型列と戻り値型列 |
| Field | `name: string`, `ty: TypeRef`, `private_owner: string or null` | field名と型、非公開fieldの所有package path |
| ConcreteMethod | `name: string`, `signature: Signature`, `pointer_receiver: bool`, `symbol: string` | 公開method名、receiverを除いたsignature、receiver種別とlink symbol |
| Export | `Type(TypeRef)`, `Constant(Value)`, `Global(TypeRef)`, `Function(Signature)` | 公開宣言の種別と情報 |
| Value | `ty: TypeRef`, `kind: ValueKind` | 確定型と評価済み値 |
| ValueKind | `Zero`, `Integer(u64)`, `Float32(u32)`, `Bool(bool)`, `Nil`, `Composite((u32, Value)[])` | zero、整数bits、binary32 bits、bool、型付きnil、疎なindex/field値列 |

TypeRefはこのinterfaceのtypes配列indexをu32で表す。\
非genericの公開宣言から到達する型を再帰的に保存し、非公開のnamed typeと具体化したgeneric型も公開戻り値などから到達すればtypesへ含める。\
pointerによる循環型参照も成立する。

型graphには[直接再帰とpointer経由再帰の成立条件](./language/v1/types.md#型)を適用する。\
Definedだけの循環で構造型へ解決できない入力はerrorとなり、Pointerへ到達した値の構造型はPointerとして、その参照先の循環を扱う。

名前付き型のidentityは[NominalId](./build/cache.md#意味型具体化定数の正規化)のpackage、source宣言名、宣言順の型argument列で定める。\
対応するTypeはkind=Definedであり、nameは非generic型では`package.name`、generic型ではこれに型argumentのTypeExpr列の[canonical JSON](./build/cache.md#canonical-json)を続けたstringとする。\
例えば`p.Box[u8]`のnameは`p.Box["U8"]`であり、入れ子の名前付き型argumentはTypeExprのDefined/NominalIdで表す。\
無名型のnameはnullとし、型番号によらずこのidentityを[NormalizedInterface](./build/cache.md#意味型具体化定数の正規化)でも維持する。

Value.tyがDefinedならunderlyingを辿って値の構造型を解決し、次のValueKind成立条件を適用する。\
子Valueのtyの一致は[型同一性](./language/v1/types.md#型同一性)によって判定する。

| ValueKind | tyとpayloadの成立条件 |
| --- | --- |
| Zero | 全型のzero value |
| Integer | 整数型。unsignedは型の値範囲内の非負値。signedは型幅のtwo's-complement値を64bitへ符号拡張したu64 bitsと完全一致 |
| Float32 | f32。payloadはbinary32のu32 bits |
| Bool | bool。payloadはbool |
| Nil | PointerまたはFunction型のnil |
| Composite | arrayまたはstruct。indexは配列では0以上length未満、structでは宣言順の0始まりfield index。各子tyは対応する要素/field型と同一で、indexは一意 |

Compositeの省略要素は指定型のzero valueとする。\
ValueKind単体のwire構造検査と、所属Interfaceのtypesを使うこの型・値の整合検査はそれぞれ適用する。\
読出しは明示したscalar zeroとzeroのComposite要素も受理する。\
writerは[正規化値](./build/cache.md#意味型具体化定数の正規化)と同じく、型のzero表現をZeroへ、Compositeをindex昇順とzero要素省略へ統一し、f32の-0 bitsを保持する。

読出時には型参照の範囲、公開binding名、nominal名の重複、aggregate indexの重複、\
成果物とinterfaceのpackage一致を検証する。\
この読出検査は成果物内の参照整合性を確認する。\
sourceの型と名前検査は[compile単位](./build/packages.md#コンパイル単位と成果物)に従う。

成果物の受理は[版識別表](#版識別と互換条件)のContract tupleとの一致を条件とする。\
cache再利用は[cache契約](./build/cache.md)に従う。

## 版識別と互換条件
<!-- ond-spec-id: artifacts/version-compatibility -->

| 識別軸 | 適用対象 | 値 | 依存と互換条件 |
| --- | --- | --- | --- |
| language | sourceの構文・型・意味 | Ond v1 | [language v1](./language/v1/README.md)の入力と規則に適用 |
| target profile | machine/layout/runtime | `kagura-v1` | languageのtarget委譲を具体化 |
| ABI | call/register/引数・戻り値 | `ond-kagura-v1` | `kagura-v1`に適用 |
| artifact Contract | PackageArtifactとCompilationManifestの外層 | revision `6`, target `kagura-v1`, abi `ond-kagura-v1` | tuple全要素が表の値に一致すると受理。Contractの未知fieldはerror |
| compiler_id | artifact生成元とcache再利用の識別 | 実行executable全体のSHA-256 | CLI linkはmanifest、全artifact、実行linkerの同一値を要求 |

language版、target版、ABI版は対応する規範文書の適用対象を識別する。\
成果物に記録する版識別は上表のContract tupleである。\
`compiler_id`は同じ生成元を識別する値であり、一致によって生成元と再利用の条件を判定する。\
Objectも本書のJSON wire形式と後掲のfield集合で表す。\
実行可能バイナリは[標準実行バイナリ](#標準実行バイナリ)の版識別を用いる。

## PackageArtifactとCompilationManifest
<!-- ond-spec-id: artifacts/package-and-manifest -->

`.ondpkg`と`.ondbuild`はUTF-8 JSONであり、byte列であるsection dataはBase64文字列で表す。\
fieldの名前と意味を次の表で定義する。

| 型 | Field | 意味 |
| --- | --- | --- |
| Contract | `revision: u32`, `target: string`, `abi: string` | 前節のtuple |
| PackageArtifact | `contract: Contract`, `compiler_id: string`, `package: string`, `build_id: string` | 外層契約、生成元、論理package path、package内容識別 |
| PackageArtifact | `dependencies: map<string,string>` | 直接依存package pathからbuild_idへの対応 |
| PackageArtifact | `imports: string[]`, `initializer: string or null` | source順のimport path、global初期化関数symbol |
| PackageArtifact | `interface: Interface`, `object: Object`, `debug: DebugObject` | 公開interface、Object、link前debug情報 |
| PackageArtifact | `tests: TestMetadata` | test compile時の[tests object](./build/test-build.md#buildと起動入口)。通常compileはfieldを省略 |
| CompilationManifest | `contract: Contract`, `compiler_id: string` | 外層契約と生成元 |
| CompilationManifest | `packages: ArtifactRef[]`, `program: ProgramMetadata` | 決定的なObject入力順のartifact列、実行計画 |
| CompilationManifest | `tests: TestMetadata` | test compile時のTestMetadata。case 0件も`cases: []`を記録 |
| ArtifactRef | `package: string`, `build_id: string`, `path: string`, `content_hash: string` | package path、内容識別、artifact file path、file bytesのSHA-256 |
| ProgramMetadata | `entry_symbol: string`, `initializers: string[]` | entry symbol、言語初期化順に並ぶinitializer symbol |

Object入力順とinitializer実行順は別の列として保持する。\
testsのcase列と`.ondtest`は[test build契約](./build/test-build.md)を定義元とする。\
CompilationManifest、Contract、TestMetadataとcaseは上表と[test schema](./build/test-build.md#buildと起動入口)のfield集合で検査し、未知fieldはerrorとなる。\
明示`tests: null`はerrorとなり、field省略とcase 0件のtest buildを区別する。\
PackageArtifactとinterfaceのpackageは一致し、型参照、public名、nominal名、method名/symbol、aggregate indexの整合性を読出時に検査する。\
build_idとcache keyの生成は[cache契約](./build/cache.md)、debug schemaは[debug metadata](./debug-metadata/v1.md)に従う。

## Manifest読出しと参照整合
<!-- ond-spec-id: artifacts/manifest-validation -->

fileから読んだmanifestの相対ArtifactRef.pathは、そのmanifest fileの親directoryを基準に解決する。\
stdinから読んだmanifestではlink呼出時のcurrent directoryを基準とし、絶対pathはそのpathを使う。

linkは以下の入力整合を検査し、違反はlink errorとなる。

| 対象 | 成立条件 |
| --- | --- |
| manifestと各artifact | 同じContract、同じcompiler_id。CLIでは実行linkerのcompiler_idにも一致 |
| ArtifactRef | package/build_idがfile内artifactに一致し、content_hashが元file bytesのSHA-256に一致 |
| package集合 | packageごとにartifactが一意 |
| interface/debug | packageが所属PackageArtifact.packageに一致。Object.nameは診断用 |
| import/dependency | importsの集合がdependenciesのkey集合に一致、各依存artifactが存在しそのbuild_idが依存値に一致 |
| import graph | 各packageのimports列をsource定義順として、[packageと循環の規則](./language/v1/names-and-scopes.md#packageとimport)を満たす |
| initializer | nullまたは対応packageのExported Function symbolで、列挙されたinitializerが全て解決できる |
| mode | manifestと全artifactのtestsの有無が一致 |
| test case | package/name/symbolの組が一意で、manifestのcase集合が全artifactのcase集合の和に一致 |

imports列の重複は[同じimportの言語規則](./language/v1/names-and-scopes.md#scopeとshadowing)に従い、集合の一致検査で列順を変更しない。\
packages列のObject入力順とProgramMetadata.initializersの順も保持する。

ProgramMetadataは通常startup計画を表す。\
通常buildのentry_symbolはmain.mainを指し、そのExported Functionを持つpackageをrootとして、[言語の初期化順](./language/v1/execution.md#初期化とエントリ)から導く全initializer列とprogram.initializersが一致することを要求する。

test起動は各case.packageから導くwrapper計画とする。\
compileは[test sourceの成立条件](./build/test-build.md#sourceと文脈)を満たす宣言からtest caseを収録する。\
各artifactのcase.packageは当該artifact.packageに一致する。\
linkはcase.symbolが[名前規則](#link-symbolの名前と所属)と対応する所属packageのExported Function定義に一致することを検査する。\
wrapperは所属packageと依存先のinitializer列、対象functionの順を[test起動契約](./build/test-build.md#buildと起動入口)で実行する。\
test modeのprogram.entry_symbolに通常mainの解決を要求せず、library testは同章のlibrary成立条件に従う。

## Object
<!-- ond-spec-id: artifacts/object -->

| 型 | Field | 値と意味 |
| --- | --- | --- |
| Object | `name: string`, `sections: Section[]`, `symbols: Symbol[]`, `relocations: Relocation[]` | nameは診断用。各kindのsectionは最大1個 |
| Section | `kind: SectionKind`, `alignment: u32`, `data: bytes`, `memory_size: u32` | kindはText/Rodata/Data/Bss。alignmentはbyte単位、dataはinitialized bytes、memory_sizeはzero末尾を含むsize |
| Symbol | `name: string`, `binding: SymbolBinding`, `kind: SymbolKind`, `section: SectionKind or null`, `offset: u32`, `size: u32` | bindingはLocal/Exported/Imported、kindはFunction/Object/Section/Runtime |
| Relocation | `section: SectionKind`, `offset: u32`, `kind: RelocationKind`, `target: string`, `addend: i32` | section内patch位置、種別Abs32/Call16/Jump16/CallSlot/JumpSlot、参照symbol名とaddend |

section内offsetとsizeはbyte単位である。\
package/import/initialization順はPackageArtifact/ProgramMetadataで保持し、Objectの入力はsection、symbol、relocationの契約で完結する。\
globalは自然alignmentのBss storage、静的aggregate定数と文字列実体は自然alignmentのRodataとなり、zero-size storageは[layout規則](./targets/kagura-v1/machine.md#5-data-layout)に従う。\
patchの成立条件、symbol解決とbranch slot形式は[linking](./linking.md)で定義する。

## Link symbolの名前と所属
<!-- ond-spec-id: artifacts/link-symbol-names -->

通常の非generic function/globalのlink名は`namespace.source_name`とし、namespaceは論理package path、実行projectのroot packageに限り`main`とする。\
実行rootの論理package名とsymbol namespaceは別の識別であり、sourceの公開範囲は[名前規則](./language/v1/names-and-scopes.md)を適用する。\
Type/Constantの公開binding自体に独立したlink storageを要求しない。\
concrete methodとinitializerのlink名はConcreteMethod.symbol、PackageArtifact.initializerの明示fieldを正とする。

通常entry `main.main`とstartupが呼ぶinitializerは、対応ObjectのExported Functionとして参照可能であることを要求する。\
CLIが生成するstartupのentryは`__ond.entry`、heap境界は`__ond_heap_base`、stack下端は`__ond_stack_bottom`を使う。\
低level LinkPlanの3つのsymbol名はcaller指定とし、CLIの固定名と区別する。

| 生成者 | 専有する名前・namespace |
| --- | --- |
| startup | `__ond.entry`、`__ond_heap_cursor`、`__ond_heap_base_slot`、`__ond_stack_bottom_slot` |
| heap/stack境界 | LinkPlan.heap_base_symbol、LinkPlan.stack_bottom_symbol |
| test wrapper | `__ond.test.`にcases列の0始まりindexをleading zeroのない10進で続けた名前 |
| runtime helper | U+0001に`ond.runtime.`を続けたprefix（JSON表記`"\u0001ond.runtime."`） |

表の名前の定義者は対応する生成者とし、source/package ObjectはImported symbolとrelocationで参照できる。\
未知helperへの参照は通常の未解決symbolのlink errorとなる。

## LinkPlanとLinkedImage
<!-- ond-spec-id: artifacts/link-plan-and-image -->

LinkPlanとLinkedImageは配置入力と解決済みlink結果を記述する論理モデルであり、公開JSON wire形式または保存成果物のschemaを定義しない。\
実行可能バイナリの保存形式は[後掲の定義](#実行可能バイナリ)に従う。

| 型 | Field | 値と意味 |
| --- | --- | --- |
| MemoryRegion | `base: u32`, `size: u32` | byte addressとbyte size |
| LinkPlan | `read_only: MemoryRegion or null`, `memory_base: u32`, `memory_size: u32`, `stack_size: u32` | optional Text/Rodata領域、RAM領域、stack予約量 |
| LinkPlan | `entry_symbol: string`, `heap_base_symbol: string`, `stack_bottom_symbol: string` | entryとruntime境界symbol |
| LinkedSection | `kind: SectionKind`, `alignment: u32`, `load_address: u32`, `memory_size: u32`, `data: bytes` | 配置済みsectionとpatch済みinitialized bytes |
| LinkedImage | `entry_point: u32`, `ram_size: u32`, `stack_size: u32`, `sections: LinkedSection[]` | entry address、容量、配置済みsection列 |
| LinkedImage | `heap_base: u32`, `stack_bottom: u32`, `stack_top: u32`, `symbols: map<string,u32>` | runtime境界と下記の公開symbol address |

LinkedImage.symbolsは全入力ObjectのExported定義とLinkPlanの合成heap/stack境界だけのmapとする。\
同名LocalのObject間共存と解決は[linking](./linking.md#symbol解決と配置)に従う。\
startup/runtime symbolもExportedなら同じ規則で収録する。\
LinkedImageはruntime addressとpatch済みbytesを表す。\
各値は[linking](./linking.md)と実行可能バイナリの配置契約から導く。

## 実行可能バイナリ
<!-- ond-spec-id: artifacts/executable-binary-draft -->

本節は、Ond Language v1、target profile `kagura-v1`、ABI `ond-kagura-v1`の通常/test buildを表す実行可能バイナリを定義する。\
永続的な配置・起動・終了設定は、projectの[compileとlink設定](./build/packages.md#compileとlink設定)に従う。

中間成果物`.ondpkg`と`.ondbuild`は[PackageArtifactとCompilationManifest](#packageartifactとcompilationmanifest)のJSON wire形式に従う。\
debug sidecarとtest sidecarのJSON表現は、[debug metadata](./debug-metadata/v1.md)と[test build](./build/test-build.md)に従う。

### 標準実行バイナリ
<!-- ond-spec-id: artifacts/standard-executable-binary -->

標準実行バイナリは、32 byteの固定ヘッダ、24 byteのレコードを`section_count`個並べたセクション表、paddingとセクションデータから構成する。

```text
[固定ヘッダ: 32 byte | セクション表: 24 × section_count byte | padding・セクションデータ]
```

magic以外の全fieldはlittle-endianのu32とし、offsetとsizeの単位はbyteとする。\
固定ヘッダのoffsetと全`file_offset`は、標準実行バイナリの先頭を0として数える。

| ヘッダ内offset | Field | 値と意味 |
| --- | --- | --- |
| 0 | magic | ASCIIの`ONDX`を表す4 byte `4f 4e 44 58` |
| 4 | version | `1` |
| 8 | target | `1` = `kagura-v1` |
| 12 | abi | `1` = `ond-kagura-v1` |
| 16 | image_size | 固定ヘッダから最後のセクションデータ末尾までの標準実行バイナリ全長 |
| 20 | entry_address | ロード完了後に制御を渡すruntime entryの絶対address |
| 24 | initial_sp | entryへ制御を渡す際のstack pointer値 |
| 28 | section_count | セクション表のレコード数 |

セクション表はoffset 32から始まり、各レコードを次の形式で格納する。

| レコード内offset | Field | 値と意味 |
| --- | --- | --- |
| 0 | kind | `1` = Text、`2` = Rodata、`3` = Data、`4` = Bss |
| 4 | file_offset | 格納byte列の開始offset |
| 8 | file_size | ファイルに格納するbyte数 |
| 12 | memory_address | 実行時にセクションの先頭となる絶対address |
| 16 | memory_size | 実行時にセクションが占有するbyte数 |
| 20 | alignment | 実行時配置に必要なbyte単位のalignment |

レコードはkindの昇順とし、kindごとに最大1個を格納する。\
`memory_size`が0のセクションは省略し、Textは必須、`section_count`は1〜4とする。\
セクションの実行時範囲は`[memory_address, memory_address + memory_size)`であり、格納範囲は`file_size > 0`の場合の`[file_offset, file_offset + file_size)`である。

| 対象 | 成立条件 |
| --- | --- |
| 全セクション | `memory_size > 0`、alignmentは正の2冪、memory_addressはalignmentの倍数 |
| Text・Rodata | `file_size = memory_size` |
| Text | alignmentは4以上、memory_sizeは4の倍数 |
| Data | `0 <= file_size <= memory_size` |
| Bss | `file_offset = 0`、`file_size = 0` |
| 格納byte列が空のData | `file_offset = 0` |
| entry_address | 4-byte alignedで、Textの実行時範囲内の命令先頭 |
| 格納範囲 | 固定ヘッダ・セクション表と分離し、互いに重ならず、image_size以内 |
| 実行時範囲 | 互いに重ならず、[配置先領域](#配置容量と出力範囲)以内 |

`file_size > 0`の各格納範囲に対応するbyte列をセクションデータとする。\
固定ヘッダ・セクション表・セクションデータ以外の標準実行バイナリ内のbyteはpaddingであり、値を0とする。\
`image_size`は格納範囲の末尾の最大値に一致し、標準実行バイナリはちょうど`image_size` byteを持つ。

範囲端、セクション表の長さ、address、alignmentの切上げは数学的な整数として計算する。\
file範囲とu32 address範囲への切り詰めや折り返しを行わず、各計算結果は0〜4294967295の範囲に収める。\
magic、version、target、abiの不一致、欠落byte、未知kind、上記の成立条件違反は読出しerrorとなり、loaderはentryへ制御を渡さない。

### ロード完了時の状態
<!-- ond-spec-id: artifacts/executable-load-state -->

loaderは[配置方式](#配置容量と出力範囲)に従って、各セクションの格納byte列が`memory_address`から同じ順序で参照できる状態を成立させる。\
ROM上で直接参照するText・Rodataはその格納byte列を使い、RAMへ配置するbyte列は指定先へコピーする。\
各セクションの`[memory_address + file_size, memory_address + memory_size)`をzero初期化し、Bssと格納byte列が空のDataは実行時範囲全体をzeroとする。

`entry_address`へ制御を渡す時点では、全セクションの準備が完了し、stack pointerが`initial_sp`に一致する。\
ロード時にaddressの再配置は行わず、entryはRAMやROMの先頭addressではなくヘッダの指定値を使う。\
通常buildのinitializerと`main`の呼出しはentry側の責務であり、[言語の初期化とエントリ](./language/v1/execution.md#初期化とエントリ)に従う。\
test buildの共通entryと個別wrapperは[test起動入口](./build/test-build.md#buildと起動入口)に従う。

### 外部起動とセルフ起動
<!-- ond-spec-id: artifacts/executable-launch-modes -->

`link.startup = 'external'`の出力は標準実行バイナリそのものとする。\
外部loaderが[ロード完了時の状態](#ロード完了時の状態)を成立させ、entryへ制御を渡す。

```text
external = [標準実行バイナリ]
```

`link.startup = 'self'`の出力は、標準実行バイナリの前に起動処理を置いたセルフ起動形式とする。

```text
self = [前置起動処理 | 標準実行バイナリ]
```

selfは`link.rom`を必須とし、ROM baseを0とする。\
実行環境は出力全体をROM address 0から同じ順序で配置し、CPUをresetする。\
CPU resetによる実行開始は、[固定Kagura規範](./targets/kagura-v1/README.md#参照するkagura規範)のPC 0に従う。\
PC 0から始まる前置起動処理がloaderの役割を担い、SPを含む[ロード完了時の状態](#ロード完了時の状態)を成立させてentryへ制御を渡す。\
外部bootloaderやhostによるSP設定をselfの成立前提に含めない。

前置起動処理の格納長は標準実行バイナリまでのpaddingを含む4の倍数とし、link時に確定する。\
前置起動処理はその確定位置のヘッダを使い、magicの探索によって標準実行バイナリを検出しない。\
selfでも`file_offset`の基準と`image_size`の対象は標準実行バイナリだけであり、前置起動処理を含めない。

### 配置容量と出力範囲
<!-- ond-spec-id: artifacts/executable-placement-capacity -->

ROMとRAMは、それぞれ[link設定](./build/packages.md#compileとlink設定)で指定した単一の連続するflat address範囲とする。\
実行時の配置とaddress解決はこの範囲に対してlink時に確定し、bank切替による実行addressの変更を含めない。\
externalの保存元とその読出し方法は外部loaderが管理し、保存元bankからRAMへのコピーもこの責務に属する。

`link.rom`がある場合、Text・RodataをROM上で直接参照し、Data・BssをRAMへ配置する。\
標準実行バイナリ先頭のROM addressを`B`とし、Text・Rodataの`memory_address = B + file_offset`を要求する。\
externalでは標準実行バイナリ全体を`link.rom.base`から配置する前提でlinkし、`B = link.rom.base`とする。\
selfでは前置起動処理を含む出力全体をROM base 0から配置するため、`B`は前置起動処理の格納長に一致する。\
Data・BssのRAM配置は[Objectの結合順とalignment](./linking.md#symbol解決と配置)に従う。

`link.rom`を省略したexternalは、固定ヘッダを含む標準実行バイナリ全体を`link.ram.base`から同じ順序でRAMへコピーする前提でlinkする。\
`file_size > 0`の全セクションは`memory_address = link.ram.base + file_offset`を満たす。\
Textは必須であるため、loaderはTextの`memory_address - file_offset`からこのコピー先のbaseを得られる。\
ヘッダ・セクション表・paddingもRAMを占有し、コピー範囲をheapへ転用しない。\
Dataのzero末尾、格納byte列が空のData、Bssは、コピー範囲の末尾`link.ram.base + image_size`以降へ配置し、他セクションの実行時範囲と分離する。\
ファイル内のセクション配置は、このコピー範囲とzero初期化範囲の分離、および[セクションの成立条件](#標準実行バイナリ)を満たす。

`RAM_END = link.ram.base + link.ram.size`、`STACK_TOP = RAM_END`、`initial_sp = STACK_TOP`、`STACK_BOTTOM = STACK_TOP - link.stack-size`とする。\
RAM baseとstack-sizeは4-byte aligned、RAM_ENDは8-byte alignedで、`0 < link.stack-size < link.ram.size`を要求する。\
heap_baseはRAM baseとRAM上の全セクション末尾の最大値を4-byte alignedへ切り上げたaddressとし、ROM省略時はコピー範囲の末尾も最大値の候補に含める。\
RAM上のセクション、コピー範囲、heap_baseはSTACK_BOTTOM以下に収め、heapとstackの利用は[targetのRAM layout](./targets/kagura-v1/machine.md#3-ram-layout)に従う。

ROM指定時はexternalの標準実行バイナリ全体、selfの前置起動処理を含む出力全体をROM容量内へ収める。\
ROM省略時は標準実行バイナリ全体とRAM上のzero初期化領域をRAM容量内へ収める。\
ROM容量とRAM容量は配置先の上限であり、容量末尾までのpaddingを出力へ付けない。\
出力の末尾は標準実行バイナリの末尾と一致する。\
起動方式ごとのaddressを解決してlinkするため、共通の標準形式を使うことはexternalとselfの出力byte列の一致を意味しない。

### RAMコピーのwire例
<!-- ond-spec-id: artifacts/executable-ram-copy-example -->

次の例は、target `kagura-v1`、ABI `ond-kagura-v1`、startup external、ROM省略、RAM base `0x10000000`、RAM size `0x100`、stack-size `0x40`に対するwire読出しとロードの適合判定を対象とする。\
観測点はentryへ制御を渡す直前とし、RAMの初期byte値は任意とする。\
Textの命令列をOnd programとして実行する場合の適合判定は、[言語規則](./language/v1/README.md)と[通常buildのstartup](./targets/kagura-v1/machine.md#4-startup)に従って別に行う。

| ヘッダfield | 値 |
| --- | --- |
| magic・version・target・abi | `ONDX`・`1`・`1`・`1` |
| image_size | `0x54` |
| entry_address | `0x10000050` |
| initial_sp | `0x10000100` |
| section_count | `2` |

| kind | file_offset | file_size | memory_address | memory_size | alignment |
| --- | --- | --- | --- | --- | --- |
| Text (`1`) | `0x50` | `4` | `0x10000050` | `4` | `4` |
| Bss (`4`) | `0` | `0` | `0x10000054` | `4` | `4` |

ヘッダと2個のレコードを前記のlittle-endian形式で並べ、offset `0x50`へTextのbyte列`00 00 00 c0`を置くと、標準実行バイナリは84 byteとなる。\
loaderはTextの`memory_address - file_offset = 0x10000000`をコピー先として84 byte全体をRAMへコピーし、`[0x10000054, 0x10000058)`をzero初期化する。\
観測点では、コピー先の84 byteが標準実行バイナリとbyte単位で一致し、続くBssの4 byteがすべて0、SPが`0x10000100`、制御移行先が`0x10000050`となる。\
配置契約からheap_baseは`0x10000058`、STACK_BOTTOMは`0x100000c0`に定まる。
