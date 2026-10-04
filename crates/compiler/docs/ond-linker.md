# Ond / Kagura linker contract

The compiler translates source through MIR into Objects. The linker consumes Objects and a caller-owned LinkPlan, and returns a LinkedImage; it does not serialize a distribution format or load a host machine.

## Object and symbols

Objects contain sections, symbols and relocations. Source package/import/init metadata is not part of Object. Section kinds are Text, Rodata, Data and Bss; each Object has at most one section of each kind.

Local symbols resolve only within their Object. Exported symbols form the shared namespace. Undefined imports, duplicate exports, malformed symbol ranges, invalid alignment, overlapping patch sites and references into branch-slot interiors are rejected.

## Layout

The caller selects memory base/size, stack reservation, optional read-only region, entry symbol and heap/stack boundary symbols. Section placement and all address/offset arithmetic are checked. Insufficient capacity is an error; the linker never raises stack or RAM limits.

The resulting LinkedImage contains load addresses, initialized bytes, section memory sizes, entry point, heap/stack boundaries and resolved symbols. A machine SDK loads payload bytes and zero-initializes the remaining storage.

## Relocations and branches

Abs32 uses checked address-plus-addend arithmetic. Call16/Jump16 require their signed word displacement to fit and diagnose overflow. CallSlot/JumpSlot reserve fixed-footprint space for a direct branch or a far-branch thunk, without link-time insertion. Startup calls use the same slot convention. See [implementation contract](../src/linker/README.md) and [backend design](../../../specs/ond/targets/kagura-v1/mir-backend.md).

## SDK boundary

ProgramMetadata obtains initializer order and main from validated MIR. The shared program_startup helper emits runtime ABI glue with caller-supplied termination instructions. Enbu SDK supplies RAM policy, loader, ROM banks, asset directory and raw ROM output; the compiler does not depend on Enbu SDK.

Compiler tests use test_support to load LinkedImage directly into a synthetic bus. This harness is cfg(test)-only and does not define a public machine or distribution format.
