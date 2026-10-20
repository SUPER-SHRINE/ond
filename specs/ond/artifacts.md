# Ond成果物と公開interface
<!-- ond-spec-id: artifacts/v1 -->

## 公開interface（旧revision 3の主題）
<!-- ond-spec-id: build/public-metadata-revision-3 -->

この節の歴史IDはrevision 3で導入した公開情報の主題を追跡する。\
現行成果物の外層contractはrevision 5であり、下表の互換条件を適用する。

`interface`はpackage、types（型定義の配列）、exports（公開名→宣言）からなる。\
target-neutralな型・公開宣言情報であり、型参照はこの成果物localのtypes配列indexで表す。

generic型・関数・method・operatorを利用するpackageはsource宣言から具体化する。\
具体化済みの型と関数は、通常の非generic成果物と同じ形式で扱う。

| 型 | Fieldまたはvariant | 意味 |
| --- | --- | --- |
| Interface | `package`, `types`, `exports` | 論理package path、型配列、ASCII大文字で始まる公開名からExportへのmap |
| Type | `name: string or null`, `kind: Kind`, `methods: ConcreteMethod[]` | 任意の完全修飾nominal名、型構造、公開concrete method |
| Kind | `Bool`, `U8`, `I8`, `U16`, `I16`, `U32`, `I32`, `F32` | primitive型 |
| Kind | `Pointer(TypeRef)`, `Array {length: u32, element: TypeRef}` | 参照先型、配列長と要素型 |
| Kind | `Function(Signature)`, `Struct(Field[])`, `Defined(TypeRef)` | 関数型、宣言順field列、underlying型 |
| Signature | `parameters: TypeRef[]`, `returns: TypeRef[]` | source-visibleな引数型列と戻り値型列 |
| Field | `name`, `ty: TypeRef`, `private_owner: string or null` | field名と型、非公開fieldの所有package path |
| ConcreteMethod | `name`, `signature`, `pointer_receiver: bool`, `symbol` | 公開method名、receiverを除いたsignature、receiver種別とlink symbol |
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

成果物の受理は現行Contract tupleとの一致を条件とする。\
cache再利用は[cache契約](./build/cache.md)に従う。

## 版識別と互換条件
<!-- ond-spec-id: artifacts/version-compatibility -->

| 識別軸 | 適用対象 | 現行値 | 依存と互換条件 |
| --- | --- | --- | --- |
| language | sourceの構文・型・意味 | Ond v1 | [language v1](./language/v1/README.md)の入力と規則に適用 |
| target profile | machine/layout/runtime | `kagura-v1` | languageのtarget委譲を具体化 |
| ABI | call/register/引数・戻り値 | `ond-kagura-v1` | `kagura-v1`に適用 |
| artifact Contract | PackageArtifactとCompilationManifestの外層 | revision `5`, target `kagura-v1`, abi `ond-kagura-v1` | tuple全要素が現行値に一致すると受理。Contractの未知fieldはerror |
| compiler_id | artifact生成元とcache再利用の識別 | 実行executable全体のSHA-256 | CLI linkはmanifest、全artifact、実行linkerの同一値を要求 |

language版、target版、ABI版は対応する規範文書の適用対象を識別する。\
成果物に記録する版識別は上表の既存Contract tupleである。\
`compiler_id`は同じ生成元を識別する値であり、一致によって生成元と再利用の条件を判定する。\
in-memory Object、LinkPlan、LinkedImageは後掲の各field集合で表す。

## PackageArtifactとCompilationManifest
<!-- ond-spec-id: artifacts/package-and-manifest -->

`.ondpkg`と`.ondbuild`はUTF-8 JSONであり、byte列であるsection dataはBase64文字列で表す。\
以下は既存fieldの名前と意味を記す。

