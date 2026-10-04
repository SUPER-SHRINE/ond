use crate::source::{FileId, Span};

pub mod ast {
    use super::{FileId, Span};

    #[derive(Debug, Clone)]
    pub struct Project {
        pub packages: Vec<Package>,
    }

    #[derive(Debug, Clone)]
    pub struct Package {
        pub logical_path: String,
        pub files: Vec<File>,
    }

    #[derive(Debug, Clone)]
    pub struct File {
        pub file_id: FileId,
        pub span: Span,
        pub package: PackageClause,
        pub imports: Vec<ImportDecl>,
        pub decls: Vec<TopLevelDecl>,
    }

    #[derive(Debug, Clone)]
    pub struct PackageClause {
        pub name: String,
        pub span: Span,
    }

    #[derive(Debug, Clone)]
    pub struct ImportDecl {
        pub alias: Option<Ident>,
        pub path: String,
        pub span: Span,
    }

    #[derive(Debug, Clone)]
    pub enum TopLevelDecl {
        Const(ConstDecl),
        Var(VarDecl),
        Type(TypeDecl),
        Func(FuncDecl),
        Operator(OperatorDecl),
    }

    #[derive(Debug, Clone)]
    pub struct ConstDecl {
        pub specs: Vec<ConstSpec>,
        pub span: Span,
    }

    #[derive(Debug, Clone)]
    pub struct ConstSpec {
        pub names: Vec<Ident>,
        pub ty: Option<Type>,
        pub values: Vec<Expr>,
        pub span: Span,
    }

    #[derive(Debug, Clone)]
    pub struct VarDecl {
        pub specs: Vec<VarSpec>,
        pub span: Span,
    }

    #[derive(Debug, Clone)]
    pub struct VarSpec {
        pub names: Vec<Ident>,
        pub ty: Option<Type>,
        pub values: Vec<Expr>,
        pub span: Span,
    }

    #[derive(Debug, Clone)]
    pub struct TypeDecl {
        pub specs: Vec<TypeSpec>,
        pub span: Span,
    }

    #[derive(Debug, Clone)]
    pub struct TypeSpec {
        pub name: Ident,
        pub type_params: Vec<Ident>,
        pub ty: Type,
        pub span: Span,
    }

    #[derive(Debug, Clone)]
    pub struct FuncDecl {
        pub receiver: Option<Box<Field>>,
        pub name: Ident,
        pub type_params: Vec<Ident>,
        pub signature: Signature,
        pub body: Block,
        pub span: Span,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub enum OperatorName {
        Add,
        Sub,
        Mul,
        Div,
        Rem,
        Index,
        IndexSet,
        Len,
    }

    #[derive(Debug, Clone)]
    pub struct OperatorDecl {
        pub name: OperatorName,
        pub type_params: Vec<Ident>,
        pub signature: Signature,
        pub body: Block,
        pub span: Span,
    }

    #[derive(Debug, Clone)]
    pub struct Signature {
        pub params: Vec<Field>,
        pub results: Vec<Type>,
        pub span: Span,
    }

    /// Function types carry optional documentation names, not local bindings.
    #[derive(Debug, Clone)]
    pub struct TypeSignature {
        pub params: Vec<TypeParameter>,
        pub results: Vec<Type>,
        pub span: Span,
    }

    #[derive(Debug, Clone)]
    pub struct TypeParameter {
        pub name: Option<Ident>,
        pub ty: Type,
        pub span: Span,
    }

    #[derive(Debug, Clone)]
    pub struct Field {
        pub name: Ident,
        pub ty: Type,
        pub span: Span,
    }

    #[derive(Debug, Clone)]
    pub struct InterfaceMethod {
        pub name: Ident,
        pub signature: TypeSignature,
        pub span: Span,
    }

    #[derive(Debug, Clone)]
    pub struct Block {
        pub statements: Vec<Stmt>,
        pub span: Span,
    }

    #[derive(Debug, Clone)]
    pub enum Stmt {
        Const(ConstDecl),
        Var(VarDecl),
        ShortVar(ShortVarDecl),
        Assign(AssignStmt),
        Expr(ExprStmt),
        Return(ReturnStmt),
        Defer(DeferStmt),
        Trap { message: String, span: Span },
        Break(Span),
        Continue(Span),
        Block(Block),
        If(IfStmt),
        For(ForStmt),
    }

    #[derive(Debug, Clone)]
    pub struct ShortVarDecl {
        pub names: Vec<Ident>,
        pub values: Vec<Expr>,
        pub span: Span,
    }

