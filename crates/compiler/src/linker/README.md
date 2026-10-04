# Kagura linker and Object boundary

The linker owns section placement, symbol resolution, relocation and fixed-size
branch islands. It does not compile MIR, synthesize startup or encode containers.
LinkError is independent of frontend diagnostics.

LinkPlan supplies RAM base/size, stack reservation, entry/runtime symbol names
and an optional disjoint read-only region for Text/Rodata. LinkedImage returns
patched bytes, runtime addresses, heap/stack boundaries and exported symbols.
The cfg(test)-only harness uses a synthetic RAM base and loads LinkedImage directly. Distribution output belongs to the machine SDK.

Object contains name (diagnostic only), sections, symbols and relocations.
PackageId/import/init/main metadata is no longer part of Object. ProgramMetadata
derives ordered initializers from validated MIR without linker graph traversal.
Local names resolve inside their input Object; only exports are globally visible.
Relocations may refer to exports without redundant Imported declarations.
Explicit Imported declarations are also checked for unresolved symbols.

Validation rejects duplicate section kinds/symbols/exports, missing sections,
invalid alignment/size, malformed imports, patch range overflow/overlap,
reserved symbol collisions, references into slot interiors and checked address
overflow. Abs32 addends no longer silently wrap. Entry must be an exported
function; branch relocations must target executable text.

This remains an in-memory contract inside ond-compiler, not a standalone crate,
serialized object format, instruction verifier or ABI-version negotiation.
There is one section per kind per Object; SectionId-based multi-section layout,
dead stripping and full instruction/CFG verification are separate future work.

The independent enbu-sdk crate uses split layout for a temporary 4096-byte boot-bank profile.
It reserves boot bytes in ROM and a return-loop word in RAM. It copies Data,
clears Bss and sets SP before ordered initialization and main. See the adjacent
Enbu repository's examples/ond-boot for actual C-machine reset-path verification.
