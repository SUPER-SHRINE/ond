# Kagura linker and Object boundary

The linker owns section placement, symbol resolution, relocation and fixed-size
branch islands. It does not compile MIR, synthesize startup or encode containers.
LinkError is independent of frontend diagnostics.

LinkPlan supplies RAM base/size, stack reservation, entry/runtime symbol names
and an optional disjoint read-only region for Text/Rodata. LinkedImage returns
patched bytes, runtime addresses, heap/stack boundaries and exported symbols.
Distribution output belongs to the machine SDK.

Object contains name (diagnostic only), sections, symbols and relocations.
Object does not contain PackageId/import/init/main metadata. ProgramMetadata
derives ordered initializers from validated MIR without linker graph traversal.
Local names resolve inside their input Object; only exports are globally visible.
Relocations may refer to exports without redundant Imported declarations.
Explicit Imported declarations are also checked for unresolved symbols.

Validation rejects duplicate section kinds/symbols/exports, missing sections,
invalid alignment/size, malformed imports, patch range overflow/overlap,
reserved symbol collisions, references into slot interiors and checked address
overflow. Abs32 address-plus-addend overflow is rejected. Entry must be an
exported function; branch relocations must target executable text.

This remains an in-memory contract inside ond-compiler, not a standalone crate,
serialized object format, instruction verifier or ABI-version negotiation.
There is one section per kind per Object.
