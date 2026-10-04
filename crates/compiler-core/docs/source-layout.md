# Compiler core source layout

The public entry points live in `src/lib.rs`. Implementation modules are private;
this organization does not change the public API or language behavior.

## Frontend pipeline

| Module | Responsibility |
| --- | --- |
| `pipeline/mod.rs` | Expose HIR and MIR lowering internally to the compilation entry point |
| `pipeline/hir/mod.rs` | Assemble packages and symbols; validate top-level declarations and imports |
| `pipeline/hir/types.rs` | Index package declarations; resolve type definitions, signatures and array lengths |
| `pipeline/hir/body_names.rs` | Validate type-independent value names and lexical scopes in generic bodies at definition time |
| `pipeline/hir/generic_functions.rs` | Discover and lower requested generic function, method and operator specializations; attach instantiation chains to diagnostics |
| `pipeline/hir/operators.rs` | Name concrete and generic operator implementations and determine their owning package |
| `pipeline/hir/control_flow.rs` | Analyze AST fallthrough and loop exits |
| `pipeline/hir/body/mod.rs` | Own function-local state, local bindings and diagnostic handling |
| `pipeline/hir/body/statements.rs` | Lower declarations, assignments, returns and structured control flow |
| `pipeline/hir/body/expressions.rs` | Resolve expression names and check expression types |
| `pipeline/hir/body/values.rs` | Apply mandatory exact constant evaluation at typed value boundaries without propagating variable initializers |
| `pipeline/hir/body/memory.rs` | Check addressability, pointer dereferences, struct visibility and indices |
| `pipeline/hir/body/composites.rs` | Infer array literal lengths and check sparse array/named struct initializers in source order |
| `pipeline/hir/body/globals.rs` | Resolve global storage types and check initializer values without capturing local scope |
| `pipeline/hir/body/interfaces.rs` | Check explicit pointer-to-interface conversions, method sets and interface equality |
| `pipeline/hir/initialization.rs` | Build package initializer bodies and source-import-ordered startup traversal |
| `pipeline/hir/body/intrinsics.rs` | Type new/alloc/free/load32/store32 without target layout |
| `pipeline/hir/body/strings.rs` | Builtin name lookup and type-level array `len` |
| `string_literal.rs` | Decode UTF-8 source, raw strings and byte/Unicode escapes |
| `identifier.rs` | Classify Unicode Letter/Nd identifier characters (not Alphabetic/XID) |
| `numeric_literal.rs` | Scan numeric spelling and convert radix-aware integers/decimal floats |
| `numeric_literal/hex_float.rs` | Round hexadecimal literals directly to binary32 |
| `pipeline/hir/body/calls.rs` | Resolve function values; check calls, argument types and multiple-result contexts |
| `pipeline/hir/body/constants.rs` | Resolve scoped constant references and connect them to evaluation |
| `constant.rs` | Evaluate typed scalar/sparse aggregate constants with arbitrary-precision integer intermediates, without package or scope state |
| `pipeline/mir_typed.rs` | Build MIR instructions and CFG from typed HIR |
| `pipeline/mir_typed/memory.rs` | Lower symbolic places, aggregate copies and array bounds checks |
| `mir/validate_memory.rs` | Validate the type chain of each place projection |
| `mir/validate_globals.rs` | Check static symbols, global types and initialization metadata |
| `mir/validate_flow.rs` | Check value definition order and reachable CFG dominance |
| `mir/validate_operations.rs` | Check operation domains, intrinsics and direct function targets |
| `mir/validate_types.rs` | Reject recursive value layouts and duplicate struct fields |
| `mir/validate_guards.rs` | Require dominating runtime guards or static safety proofs |

The body submodules share one `BodyLowerer` state. They do not maintain separate
copies of local scope or diagnostics. Statement lowering restores local state after
an error, poisons names introduced by failed declarations, and continues with
independent statements without manufacturing a placeholder type. Type resolution and constant resolution
cooperate for constant array lengths; the pure constant evaluator does not depend
on either resolver. Visibility is restricted to the owning module hierarchy.

Use ordinary Rust modules for pipeline implementation, not textual `include!`,
so dependencies are explicit and standard formatting traverses each source file.