    #[derive(Debug, Clone)]
    pub struct AssignStmt {
        pub targets: Vec<Expr>,
        pub values: Vec<Expr>,
        pub op: Option<BinaryOp>,
        pub implicit_one: bool,
        pub span: Span,
    }

    #[derive(Debug, Clone)]
    pub struct ExprStmt {
        pub expr: Expr,
        pub span: Span,
    }

    #[derive(Debug, Clone)]
    pub struct ReturnStmt {
        pub values: Vec<Expr>,
        pub span: Span,
    }

    #[derive(Debug, Clone)]
    pub struct DeferStmt {
        pub block: Block,
        pub span: Span,
    }

    #[derive(Debug, Clone)]
    pub struct IfStmt {
        pub init: Option<Box<Stmt>>,
        pub condition: Expr,
        pub then_block: Block,
        pub else_branch: Option<Box<Stmt>>,
        pub span: Span,
    }

    #[derive(Debug, Clone)]
    pub enum ForKind {
        Infinite,
        While(Expr),
        ThreeClause {
            init: Option<Box<Stmt>>,
            condition: Option<Expr>,
            post: Option<Box<Stmt>>,
        },
    }

    #[derive(Debug, Clone)]
    pub struct ForStmt {
        pub kind: ForKind,
        pub body: Block,
        pub span: Span,
    }

    #[derive(Debug, Clone)]
    pub struct Ident {
        pub name: String,
        pub span: Span,
    }

    #[derive(Debug, Clone)]
    pub enum Type {
        Named(Path),
        Apply {
            base: Path,
            arguments: Vec<Type>,
            span: Span,
        },
        /// Compiler-created type after generic substitution; never parsed from source.
        Resolved(crate::semantic::TypeId, Span),
        Pointer(Box<Type>, Span),
        Array {
            /// None denotes `[...]T`, valid only as a composite literal type.
            len: Option<Box<Expr>>,
            element: Box<Type>,
            span: Span,
        },
        Struct {
            fields: Vec<Field>,
            span: Span,
        },
        Interface {
            methods: Vec<InterfaceMethod>,
            span: Span,
        },
        Func {
            signature: TypeSignature,
            span: Span,
        },
    }

    #[derive(Debug, Clone)]
    pub struct Path {
        pub segments: Vec<Ident>,
        pub span: Span,
    }

    #[derive(Debug, Clone)]
    pub enum Literal {
        Int(String, Span),
        Float(String, Span),
        String(String, Span),
        Bool(bool, Span),
        Nil(Span),
    }

    #[derive(Debug, Clone)]
    pub struct CompositeElement {
        pub key: Option<Expr>,
        pub value: Expr,
        pub span: Span,
    }

    #[derive(Debug, Clone)]
    pub enum Expr {
        Literal(Literal),
        Name(Path),
        Unary {
            op: UnaryOp,
            expr: Box<Expr>,
            span: Span,
        },
        Binary {
            op: BinaryOp,
            lhs: Box<Expr>,
            rhs: Box<Expr>,
            span: Span,
        },
        Call {
            callee: Box<Expr>,
            args: Vec<Expr>,
            span: Span,
        },
        TypeApply {
            base: Box<Expr>,
            arguments: Vec<Type>,
            span: Span,
        },
        LayoutQuery {
            ty: Type,
            alignment: bool,
            span: Span,
        },
        Selector {
            base: Box<Expr>,
            field: Ident,
            span: Span,
        },
        Index {
            base: Box<Expr>,
            index: Box<Expr>,
            span: Span,
        },
        Cast {
            expr: Box<Expr>,
            ty: Type,
            span: Span,
        },
        Composite {
            ty: Type,
            elements: Vec<CompositeElement>,
            span: Span,
        },
        New {
            ty: Type,
            span: Span,
        },
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum UnaryOp {
        Plus,
        Minus,
        Not,
        BitNot,
        Deref,
        AddrOf,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum BinaryOp {
        Add,
        Sub,
        Mul,
        Div,
        Rem,
        BitAnd,
        BitOr,
        BitXor,
        BitAndNot,
        ShiftLeft,
        ShiftRight,
        Eq,
        Ne,
        Lt,
        Le,
        Gt,
        Ge,
        AndAnd,
        OrOr,
    }

    impl Literal {
        pub fn span(&self) -> Span {
            match self {
                Self::Int(_, span)
                | Self::Float(_, span)
                | Self::String(_, span)
                | Self::Bool(_, span)
                | Self::Nil(span) => *span,
            }
        }
    }

    impl Expr {
        pub fn span(&self) -> Span {
            match self {
                Self::Literal(literal) => literal.span(),
                Self::Name(path) => path.span,
                Self::Unary { span, .. }
                | Self::Binary { span, .. }
                | Self::Call { span, .. }
                | Self::TypeApply { span, .. }
                | Self::LayoutQuery { span, .. }
                | Self::Selector { span, .. }
                | Self::Index { span, .. }
                | Self::Cast { span, .. }
                | Self::Composite { span, .. }
                | Self::New { span, .. } => *span,
            }
        }
    }
}

pub mod hir {
    use std::collections::BTreeMap;

