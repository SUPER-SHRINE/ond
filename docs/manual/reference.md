# 仕様書参照

## 字句と構文

- source形式、identifier、literal、comment: [lexical](../../specs/ond/language/v1/lexical.md)
- 改行とsemicolon: [lexical](../../specs/ond/language/v1/lexical.md)
- packageとimport、scope: [names and scopes](../../specs/ond/language/v1/names-and-scopes.md)
- operator precedence: [expressions](../../specs/ond/language/v1/expressions.md)
- grammar: [概念別EBNF索引](../../specs/ond/language/v1/lexical.md)

## 言語意味

- declaration、型、generic: [declarations](../../specs/ond/language/v1/declarations.md)、[types](../../specs/ond/language/v1/types.md)
- pointer、array、struct: [types](../../specs/ond/language/v1/types.md)
- 式、演算、文字列: [expressions](../../specs/ond/language/v1/expressions.md)
- statement、defer: [statements](../../specs/ond/language/v1/statements.md)
- memory、init、entry: [execution](../../specs/ond/language/v1/execution.md)
- builtins: [builtins](../../specs/ond/language/v1/builtins.md)

## Target・成果物・build

- machine layout/runtime: [machine profile](../../specs/ond/targets/kagura-v1/machine.md)
- calling convention: [ABI](../../specs/ond/targets/kagura-v1/abi.md)
- Object/LinkedImage: [artifacts](../../specs/ond/artifacts.md)
- linker: [linking](../../specs/ond/linking.md)
- package、compile、link、cache: [packages](../../specs/ond/build/packages.md)、[commands](../../specs/ond/build/commands.md)、[cache](../../specs/ond/build/cache.md)
- test build: [test build](../../specs/ond/build/test-build.md)
- debug mapping/fault/unwind: [debug metadata](../../specs/ond/debug-metadata/v1.md)
- requirements and cases: [requirements](../../specs/ond/conformance/requirements.md)、[case index](../../specs/ond/conformance/cases/README.md)