## Representations and tests

`ir.rs` contains AST/HIR data models; `semantic.rs` owns type IDs and the type
table. `mir.rs` contains the MIR model, with display and validation in `mir/`.
The `ond/` modules own parsing, independently of lowering.

`conformance/` contains the test-only spec case schema, isolated fixture lifecycle,
runner, seed registry and harness rejection tests. The `scope`, `precedence` and
`constant_contexts` modules own package/entry contracts, AST binding/value checks
and Q-03 constant contexts respectively. See [case format](./conformance-cases.md).
`conformance/numeric/` separates integer boundaries/operators, conversions,
binary32 goldens, dynamic MIR contracts and backend-ready oracle vectors.
See [numeric matrix and verification boundaries](./numeric-conformance.md).
`conformance/evaluation.rs` executes small MIR programs and compares source-side
effect traces and final writes. `flow.rs`, `names.rs` and `name_duplicates.rs`
cover return paths and binding scopes. `control_headers.rs` verifies empty control
bodies versus composite literals, including branch/post execution.
`ond/control_syntax.rs` owns the narrowly scoped header parsing context.
See [control-flow matrix](./control-flow-conformance.md) for the regression cases.

`ond/signatures.rs` shares signature parsing while keeping mandatory declaration
parameter names separate from optional function-type documentation names.
`function_type_tests.rs` checks mixed named/unnamed parameters, source spans,
type identity, callback use and rejection of unnamed declaration parameters.

`type_identity_tests.rs` covers source-independent structural identity, nominal
types, package-private fields, equality/hash consistency and owner validation.

`constant_tests.rs` covers exact arithmetic, shift normalization, aggregate constants,
constant eligibility, binary32 results and checked-value handoff to MIR.
`constant_value_tests.rs` checks mandatory evaluation across assignments, calls,
returns and globals, runtime-operation preservation, and single evaluation of array
`len` operands.
`inferred_array_tests.rs` checks inferred length, sparse/keyed elements, fixed-type
compatibility, source evaluation order, integer boundaries and invalid inference contexts.

`tests.rs` covers end-to-end compilation and type integration,
`control_flow_tests.rs` covers CFG behavior, and `semantic_tests.rs` covers name
resolution, constants and diagnostics. `call_tests.rs` checks direct/indirect
calls, multiple results, evaluation order and recursion using a test-only MIR
executor. Shared source compilation setup is in `test_support.rs`.
`memory_tests.rs` and `memory_test_vm.rs` verify aggregate snapshots, pointer
aliasing, byte-array string literals, initialization order and bounds faults without relying on
the legacy AST backend. The test VM is not a production runtime.
`global_tests.rs` verifies startup ordering, zero initialization, global mutation,
forward references, initializer diagnostics and global aggregate copies.
Keep regression tests separate from the public entry-point implementation.

`conformance/diagnostics.rs` verifies exact lexer/parser/HIR diagnostics across
UTF-8, CRLF, imported packages and multiple files. `test_support::reject` and
`reject_at` adapt existing negative tests to the same diagnostic runner.
See `diagnostic-conformance.md` for diagnostic contracts and scope.

`conformance/boundaries/` contains the generated P1 type/value matrices:
`conversions.rs` (implicit/explicit conversions and nil), `operators.rs`
(operand/result type contracts), `builtins.rs` (arity/types/results), and
`declarations.rs` (inference, zero storage, constant eligibility).
`mod.rs` adapts rows to the common conformance runner and fixes the inventory.
See `type-boundary-conformance.md` for scope and backend exclusions.

When a file grows beyond roughly 1,000 lines, proactively split cohesive
responsibilities into modules rather than continuing to extend one large file.
`completion_tests.rs` covers the strict compilation contract, memory intrinsics,
literal diagnostics, and mutation-based negative validator tests.

`syntax_discard_tests.rs` covers blank identifiers, source-order evaluation of
discarded values, and statement syntax acceptance/rejection through the MIR test VM.

`ond/lexical_tests.rs` checks token boundaries/spans, Unicode categories, literal
spelling errors, string escapes, EOF handling, and hexadecimal float rounding.