    use super::ast;
    use crate::project::PackageId;
    use crate::semantic::{FunctionType, TypeId, TypeTable};
    use crate::source::{FileId, Span};

    #[derive(Debug, Clone)]
    pub struct Project {
        pub types: TypeTable,
        pub symbols: Vec<Symbol>,
        pub packages: Vec<Package>,
        pub entry_package: PackageId,
        pub initialization_order: Vec<PackageId>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct SymbolId(pub u32);

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum SymbolKind {
        Constant,
        Global,
        Function,
        Type,
    }

    #[derive(Debug, Clone)]
    pub struct Symbol {
        pub id: SymbolId,
        pub package: PackageId,
        pub name: String,
        pub canonical_name: String,
        pub kind: SymbolKind,
        pub exported: bool,
        pub span: Span,
    }

    impl Project {
        /// Resolve a package-level name after lexical locals have been checked.
        pub fn resolve_symbol(
            &self,
            from: PackageId,
            file: FileId,
            qualifier: Option<&str>,
            name: &str,
        ) -> Result<&Symbol, String> {
            let owner = if let Some(qualifier) = qualifier {
                let current = self
                    .packages
                    .iter()
                    .find(|package| package.id == from)
                    .ok_or("unknown source package")?;
                current
                    .import_names
                    .get(&file)
                    .and_then(|imports| imports.get(qualifier))
                    .copied()
                    .ok_or_else(|| format!("unknown imported package `{qualifier}`"))?
            } else {
                from
            };
            let symbol = self
                .symbols
                .iter()
                .find(|symbol| symbol.package == owner && symbol.name == name)
                .ok_or_else(|| format!("unknown identifier `{name}`"))?;
            if qualifier.is_some() && !symbol.exported {
                return Err(format!("`{name}` is not exported"));
            }
            Ok(symbol)
        }
    }

    #[derive(Debug, Clone)]
    pub struct Package {
        pub id: PackageId,
        pub name: String,
        pub logical_path: String,
        pub imports: Vec<PackageId>,
        pub import_names: BTreeMap<FileId, BTreeMap<String, PackageId>>,
        pub files: Vec<File>,
        pub items: Vec<Item>,
        pub statics: Vec<Static>,
        pub initializer: Option<FuncItem>,
    }

    #[derive(Debug, Clone)]
    pub struct Static {
        pub symbol: String,
        pub value: Expr,
        pub exported: bool,
        /// The logical value excludes the trailing zero stored in read-only data.
        pub null_terminated: bool,
        pub span: Span,
    }

    #[derive(Debug, Clone)]
    pub struct File {
        pub file_id: crate::source::FileId,
        pub package_name: String,
    }

    #[derive(Debug, Clone)]
    pub enum Item {
        Const(ConstItem),
        Var(VarItem),
        Type(TypeItem),
        Func(FuncItem),
    }

    #[derive(Debug, Clone)]
    pub struct ConstItem {
        pub names: Vec<String>,
        pub values: Vec<Expr>,
        pub span: crate::source::Span,
    }

    #[derive(Debug, Clone)]
    pub struct VarItem {
        pub names: Vec<String>,
        /// One slot per declared value; None discards it without storage.
        pub globals: Vec<Option<Global>>,
        pub initializer: Option<ValueList>,
        pub span: crate::source::Span,
    }

    #[derive(Debug, Clone)]
    pub struct Global {
        pub symbol: String,
        pub ty: TypeId,
        pub initializer: Option<Expr>,
        pub span: Span,
    }

    #[derive(Debug, Clone)]
    pub struct TypeItem {
        pub name: String,
        pub ty: TypeId,
        pub span: crate::source::Span,
    }

    #[derive(Debug, Clone)]
    pub struct Parameter {
        pub name: String,
        pub ty: TypeId,
        pub span: crate::source::Span,
    }

    #[derive(Debug, Clone)]
    pub struct Signature {
        pub parameters: Vec<Parameter>,
        pub returns: Vec<TypeId>,
        pub span: crate::source::Span,
    }

