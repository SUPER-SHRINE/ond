use super::*;

#[test]
fn narrow_f32_roundtrip_keeps_shared_float_and_rewrites_nested_casts_across_blocks() {
    let span = Span::synthetic();
    let value = ValueId;
    let cast = |id, result, operand, to, behavior| Instruction {
        id: InstructionId(id),
        results: vec![value(result)],
        kind: InstructionKind::Cast {
            value: value(operand),
            to,
            behavior,
        },
        span,
    };
    let mut function = Function {
        name: "shared".into(),
        parameters: vec![],
        returns: vec![TypeId::F32, TypeId::I16, TypeId::I16],
        locals: vec![],
        values: [
            TypeId::I16,
            TypeId::F32,
            TypeId::I16,
            TypeId::F32,
            TypeId::I16,
            TypeId::I16,
        ]
        .into_iter()
        .enumerate()
        .map(|(id, ty)| Value {
            id: value(id as u32),
            ty,
            span,
        })
        .collect(),
        // The producer block and the first round trip's result have higher IDs.
        entry: BlockId(1),
        blocks: vec![
            BasicBlock {
                id: BlockId(0),
                parameters: vec![],
                instructions: vec![
                    cast(2, 4, 1, TypeId::I16, CastBehavior::Checked),
                    cast(3, 3, 4, TypeId::F32, CastBehavior::Infallible),
                    cast(4, 0, 3, TypeId::I16, CastBehavior::Checked),
                    cast(5, 2, 1, TypeId::I16, CastBehavior::Checked),
                ],
                terminator: Terminator {
                    kind: TerminatorKind::Return(vec![value(1), value(0), value(2)]),
                    span,
                },
            },
            BasicBlock {
                id: BlockId(1),
                parameters: vec![],
                instructions: vec![
                    Instruction {
                        id: InstructionId(0),
                        results: vec![value(5)],
                        kind: InstructionKind::Constant(Constant::Integer {
                            bits: 42,
                            ty: TypeId::I16,
                        }),
                        span,
                    },
                    cast(1, 1, 5, TypeId::F32, CastBehavior::Infallible),
                ],
                terminator: Terminator {
                    kind: TerminatorKind::Jump(Edge {
                        target: BlockId(0),
                        arguments: vec![],
                    }),
                    span,
                },
            },
        ],
        span,
    };
    let types = TypeTable::new();
    validate_function(&types, &function).unwrap();
    elide_narrow_f32_roundtrips(
        &mut function,
        &BTreeSet::from([value(4), value(0), value(2)]),
    );
    validate_function(&types, &function).unwrap();
    assert!(function.blocks[0].instructions.is_empty());
    let remaining = &function.blocks[1].instructions;
    assert_eq!(remaining.len(), 2);
    let original = remaining[0].results[0];
    let shared_float = remaining[1].results[0];
    assert_eq!(
        remaining[1].kind,
        InstructionKind::Cast {
            value: original,
            to: TypeId::F32,
            behavior: CastBehavior::Infallible,
        }
    );
    assert_eq!(
        function.blocks[0].terminator.kind,
        TerminatorKind::Return(vec![shared_float, original, original])
    );
}
