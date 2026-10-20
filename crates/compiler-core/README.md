# ond-compiler-core

`ond-compiler-core` is the target-neutral Ond frontend. It owns source discovery,
parsing, HIR construction, and MIR construction.

It intentionally does not depend on a target ISA, object format, linker,
executable format, or cartridge format. Target backends consume its MIR and are
responsible for code generation and object emission.

The `ond-compiler` crate provides the Kagura backend and linker on top of this
crate. Cartridge layout and asset packaging belong to a machine SDK such as Enbu.

The target-neutral MIR contract is documented in [docs/mir.md](docs/mir.md).

Successful compilation returns fully lowered HIR/MIR and automatically runs the
MIR validator. Unsupported source constructs produce diagnostics; compilation
does not return partial bodies. The retained AST in `Compilation` is available
for source-oriented tools and inspection; code generation consumes validated MIR.
