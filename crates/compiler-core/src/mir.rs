mod display;
mod validate;
mod validate_flow;
mod validate_globals;
mod validate_guards;
mod validate_operations;
mod validate_types;

use crate::project::PackageId;
pub use crate::semantic::{FunctionType, TypeId, TypeKind, TypeTable};
use crate::source::Span;

pub use display::format_project;
pub use validate::{ValidationError, validate_function, validate_project};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub types: TypeTable,
    pub packages: Vec<Package>,
    /// Reserved for format compatibility. Ond source projects must leave this empty.
    pub initialization_order: Vec<PackageId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    pub id: PackageId,
    pub imports: Vec<PackageId>,
    pub logical_path: String,
    pub functions: Vec<Function>,
    pub globals: Vec<Global>,
    pub statics: Vec<Static>,
    pub initializer: Option<Function>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Global {
    pub symbol: String,
    pub ty: TypeId,
    pub initializer: Option<StaticValue>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Static {
    pub symbol: String,
    pub value: StaticValue,
    pub exported: bool,
    /// The logical value excludes the trailing zero stored in read-only data.
    pub null_terminated: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticValue {
    pub ty: TypeId,
    pub kind: StaticValueKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StaticValueKind {
    Zero,
    Integer(u64),
    Float32(u32),
    Bool(bool),
    Nil,
    Function {
        symbol: String,
        signature: FunctionType,
    },
    Composite(Vec<(u32, StaticValue)>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub name: String,
    pub parameters: Vec<LocalId>,
    pub returns: Vec<TypeId>,
    pub locals: Vec<Local>,
    pub values: Vec<Value>,
    pub blocks: Vec<BasicBlock>,
    pub entry: BlockId,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Local {
    pub id: LocalId,
    pub name: Option<String>,
    pub ty: TypeId,
    pub kind: LocalKind,
    pub address_taken: bool,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalKind {
    Parameter,
    User,
    Temporary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Value {
    pub id: ValueId,
    pub ty: TypeId,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicBlock {
    pub id: BlockId,
    pub parameters: Vec<ValueId>,
    pub instructions: Vec<Instruction>,
    pub terminator: Terminator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    pub id: InstructionId,
    pub results: Vec<ValueId>,
    pub kind: InstructionKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstructionKind {
    Constant(Constant),
    Unary {
        op: UnaryOp,
        operand: ValueId,
    },
    Binary {
        op: BinaryOp,
        lhs: ValueId,
        rhs: ValueId,
        overflow: OverflowBehavior,
    },
    Cast {
        value: ValueId,
        to: TypeId,
        behavior: CastBehavior,
    },
    AddressOf(Place),
    Load {
        place: Place,
        access: AccessKind,
    },
    Store {
        place: Place,
        value: ValueId,
        access: AccessKind,
    },
    AggregateCopy {
        destination: Place,
        source: Place,
        access: AccessKind,
    },
    Check(Check),
    Call {
        callee: Callee,
        signature: FunctionType,
        arguments: Vec<ValueId>,
        effects: CallEffects,
    },
}

impl InstructionKind {
    pub fn effects(&self) -> Effects {
        match self {
            Self::Constant(_) | Self::Unary { .. } | Self::AddressOf(_) => Effects::PURE,
            Self::Binary { overflow, .. } => Effects {
                memory: MemoryEffect::None,
                may_trap: matches!(overflow, OverflowBehavior::Checked),
                calls: false,
            },
            Self::Cast { behavior, .. } => Effects {
                memory: MemoryEffect::None,
                may_trap: matches!(behavior, CastBehavior::Checked),
                calls: false,
            },
            Self::Load { access, .. } => Effects {
                memory: MemoryEffect::Read,
                may_trap: matches!(access, AccessKind::Ordered),
                calls: false,
            },
            Self::Store { access, .. } => Effects {
                memory: MemoryEffect::Write,
                may_trap: matches!(access, AccessKind::Ordered),
                calls: false,
            },
            Self::AggregateCopy { access, .. } => Effects {
                memory: MemoryEffect::ReadWrite,
                may_trap: matches!(access, AccessKind::Ordered),
                calls: false,
            },
            Self::Check(_) => Effects {
                memory: MemoryEffect::None,
                may_trap: true,
                calls: false,
            },
            Self::Call { effects, .. } => effects.effects(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Terminator {
    pub kind: TerminatorKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminatorKind {
    Jump(Edge),
    Branch {
        condition: ValueId,
        then_edge: Edge,
        else_edge: Edge,
    },
    Return(Vec<ValueId>),
    Trap(TrapKind),
    Unreachable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    pub target: BlockId,
    pub arguments: Vec<ValueId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    pub base: PlaceBase,
    pub projections: Vec<Projection>,
    pub ty: TypeId,
}

impl Place {
    pub fn requires_ordered_access(&self) -> bool {
        matches!(self.base, PlaceBase::Pointer(_))
            || self
                .projections
                .iter()
                .any(|projection| matches!(projection, Projection::Dereference))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaceBase {
    Local(LocalId),
    Static { symbol: String, ty: TypeId },
    Pointer(ValueId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Projection {
    Dereference,
    Field {
        index: u32,
        ty: TypeId,
    },
    Index {
        index: ValueId,
        element: TypeId,
    },
    /// Unchecked element offset from a raw pointer, before field/array projections.
    Offset {
        index: ValueId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Check {
    Bounds { index: ValueId, length: ValueId },
    NonZero { value: ValueId },
    SignedDivisionOverflow { lhs: ValueId, rhs: ValueId },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Callee {
    Intrinsic(crate::semantic::Intrinsic),
    Direct(String),
    Indirect(ValueId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallEffects {
    Unknown,
    Pure,
    ReadOnly,
}

impl CallEffects {
    fn effects(self) -> Effects {
        match self {
            Self::Unknown => Effects {
                memory: MemoryEffect::ReadWrite,
                may_trap: true,
                calls: true,
            },
            Self::Pure => Effects {
                memory: MemoryEffect::None,
                may_trap: false,
                calls: true,
            },
            Self::ReadOnly => Effects {
                memory: MemoryEffect::Read,
                may_trap: true,
                calls: true,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessKind {
    Local,
    Ordered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Effects {
    pub memory: MemoryEffect,
    pub may_trap: bool,
    pub calls: bool,
}

impl Effects {
    pub const PURE: Self = Self {
        memory: MemoryEffect::None,
        may_trap: false,
        calls: false,
    };

    pub fn is_pure(self) -> bool {
        self == Self::PURE
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryEffect {
    None,
    Read,
    Write,
    ReadWrite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverflowBehavior {
    Wrap,
    Checked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CastBehavior {
    Infallible,
    Checked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Negate,
    BitNot,
    LogicalNot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    BitAnd,
    BitOr,
    BitXor,
    BitAndNot,
    ShiftLeft,
    ShiftRightLogical,
    ShiftRightArithmetic,
    Equal,
    NotEqual,
    LessSigned,
    LessUnsigned,
    LessEqualSigned,
    LessEqualUnsigned,
    GreaterSigned,
    GreaterUnsigned,
    GreaterEqualSigned,
    GreaterEqualUnsigned,
    FloatAdd,
    FloatSubtract,
    FloatMultiply,
    FloatDivide,
    FloatEqual,
    FloatNotEqual,
    FloatLess,
    FloatLessEqual,
    FloatGreater,
    FloatGreaterEqual,
}

impl BinaryOp {
    pub fn is_comparison(self) -> bool {
        matches!(
            self,
            Self::Equal
                | Self::NotEqual
                | Self::LessSigned
                | Self::LessUnsigned
                | Self::LessEqualSigned
                | Self::LessEqualUnsigned
                | Self::GreaterSigned
                | Self::GreaterUnsigned
                | Self::GreaterEqualSigned
                | Self::GreaterEqualUnsigned
                | Self::FloatEqual
                | Self::FloatNotEqual
                | Self::FloatLess
                | Self::FloatLessEqual
                | Self::FloatGreater
                | Self::FloatGreaterEqual
        )
    }

    pub fn is_shift(self) -> bool {
        matches!(
            self,
            Self::ShiftLeft | Self::ShiftRightLogical | Self::ShiftRightArithmetic
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Constant {
    Zero(TypeId),
    Integer {
        bits: u64,
        ty: TypeId,
    },
    Float32 {
        bits: u32,
        ty: TypeId,
    },
    Bool {
        value: bool,
        ty: TypeId,
    },
    Nil(TypeId),
    Function {
        symbol: String,
        signature: FunctionType,
        ty: TypeId,
    },
}

impl Constant {
    pub fn ty(&self) -> TypeId {
        match self {
            Self::Zero(ty)
            | Self::Integer { ty, .. }
            | Self::Float32 { ty, .. }
            | Self::Bool { ty, .. }
            | Self::Nil(ty)
            | Self::Function { ty, .. } => *ty,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocalId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ValueId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstructionId(pub u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrapKind {
    Bounds,
    DivisionByZero,
    SignedDivisionOverflow,
    InvalidConversion,
    StackOverflow,
    Explicit(String),
}
