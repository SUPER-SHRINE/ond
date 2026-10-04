//! Test-only interpreter for by-value aggregates and aliased memory.
use crate::mir::*;
type Globals = std::collections::BTreeMap<String, Rc<RefCell<Value>>>;
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Debug)]
enum Value {
    Number(u64),
    Aggregate(Vec<Value>),
    Pointer(Rc<RefCell<Value>>, Vec<usize>),
}

impl Value {
    fn number(&self) -> u64 {
        match self {
            Self::Number(value) => *value,
            _ => panic!("not a number: {self:?}"),
        }
    }
    fn at(&self, path: &[usize]) -> Value {
        if path.is_empty() {
            return self.clone();
        }
        match self {
            Self::Aggregate(values) => values[path[0]].at(&path[1..]),
            _ => panic!("invalid memory path"),
        }
    }
    fn set(&mut self, path: &[usize], value: Value) {
        if path.is_empty() {
            *self = value;
            return;
        }
        match self {
            Self::Aggregate(values) => values[path[0]].set(&path[1..], value),
            _ => panic!("invalid memory path"),
        }
    }
}

fn zero(types: &TypeTable, ty: TypeId) -> Value {
    match types.underlying_kind(ty).unwrap() {
        TypeKind::Array { length, element } => {
            assert!(*length < 1000);
            Value::Aggregate((0..*length).map(|_| zero(types, *element)).collect())
        }
        TypeKind::Struct(structure) => Value::Aggregate(
            structure
                .fields
                .iter()
                .map(|field| zero(types, field.ty))
                .collect(),
        ),
        TypeKind::Interface(interface) => Value::Aggregate(vec![
            zero(types, interface.data_pointer),
            zero(types, interface.vtable),
        ]),
        _ => Value::Number(0),
    }
}

fn static_value(types: &TypeTable, value: &StaticValue) -> Value {
    match &value.kind {
        StaticValueKind::Zero | StaticValueKind::Nil => zero(types, value.ty),
        StaticValueKind::Integer(bits) => Value::Number(*bits),
        StaticValueKind::Float32(bits) => Value::Number(u64::from(*bits)),
        StaticValueKind::Bool(value) => Value::Number(u64::from(*value)),
        StaticValueKind::Function { .. } => Value::Number(1),
        StaticValueKind::Composite(items) => {
            let mut result = zero(types, value.ty);
            for (index, item) in items {
                result.set(&[*index as usize], static_value(types, item));
            }
            result
        }
    }
}

fn address(
    place: &Place,
    locals: &[Rc<RefCell<Value>>],
    globals: &Globals,
    values: &[Value],
) -> (Rc<RefCell<Value>>, Vec<usize>) {
    let (cell, mut path) = match &place.base {
        PlaceBase::Local(id) => (locals[id.0 as usize].clone(), Vec::new()),
        PlaceBase::Pointer(id) => match &values[id.0 as usize] {
            Value::Pointer(cell, path) => (cell.clone(), path.clone()),
            _ => panic!("not a pointer"),
        },
        PlaceBase::Static { symbol, .. } => (globals[symbol].clone(), Vec::new()),
    };
    for projection in &place.projections {
        match projection {
            Projection::Field { index, .. } => path.push(*index as usize),
            Projection::Index { index, .. } => {
                path.push(values[index.0 as usize].number() as usize)
            }
            Projection::Offset { index } => {
                let offset = values[index.0 as usize].number() as usize;
                if let Some(last) = path.last_mut() {
                    *last += offset;
                } else {
                    assert_eq!(offset, 0);
                }
            }
            _ => panic!("unexpected dereference projection"),
        }
    }
    (cell, path)
}

pub(super) fn execute(
    project: &Project,
    name: &str,
    input: &[u64],
) -> Result<Vec<u64>, &'static str> {
    let args = input
        .iter()
        .map(|value| Value::Number(*value))
        .collect::<Vec<_>>();
    let globals: Globals = project
        .packages
        .iter()
        .flat_map(|package| {
            package
                .globals
                .iter()
                .map(|global| {
                    (
                        global.symbol.clone(),
                        Rc::new(RefCell::new(
                            global
                                .initializer
                                .as_ref()
                                .map(|value| static_value(&project.types, value))
                                .unwrap_or_else(|| zero(&project.types, global.ty)),
                        )),
                    )
                })
                .chain(package.statics.iter().map(|item| {
                    (
                        item.symbol.clone(),
                        Rc::new(RefCell::new(static_value(&project.types, &item.value))),
                    )
                }))
        })
        .collect();
    let mut budget = 10000;
    for id in &project.initialization_order {
        let package = project.packages.iter().find(|p| p.id == *id).unwrap();
        if let Some(init) = &package.initializer {
            run(project, &init.name, Vec::new(), &globals, &mut budget)?;
        }
    }
    run(project, name, args, &globals, &mut budget)
        .map(|values| values.iter().map(Value::number).collect())
}

