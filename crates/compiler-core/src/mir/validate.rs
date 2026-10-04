use std::collections::BTreeSet;

use super::*;
#[path = "validate_memory.rs"]
mod memory;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    pub function: String,
    pub block: Option<BlockId>,
    pub message: String,
}

pub fn validate_project(project: &Project) -> Result<(), Vec<ValidationError>> {
    let mut errors = validate_type_table(&project.types);
    errors.extend(super::validate_types::validate(project));
    errors.extend(super::validate_globals::validate(project));
    errors.extend(super::validate_operations::project(project));
    for package in &project.packages {
        for function in package.functions.iter().chain(package.initializer.iter()) {
            if let Err(mut function_errors) = validate_function(&project.types, function) {
                errors.append(&mut function_errors);
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_type_table(types: &TypeTable) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    fn check(
        types: &TypeTable,
        errors: &mut Vec<ValidationError>,
        owner: TypeId,
        referenced: TypeId,
        role: &str,
    ) {
        if types.get(referenced).is_none() {
            errors.push(ValidationError {
                function: "<type-table>".to_string(),
                block: None,
                message: format!("{owner} {role} references missing {referenced}"),
            });
        }
    }
    for (index, definition) in types.definitions().iter().enumerate() {
        if definition.id != TypeId(index as u32) {
            errors.push(ValidationError {
                function: "<type-table>".to_string(),
                block: None,
                message: format!("type index {index} has id {:?}", definition.id),
            });
        }
        match &definition.kind {
            TypeKind::Pending => errors.push(ValidationError {
                function: "<type-table>".to_string(),
                block: None,
                message: format!("{} is still pending", definition.id),
            }),
            TypeKind::Pointer(pointee) => {
                check(types, &mut errors, definition.id, *pointee, "pointee")
            }
            TypeKind::Function(signature) => {
                for parameter in &signature.parameters {
                    check(types, &mut errors, definition.id, *parameter, "parameter");
                }
                for result in &signature.returns {
                    check(types, &mut errors, definition.id, *result, "return");
                }
            }
            TypeKind::Array { element, .. } => {
                check(types, &mut errors, definition.id, *element, "element")
            }
            TypeKind::Struct(structure) => {
                for field in &structure.fields {
                    check(types, &mut errors, definition.id, field.ty, "field");
                }
            }
            TypeKind::Interface(interface) => {
                check(
                    types,
                    &mut errors,
                    definition.id,
                    interface.data_pointer,
                    "interface data pointer",
                );
                check(
                    types,
                    &mut errors,
                    definition.id,
                    interface.vtable,
                    "interface vtable",
                );
                for method in &interface.methods {
                    for parameter in &method.signature.parameters {
                        check(
                            types,
                            &mut errors,
                            definition.id,
                            *parameter,
                            "interface method parameter",
                        );
                    }
                    for result in &method.signature.returns {
                        check(
                            types,
                            &mut errors,
                            definition.id,
                            *result,
                            "interface method return",
                        );
                    }
                }
            }
            TypeKind::Defined { underlying } => check(
                types,
                &mut errors,
                definition.id,
                *underlying,
                "underlying type",
            ),
            TypeKind::Bool
            | TypeKind::U8
            | TypeKind::I8
            | TypeKind::U16
            | TypeKind::I16
            | TypeKind::U32
            | TypeKind::I32
            | TypeKind::F32 => {}
        }
    }
    errors
}

pub fn validate_function(
    types: &TypeTable,
    function: &Function,
) -> Result<(), Vec<ValidationError>> {
    let mut validator = Validator {
        types,
        function,
        errors: Vec::new(),
        defined_values: BTreeSet::new(),
        instruction_ids: BTreeSet::new(),
    };
    validator.validate();
    validator
        .errors
        .extend(super::validate_flow::validate(types, function));
    validator
        .errors
        .extend(super::validate_operations::function(types, function));
    if validator.errors.is_empty() {
        Ok(())
    } else {
        Err(validator.errors)
    }
}

struct Validator<'a> {
    types: &'a TypeTable,
    function: &'a Function,
    errors: Vec<ValidationError>,
    defined_values: BTreeSet<ValueId>,
    instruction_ids: BTreeSet<InstructionId>,
}

impl Validator<'_> {
    fn validate(&mut self) {
        if self.block(self.function.entry).is_none() {
            self.error(
                None,
                format!("entry block {:?} does not exist", self.function.entry),
            );
        }

        for (index, local) in self.function.locals.iter().enumerate() {
            if local.id != LocalId(index as u32) {
                self.error(None, format!("local index {index} has id {:?}", local.id));
            }
            self.validate_type_id(None, local.ty, "local");
        }
        for parameter in &self.function.parameters {
            match self.local(*parameter) {
                Some(local) if local.kind == LocalKind::Parameter => {}
                Some(_) => self.error(
                    None,
                    format!("parameter {parameter:?} is not a parameter local"),
                ),
                None => self.error(None, format!("parameter {parameter:?} does not exist")),
            }
        }
        for (index, value) in self.function.values.iter().enumerate() {
            if value.id != ValueId(index as u32) {
                self.error(None, format!("value index {index} has id {:?}", value.id));
            }
            self.validate_type_id(None, value.ty, "value");
        }
        for ty in &self.function.returns {
            self.validate_type_id(None, *ty, "return type");
        }
        for (index, block) in self.function.blocks.iter().enumerate() {
            if block.id != BlockId(index as u32) {
                self.error(
                    Some(block.id),
                    format!("block index {index} has id {:?}", block.id),
                );
            }
            for value in &block.parameters {
                self.define_value(Some(block.id), *value, "block parameter");
            }
            for instruction in &block.instructions {
                if !self.instruction_ids.insert(instruction.id) {
                    self.error(
                        Some(block.id),
                        format!("instruction {:?} is defined more than once", instruction.id),
                    );
                }
                self.validate_instruction(block.id, instruction);
                for value in &instruction.results {
                    self.define_value(Some(block.id), *value, "instruction result");
                }
            }
            self.validate_terminator(block.id, &block.terminator);
        }

        for value in &self.function.values {
            if !self.defined_values.contains(&value.id) {
                self.error(None, format!("value {:?} has no definition", value.id));
            }
        }

        let instruction_ids = self.instruction_ids.iter().copied().collect::<Vec<_>>();
        for (expected, actual) in instruction_ids.into_iter().enumerate() {
            if actual.0 != expected as u32 {
                self.error(
                    None,
                    format!(
                        "instruction ids are not dense: expected InstructionId({expected}), found {actual:?}"
                    ),
                );
            }
        }
    }

    fn validate_instruction(&mut self, block: BlockId, instruction: &Instruction) {
        let result_count = instruction.results.len();
        match &instruction.kind {
            InstructionKind::Constant(constant) => {
                self.expect_results(block, instruction, 1);
                self.expect_result_type(block, instruction, 0, &constant.ty());
            }
            InstructionKind::Unary { operand, .. } => {
                self.expect_results(block, instruction, 1);
                if let Some(ty) = self.value_type(block, *operand) {
                    self.expect_result_type(block, instruction, 0, &ty);
                }
            }
            InstructionKind::Binary { op, lhs, rhs, .. } => {
                self.expect_results(block, instruction, 1);
                let lhs_ty = self.value_type(block, *lhs);
                let rhs_ty = self.value_type(block, *rhs);
                if let (Some(lhs_ty), Some(rhs_ty)) = (&lhs_ty, &rhs_ty) {
                    if op.is_shift() {
                        if *rhs_ty != TypeId::U32 {
                            self.error(
                                Some(block),
                                format!("shift count has type {rhs_ty}, expected u32"),
                            );
                        }
                    } else if lhs_ty != rhs_ty {
                        self.error(
                            Some(block),
                            format!("binary operands have types {lhs_ty} and {rhs_ty}"),
                        );
                    }
                    let result_ty = if op.is_comparison() {
                        TypeId::BOOL
                    } else {
                        lhs_ty.clone()
                    };
                    self.expect_result_type(block, instruction, 0, &result_ty);
                }
            }
            InstructionKind::Cast { value, to, .. } => {
                self.expect_results(block, instruction, 1);
                self.value_type(block, *value);
                self.expect_result_type(block, instruction, 0, to);
            }
            InstructionKind::AddressOf(place) => {
                self.expect_results(block, instruction, 1);
                self.validate_place(block, place);
                if let Some(result) = instruction.results.first() {
                    if let Some(result_ty) = self.value_type(block, *result) {
                        if !matches!(
                            self.types.kind(result_ty),
                            Some(TypeKind::Pointer(pointee)) if *pointee == place.ty
                        ) {
                            self.error(
                                Some(block),
                                "address-of result is not a pointer to the place type".to_string(),
                            );
                        }
                    }
                }
            }
            InstructionKind::Load { place, access } => {
                self.expect_results(block, instruction, 1);
                self.validate_place(block, place);
                self.validate_access_kind(block, place, *access);
                self.expect_result_type(block, instruction, 0, &place.ty);
            }
            InstructionKind::Store {
                place,
                value,
                access,
            } => {
                self.expect_results(block, instruction, 0);
                self.validate_place(block, place);
                self.validate_access_kind(block, place, *access);
                if let Some(value_ty) = self.value_type(block, *value) {
                    if value_ty != place.ty {
                        self.error(
                            Some(block),
                            format!(
                                "store value has type {value_ty}, place has type {}",
                                place.ty
                            ),
                        );
                    }
                }
            }
            InstructionKind::AggregateCopy {
                destination,
                source,
                access,
            } => {
                self.expect_results(block, instruction, 0);
                self.validate_place(block, destination);
                self.validate_place(block, source);
                if (destination.requires_ordered_access() || source.requires_ordered_access())
                    && *access != AccessKind::Ordered
                {
                    self.error(
                        Some(block),
                        "aggregate copy through a pointer must be ordered".to_string(),
                    );
                }
                if destination.ty != source.ty {
                    self.error(
                        Some(block),
                        format!(
                            "aggregate copy types differ: {} and {}",
                            destination.ty, source.ty
                        ),
                    );
                }
            }
            InstructionKind::Check(check) => {
                self.expect_results(block, instruction, 0);
                match check {
                    Check::Bounds { index, length } => {
                        self.expect_integer(block, *index, "bounds index");
                        self.expect_integer(block, *length, "bounds length");
                    }
                    Check::NonZero { value } => {
                        self.expect_integer(block, *value, "non-zero check")
                    }
                    Check::SignedDivisionOverflow { lhs, rhs } => {
                        self.expect_integer(block, *lhs, "division lhs");
                        self.expect_integer(block, *rhs, "division rhs");
                    }
                }
            }
            InstructionKind::Call {
                callee,
                signature,
                arguments,
                ..
            } => {
                if let Callee::Indirect(value) = callee {
                    if let Some(ty) = self.value_type(block, *value) {
                        if !matches!(
                            self.types.underlying_kind(ty),
                            Some(TypeKind::Function(actual)) if actual == signature
                        ) {
                            self.error(
                                Some(block),
                                format!(
                                    "indirect callee has type {ty}, expected function type {signature:?}"
                                ),
                            );
                        }
                    }
                }
                if arguments.len() != signature.parameters.len() {
                    self.error(
                        Some(block),
                        format!(
                            "call has {} arguments, expected {}",
                            arguments.len(),
                            signature.parameters.len()
                        ),
                    );
                }
                for (index, value) in arguments.iter().enumerate() {
                    let actual = self.value_type(block, *value);
                    let expected = signature.parameters.get(index);
                    if let (Some(actual), Some(expected)) = (actual, expected) {
                        if &actual != expected {
                            self.error(
                                Some(block),
                                format!(
                                    "call argument {index} has type {actual}, expected {expected}"
                                ),
                            );
                        }
                    }
                }
                if result_count != signature.returns.len() {
                    self.error(
                        Some(block),
                        format!(
                            "call has {result_count} results, expected {}",
                            signature.returns.len()
                        ),
                    );
                }
                for (index, expected) in signature.returns.iter().enumerate() {
                    self.expect_result_type(block, instruction, index, expected);
                }
            }
        }
    }

    fn validate_terminator(&mut self, block: BlockId, terminator: &Terminator) {
        match &terminator.kind {
            TerminatorKind::Jump(edge) => self.validate_edge(block, edge),
            TerminatorKind::Branch {
                condition,
                then_edge,
                else_edge,
            } => {
                if let Some(ty) = self.value_type(block, *condition) {
                    if !matches!(self.types.underlying_kind(ty), Some(TypeKind::Bool)) {
                        self.error(
                            Some(block),
                            format!("branch condition has type {ty}, expected bool"),
                        );
                    }
                }
                self.validate_edge(block, then_edge);
                self.validate_edge(block, else_edge);
            }
            TerminatorKind::Return(values) => {
                if values.len() != self.function.returns.len() {
                    self.error(
                        Some(block),
                        format!(
                            "return has {} values, expected {}",
                            values.len(),
                            self.function.returns.len()
                        ),
                    );
                }
                for (index, value) in values.iter().enumerate() {
                    let expected = self.function.returns.get(index).cloned();
                    if let (Some(actual), Some(expected)) =
                        (self.value_type(block, *value), expected)
                    {
                        if actual != expected {
                            self.error(
                                Some(block),
                                format!(
                                    "return value {index} has type {actual}, expected {expected}"
                                ),
                            );
                        }
                    }
                }
            }
            TerminatorKind::Trap(_) | TerminatorKind::Unreachable => {}
        }
    }

    fn validate_edge(&mut self, from: BlockId, edge: &Edge) {
        let Some(target) = self.block(edge.target) else {
            self.error(
                Some(from),
                format!("edge target {:?} does not exist", edge.target),
            );
            return;
        };
        let parameters = target.parameters.clone();
        if edge.arguments.len() != parameters.len() {
            self.error(
                Some(from),
                format!(
                    "edge to {:?} has {} arguments, expected {}",
                    edge.target,
                    edge.arguments.len(),
                    parameters.len()
                ),
            );
        }
        for (argument, parameter) in edge.arguments.iter().zip(parameters) {
            let argument_ty = self.value_type(from, *argument);
            let parameter_ty = self.value_type(from, parameter);
            if let (Some(argument_ty), Some(parameter_ty)) = (argument_ty, parameter_ty) {
                if argument_ty != parameter_ty {
                    self.error(Some(from), format!("edge argument has type {argument_ty}, parameter has type {parameter_ty}"));
                }
            }
        }
    }

    fn validate_access_kind(&mut self, block: BlockId, place: &Place, access: AccessKind) {
        if place.requires_ordered_access() && access != AccessKind::Ordered {
            self.error(
                Some(block),
                "access through a pointer must be ordered".to_string(),
            );
        }
    }

    fn expect_results(&mut self, block: BlockId, instruction: &Instruction, count: usize) {
        if instruction.results.len() != count {
            self.error(
                Some(block),
                format!(
                    "instruction {:?} has {} results, expected {count}",
                    instruction.id,
                    instruction.results.len()
                ),
            );
        }
    }

    fn expect_result_type(
        &mut self,
        block: BlockId,
        instruction: &Instruction,
        index: usize,
        expected: &TypeId,
    ) {
        if let Some(actual) = self.result_type(block, instruction, index) {
            if actual != *expected {
                self.error(
                    Some(block),
                    format!(
                        "instruction {:?} result {index} has type {actual}, expected {expected}",
                        instruction.id
                    ),
                );
            }
        }
    }

    fn result_type(
        &mut self,
        block: BlockId,
        instruction: &Instruction,
        index: usize,
    ) -> Option<TypeId> {
        let value = instruction.results.get(index).copied()?;
        self.value_type(block, value)
    }

    fn expect_integer(&mut self, block: BlockId, value: ValueId, role: &str) {
        if let Some(ty) = self.value_type(block, value) {
            if !self.types.is_integer(ty) {
                self.error(Some(block), format!("{role} has non-integer type {ty}"));
            }
        }
    }

    fn define_value(&mut self, block: Option<BlockId>, value: ValueId, role: &str) {
        if self.function.values.get(value.0 as usize).is_none() {
            self.error(block, format!("{role} {value:?} does not exist"));
        } else if !self.defined_values.insert(value) {
            self.error(block, format!("value {value:?} is defined more than once"));
        }
    }

    fn value_type(&mut self, block: BlockId, value: ValueId) -> Option<TypeId> {
        match self.function.values.get(value.0 as usize) {
            Some(value) => Some(value.ty),
            None => {
                self.error(Some(block), format!("value {value:?} does not exist"));
                None
            }
        }
    }

    fn local(&self, id: LocalId) -> Option<&Local> {
        self.function.locals.get(id.0 as usize)
    }

    fn block(&self, id: BlockId) -> Option<&BasicBlock> {
        self.function.blocks.get(id.0 as usize)
    }

    fn error(&mut self, block: Option<BlockId>, message: String) {
        self.errors.push(ValidationError {
            function: self.function.name.clone(),
            block,
            message,
        });
    }

    fn validate_type_id(&mut self, block: Option<BlockId>, ty: TypeId, role: &str) {
        if self.types.get(ty).is_none() {
            self.error(block, format!("{role} references missing {ty}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_function() -> Function {
        Function {
            name: "main.main".to_string(),
            parameters: Vec::new(),
            returns: Vec::new(),
            locals: Vec::new(),
            values: Vec::new(),
            blocks: vec![BasicBlock {
                id: BlockId(0),
                parameters: Vec::new(),
                instructions: Vec::new(),
                terminator: Terminator {
                    kind: TerminatorKind::Return(Vec::new()),
                    span: Span::synthetic(),
                },
            }],
            entry: BlockId(0),
            span: Span::synthetic(),
        }
    }

    #[test]
    fn accepts_well_formed_empty_function() {
        assert_eq!(
            validate_function(&TypeTable::new(), &empty_function()),
            Ok(())
        );
    }

    #[test]
    fn rejects_non_boolean_branch_condition() {
        let mut function = empty_function();
        function.values.push(Value {
            id: ValueId(0),
            ty: TypeId::U32,
            span: Span::synthetic(),
        });
        function.blocks[0].instructions.push(Instruction {
            id: InstructionId(0),
            results: vec![ValueId(0)],
            kind: InstructionKind::Constant(Constant::Integer {
                bits: 1,
                ty: TypeId::U32,
            }),
            span: Span::synthetic(),
        });
        function.blocks[0].terminator.kind = TerminatorKind::Branch {
            condition: ValueId(0),
            then_edge: Edge {
                target: BlockId(0),
                arguments: Vec::new(),
            },
            else_edge: Edge {
                target: BlockId(0),
                arguments: Vec::new(),
            },
        };

        let errors = validate_function(&TypeTable::new(), &function).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.message.contains("expected bool"))
        );
    }
}