| 型 | Field | 意味 |
| --- | --- | --- |
| Contract | `revision: u32`, `target: string`, `abi: string` | 前節の現行tuple |
| PackageArtifact | `contract`, `compiler_id`, `package`, `build_id` | 外層契約、生成元、論理package path、package内容識別 |
| PackageArtifact | `dependencies: map<string,string>` | 直接依存package pathからbuild_idへの対応 |
| PackageArtifact | `imports: string[]`, `initializer: string or null` | source順のimport path、global初期化関数symbol |
| PackageArtifact | `interface`, `object`, `debug` | 公開interface、Object、link前debug情報 |
| PackageArtifact | `tests` | test compile時のTestMetadata。通常compileはfieldを省略 |
| CompilationManifest | `contract`, `compiler_id` | 外層契約と生成元 |
| CompilationManifest | `packages: ArtifactRef[]`, `program: ProgramMetadata` | 決定的なObject入力順のartifact列、実行計画 |
| CompilationManifest | `tests` | test compile時のTestMetadata。case 0件も`cases: []`を記録 |
| ArtifactRef | `package`, `build_id`, `path`, `content_hash` | package path、内容識別、artifact file path、file bytesのSHA-256 |
| ProgramMetadata | `entry_symbol`, `initializers: string[]` | entry symbol、言語初期化順に並ぶinitializer symbol |

Object入力順とinitializer実行順は別の列として保持する。\
testsのcase列と`.ondtest`は[test build契約](./build/test-build.md)を定義元とする。\
CompilationManifest、Contract、TestMetadataとcaseは上表と[test schema](./build/test-build.md#buildと起動入口)のfield集合で検査し、未知fieldはerrorとなる。\
明示`tests: null`はerrorとなり、field省略とcase 0件のtest buildを区別する。\
PackageArtifactとinterfaceのpackageは一致し、型参照、public名、nominal名、method名/symbol、aggregate indexの整合性を読出時に検査する。\
build_idとcache keyの生成は[cache契約](./build/cache.md)、debug schemaは[debug metadata](./debug-metadata/v1.md)に従う。

## Object
<!-- ond-spec-id: artifacts/object -->

| 型 | Field | 値と意味 |
| --- | --- | --- |
| Object | `name: string`, `sections: Section[]`, `symbols: Symbol[]`, `relocations: Relocation[]` | nameは診断用。各kindのsectionは最大1個 |
| Section | `kind`, `alignment: u32`, `data: bytes`, `memory_size: u32` | kindはText/Rodata/Data/Bss。alignmentはbyte単位、dataはinitialized bytes、memory_sizeはzero末尾を含むsize |
| Symbol | `name`, `binding`, `kind`, `section: SectionKind or null`, `offset: u32`, `size: u32` | bindingはLocal/Exported/Imported、kindはFunction/Object/Section/Runtime |
| Relocation | `section`, `offset: u32`, `kind`, `target: string`, `addend: i32` | section内patch位置、種別Abs32/Call16/Jump16/CallSlot/JumpSlot、参照symbol名とaddend |

section内offsetとsizeはbyte単位である。\
package/import/initialization順はPackageArtifact/ProgramMetadataで保持し、Objectの入力はsection、symbol、relocationの契約で完結する。\
globalは自然alignmentのBss storage、静的aggregate定数と文字列実体は自然alignmentのRodataとなり、zero-size storageは[layout規則](./targets/kagura-v1/machine.md#5-data-layout)に従う。\
patchの成立条件、symbol解決とbranch slot形式は[linking](./linking.md)で定義する。

## LinkPlanとLinkedImage
<!-- ond-spec-id: artifacts/link-plan-and-image -->

| 型 | Field | 値と意味 |
| --- | --- | --- |
| MemoryRegion | `base: u32`, `size: u32` | byte addressとbyte size |
| LinkPlan | `read_only: MemoryRegion or null`, `memory_base: u32`, `memory_size: u32`, `stack_size: u32` | optional Text/Rodata領域、RAM領域、stack予約量 |
| LinkPlan | `entry_symbol`, `heap_base_symbol`, `stack_bottom_symbol` | entryとruntime境界symbol |
| LinkedSection | `kind`, `alignment: u32`, `load_address: u32`, `memory_size: u32`, `data: bytes` | 配置済みsectionとpatch済みinitialized bytes |
| LinkedImage | `build_id`, `entry_point: u32`, `ram_size: u32`, `stack_size: u32`, `sections: LinkedSection[]` | image識別、entry address、容量、loadするsection列 |
| LinkedImage | `heap_base: u32`, `stack_bottom: u32`, `stack_top: u32`, `symbols: map<string,u32>` | runtime境界と解決済みsymbol address |

LinkedImageはruntime addressとpatch済みbytesを表す。\
loaderはこれをloadし、各sectionのdata末尾からmemory_sizeまでをzero初期化する。\
配布ツールはLinkedImageを配布形式へ配置する。\
CLI出力のimage build_idは[link command](./build/commands.md#ond-link)の規則で求める。
