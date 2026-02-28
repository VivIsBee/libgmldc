//! The output AST.
#![allow(missing_docs)]

#[derive(Debug, Clone)]
pub struct Block(pub Vec<Statement>);

#[derive(Debug, Clone)]
pub enum Statement {
    Empty,
    Block(Block),
    Enum {
        name: String,
        variants: Vec<(String, Option<Expr>)>,
    },
    Function {
        name: String,
        is_constructor: bool,
        inherit: Option<Call>,
        params: Vec<Param>,
    },
    Var(Vec<(String, Option<Expr>)>),
    Static(Vec<(String, Option<Expr>)>),
    GlobalVar(String),
    Assignment {
        target: MutableExpr,
        op: AssignmentOp,
        value: Box<Expr>,
    },
    Return(Option<Box<Expr>>),
    If {
        cond: Box<Expr>,
        then: Box<Statement>,
        r#else: Option<Box<Statement>>,
    },
    For {
        initializer: Box<Statement>,
        condition: Box<Expr>,
        iterator: Box<Statement>,
        body: Box<Statement>,
    },
    While(LoopStmt),
    Repeat(LoopStmt),
    Switch {
        target: Box<Expr>,
        cases: Vec<SwitchCase>,
        default: Option<Block>,
    },
    With(LoopStmt),
    TryCatch {
        try_block: Box<Statement>,
        err: String,
        catch_block: Box<Statement>,
    },
    Throw(Box<Expr>),
    Call(Call),
    Prefix(Mutation),
    Postfix(Mutation),
    Break,
    Continue,
}

#[derive(Debug, Clone)]
pub struct Call {
    pub base: Box<Expr>,
    pub arguments: Vec<Expr>,
    pub has_new: bool,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub default: Expr,
}

#[derive(Debug, Clone)]
pub struct LoopStmt {
    target: Box<Expr>,
    body: Box<Statement>,
}

#[derive(Debug, Clone)]
pub struct SwitchCase {
    pub compare: Expr,
    pub body: Block,
}

#[derive(Debug, Clone)]
pub struct Mutation {
    pub op: MutationOp,
    pub target: Box<MutableExpr>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum MutationOp {
    Increment,
    Decrement,
}

#[derive(Debug, Clone)]
pub enum MutableExpr {
    Ident(String),
    Field {
        base: Box<Expr>,
        field: String,
    },
    Index {
        base: Box<Expr>,
        accessor_type: Option<AccessorType>,
        indexes: Vec<Expr>,
    },
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum AccessorType {
    List,
    Map,
    Grid,
    Array,
    Struct,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum AssignmentOp {
    Equal,
    PlusEqual,
    MinusEqual,
    MultEqual,
    DivEqual,
    RemEqual,
    BitAndEqual,
    BitOrEqual,
    BitXorEqual,
    NullCoalesce,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Global,
    This,
    Other,
    Constant(Constant),
    Ident(String),
    Group(Box<Expr>),
    Object(Vec<Field>),
    Array(Vec<Expr>),
    Unary {
        op: UnaryOp,
        target: Box<Expr>,
    },
    Prefix(Mutation),
    Postfix(Mutation),
    Binary {
        lhs: Box<Expr>,
        op: BinaryOp,
        rhs: Box<Expr>,
    },
    Ternary {
        cond: Box<Expr>,
        if_true: Box<Expr>,
        if_false: Box<Expr>,
    },
    Call(Call),
    Field {
        base: Box<Expr>,
        field: String,
    },
    Index{
        base: Box<Expr>,
        accessor_type: Option<AccessorType>,
        indexes: Vec<Expr>,
    },
    Argument {
        arg_index: Box<Expr>,
    },
    ArgumentCount,
}

#[derive(Debug, Clone)]
pub enum BinaryOp {
    Add,
    Sub,
    Mult,
    Div,
    Rem,
    IDiv,
    Equal,
    NotEqual,
    LessThan,
    LessEqual,
    GreaterThan,
    GreaterEqual,
    And,
    Or,
    Xor,
    BitAnd,
    BitOr,
    BitXor,
    BitShiftLeft,
    BitShiftRight,
    NullCoalesce,
}

#[derive(Debug, Clone)]
pub enum UnaryOp {
    Not,
    Minus,
    BitNegate,
}

#[derive(Debug, Clone)]
pub enum Field {
    Value(String, Expr),
    Init(String)
}

#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    Undefined,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
}