    impl Signature {
        pub fn function_type(&self) -> FunctionType {
            FunctionType {
                parameters: self
                    .parameters
                    .iter()
                    .map(|parameter| parameter.ty)
                    .collect(),
                returns: self.returns.clone(),
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct FuncItem {
        pub name: String,
        /// Exact receiver type for methods, including pointer-ness.
        pub receiver: Option<TypeId>,
        pub signature: Signature,
        pub body: Body,
        pub span: crate::source::Span,
        pub canonical_name: String,
    }

    #[derive(Debug, Clone)]
    pub enum Body {
        Typed(TypedBody),
    }

    #[derive(Debug, Clone)]
    pub struct TypedBody {
        pub locals: Vec<Local>,
        pub block: Block,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct LocalId(pub u32);

    #[derive(Debug, Clone)]
    pub struct Local {
        pub id: LocalId,
        pub name: String,
        pub ty: TypeId,
        pub kind: LocalKind,
        pub span: Span,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum LocalKind {
        Parameter,
        User,
    }

    #[derive(Debug, Clone)]
    pub struct Block {
        pub statements: Vec<Stmt>,
        pub span: Span,
    }

    #[derive(Debug, Clone)]
    pub enum Stmt {
        LocalDecl {
            locals: Vec<Option<LocalId>>,
            values: ValueList,
            span: Span,
        },
        Assign {
            targets: Vec<Option<Expr>>,
            values: ValueList,
            op: Option<ast::BinaryOp>,
            overloaded: Option<Call>,
            span: Span,
        },
        IndexAssign(Box<IndexAssignStmt>),
        Expr(Expr),
        Call(Call),
        Return {
            values: ValueList,
            span: Span,
        },
        Defer {
            id: usize,
            block: Block,
            span: Span,
        },
        Trap {
            message: String,
            span: Span,
        },
        Block(Block),
        If {
            condition: Expr,
            then_block: Block,
            else_block: Block,
            span: Span,
        },
        For {
            condition: Option<Expr>,
            body: Block,
            post: Block,
            span: Span,
        },
        Break(Span),
        Continue(Span),
    }

    #[derive(Debug, Clone)]
    pub struct IndexAssignStmt {
        pub base: Expr,
        pub index: Expr,
        pub getter: Call,
        pub values: ValueList,
        pub op: ast::BinaryOp,
        pub overloaded: Option<Call>,
        pub setter: Call,
        pub span: Span,
    }

    #[derive(Debug, Clone)]
    pub struct Expr {
        pub kind: ExprKind,
        pub ty: TypeId,
        pub span: Span,
    }

    /// A multiple-result call is evaluated once, not once per destination.
    /// Multiple results are not a first-class tuple value.
    #[derive(Debug, Clone)]
    pub enum ValueList {
        Expressions(Vec<Expr>),
        Call(Call),
    }

    impl ValueList {
        pub fn types(&self) -> Vec<TypeId> {
            match self {
                Self::Expressions(values) => values.iter().map(|value| value.ty).collect(),
                Self::Call(call) => call.signature.returns.clone(),
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct Call {
        pub callee: Box<Expr>,
        pub signature: FunctionType,
        pub arguments: Vec<Expr>,
        pub span: Span,
    }

    #[derive(Debug, Clone)]
    pub enum ExprKind {
        Zero,
        Integer(u64),
        Float32(u32),
        Bool(bool),
        Nil,
        Local(LocalId),
        Global(String),
        Dereference(Box<Expr>),
        AddressOf(Box<Expr>),
        Field {
            base: Box<Expr>,
            index: u32,
        },
        Index {
            base: Box<Expr>,
            index: Box<Expr>,
        },
        Composite(Vec<(u32, Expr)>),
        Function(String),
        InterfaceMethod {
            receiver: Box<Expr>,
            index: u32,
        },
        InterfaceEqual {
            lhs: Box<Expr>,
            rhs: Box<Expr>,
            negate: bool,
        },
        EvaluatedLength {
            value: Box<Expr>,
            length: u32,
        },
        Intrinsic(crate::semantic::Intrinsic),
        Call(Box<Call>),
        Unary {
            op: ast::UnaryOp,
            operand: Box<Expr>,
        },
        Binary {
            op: ast::BinaryOp,
            lhs: Box<Expr>,
            rhs: Box<Expr>,
        },
        Cast {
            value: Box<Expr>,
        },
    }
}

pub mod mir {
    pub use crate::mir::*;
}