fn run(
    project: &Project,
    name: &str,
    input: Vec<Value>,
    globals: &Globals,
    budget: &mut usize,
) -> Result<Vec<Value>, &'static str> {
    let function = project
        .packages
        .iter()
        .flat_map(|p| p.functions.iter().chain(p.initializer.iter()))
        .find(|f| f.name == name)
        .unwrap();
    let locals = function
        .locals
        .iter()
        .map(|local| Rc::new(RefCell::new(zero(&project.types, local.ty))))
        .collect::<Vec<_>>();
    for (id, value) in function.parameters.iter().zip(input) {
        *locals[id.0 as usize].borrow_mut() = value;
    }
    let mut values = vec![Value::Number(0); function.values.len()];
    let mut block = function.entry;
    loop {
        if *budget == 0 {
            return Err("step limit");
        }
        *budget -= 1;
        let current = &function.blocks[block.0 as usize];
        for instruction in &current.instructions {
            let results = match &instruction.kind {
                InstructionKind::Constant(Constant::Zero(ty)) => vec![zero(&project.types, *ty)],
                InstructionKind::Constant(Constant::Integer { bits, .. }) => {
                    vec![Value::Number(*bits)]
                }
                InstructionKind::Constant(Constant::Bool { value, .. }) => {
                    vec![Value::Number(u64::from(*value))]
                }
                InstructionKind::Constant(Constant::Nil(_)) => vec![Value::Number(0)],
                InstructionKind::AddressOf(place) => {
                    let (cell, path) = address(place, &locals, globals, &values);
                    vec![Value::Pointer(cell, path)]
                }
                InstructionKind::Load { place, .. } => {
                    let (cell, path) = address(place, &locals, globals, &values);
                    let value = cell.borrow().at(&path);
                    vec![value]
                }
                InstructionKind::Store { place, value, .. } => {
                    let (cell, path) = address(place, &locals, globals, &values);
                    cell.borrow_mut()
                        .set(&path, values[value.0 as usize].clone());
                    vec![]
                }
                InstructionKind::AggregateCopy {
                    destination,
                    source,
                    ..
                } => {
                    let (src, path) = address(source, &locals, globals, &values);
                    let value = src.borrow().at(&path);
                    let (dst, path) = address(destination, &locals, globals, &values);
                    dst.borrow_mut().set(&path, value);
                    vec![]
                }
                InstructionKind::Check(Check::Bounds { index, length }) => {
                    if values[index.0 as usize].number() >= values[length.0 as usize].number() {
                        return Err("bounds");
                    }
                    vec![]
                }
                InstructionKind::Check(Check::NonZero { value }) => {
                    if values[value.0 as usize].number() == 0 {
                        return Err("division by zero");
                    }
                    vec![]
                }
                InstructionKind::Cast { value, .. } => vec![values[value.0 as usize].clone()],
                InstructionKind::Binary { op, lhs, rhs, .. } => {
                    let a = values[lhs.0 as usize].number();
                    let b = values[rhs.0 as usize].number();
                    vec![Value::Number(match op {
                        BinaryOp::Add => a + b,
                        BinaryOp::Multiply => a * b,
                        BinaryOp::Equal => u64::from(a == b),
                        BinaryOp::LessSigned => u64::from((a as i64) < (b as i64)),
                        _ => panic!("unsupported test binary"),
                    })]
                }
                InstructionKind::Call {
                    callee: Callee::Direct(name),
                    arguments,
                    ..
                } => {
                    let args = arguments
                        .iter()
                        .map(|id| values[id.0 as usize].clone())
                        .collect();
                    run(project, name, args, globals, budget)?
                }
                other => panic!("unsupported test instruction: {other:?}"),
            };
            assert_eq!(results.len(), instruction.results.len());
            for (id, value) in instruction.results.iter().zip(results) {
                values[id.0 as usize] = value;
            }
        }
        let edge = match &current.terminator.kind {
            TerminatorKind::Return(result) => {
                return Ok(result
                    .iter()
                    .map(|id| values[id.0 as usize].clone())
                    .collect());
            }
            TerminatorKind::Jump(edge) => edge,
            TerminatorKind::Branch {
                condition,
                then_edge,
                else_edge,
            } => {
                if values[condition.0 as usize].number() != 0 {
                    then_edge
                } else {
                    else_edge
                }
            }
            _ => return Err("unexpected terminator"),
        };
        let args = edge
            .arguments
            .iter()
            .map(|id| values[id.0 as usize].clone())
            .collect::<Vec<_>>();
        for (id, value) in function.blocks[edge.target.0 as usize]
            .parameters
            .iter()
            .zip(args)
        {
            values[id.0 as usize] = value;
        }
        block = edge.target;
    }
}
