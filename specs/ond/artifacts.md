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

compiler_id、build_id、content_hashの保存値は[SHA-256識別子](./build/cache.md#公開識別子)の小文字hex64桁とする。\
image hash計算用の空build_idは計算時だけの値であり、保存するimageは上記の識別子を持つ。

schema違反は公開入力のerrorとなり、compile/link CLIでは[失敗契約](./build/commands.md#入出力と失敗時)を適用する。\
公開JSONの生成byte列とhashは[canonical JSONと識別子](./build/cache.md#canonical-json)に従う。

## 公開interface
<!-- ond-spec-id: build/public-metadata-revision-3 -->

成果物の外層contractはrevision 6とし、[版識別と互換条件](#版識別と互換条件)を適用する。

`interface`はpackage、types（型定義の配列）、exports（公開名→宣言）からなる。\
target-neutralな型・公開宣言情報であり、型参照はこの成果物localのtypes配列indexで表す。

generic型・関数・method・operatorを利用するpackageはsource宣言から具体化する。\
具体化済みの型と関数は、通常の非generic成果物と同じ形式で扱う。

| 型 | Fieldまたはvariant | 意味 |
| --- | --- | --- |
| Interface | `package: string`, `types: Type[]`, `exports: map<string,Export>` | 論理package path、型配列、ASCII大文字で始まる公開名からExportへのmap |
| Type | `name: string or null`, `kind: Kind`, `methods: ConcreteMethod[]` | 任意の完全修飾nominal名、型構造、公開concrete method |
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
公開宣言から到達する型を再帰的に保存し、非公開のnamed typeも公開戻り値などから到達すればtypesへ含める。\
exportsは公開bindingの集合であり、pointerによる循環型参照も成立する。\
Compositeの省略要素は指定型のzero valueとする。\
IntegerとFloat32の値はそれぞれu64/u32のbit表現として運ぶ。

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
Object、LinkPlan、LinkedImageも本書のJSON wire形式と後掲の各field集合で表す。

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
各artifactのcase.packageは当該artifact.packageに一致する。\
各caseは所属packageのparameter/resultなしでTest prefixを持つfunctionを指し、symbolは[名前規則](#link-symbolの名前と所属)と対応する定義Functionに一致する。\
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

| 型 | Field | 値と意味 |
| --- | --- | --- |
| MemoryRegion | `base: u32`, `size: u32` | byte addressとbyte size |
| LinkPlan | `read_only: MemoryRegion or null`, `memory_base: u32`, `memory_size: u32`, `stack_size: u32` | optional Text/Rodata領域、RAM領域、stack予約量 |
| LinkPlan | `entry_symbol: string`, `heap_base_symbol: string`, `stack_bottom_symbol: string` | entryとruntime境界symbol |
| LinkedSection | `kind: SectionKind`, `alignment: u32`, `load_address: u32`, `memory_size: u32`, `data: bytes` | 配置済みsectionとpatch済みinitialized bytes |
| LinkedImage | `build_id: string`, `entry_point: u32`, `ram_size: u32`, `stack_size: u32`, `sections: LinkedSection[]` | image識別、entry address、容量、loadするsection列 |
| LinkedImage | `heap_base: u32`, `stack_bottom: u32`, `stack_top: u32`, `symbols: map<string,u32>` | runtime境界と下記の公開symbol address |

LinkedImage.symbolsは全入力ObjectのExported定義とLinkPlanの合成heap/stack境界だけのmapとする。\
同名LocalのObject間共存と解決は[linking](./linking.md#symbol解決と配置)に従う。\
startup/runtime symbolもExportedなら同じ規則で収録する。\
LinkedImageはruntime addressとpatch済みbytesを表す。\
loaderはこれをloadし、各sectionのdata末尾からmemory_sizeまでをzero初期化する。\
配布ツールはLinkedImageを配布形式へ配置する。\
image build_idは[公開識別子](./build/cache.md#公開識別子)の規則で求める。
