mod parse;

pub use self::parse::parse_ast;

#[derive(Debug)]
pub struct Ast {
    pub name: String,
    pub vardef: Vec<Definition>,
    pub statements: StatementList,
}

#[derive(Debug)]
pub struct Definition {
    pub name: String,
    pub dimensions: Vec<(i32, i32)>,
    pub dtype: DefinitionType,
}

#[derive(Debug)]
pub enum DefinitionType {
    Real,
    Integer,
}

#[derive(Debug)]
pub struct StatementList(pub Vec<Statement>);

#[derive(Debug)]
pub enum Statement {
    Assignment(Assign),
    Loop(Loop),
    If(If),
}

#[derive(Debug)]
pub struct Assign {
    pub label: i32,
    pub lhs: Variable,
    pub rhs: Expression,
}

#[derive(Debug)]
pub struct Loop {
    pub label: i32,
    pub var: String,
    pub lower: Expression,
    pub upper: Expression,
    pub statements: StatementList,
}

#[derive(Debug)]
pub struct If {
    pub label: i32,
    pub expr: Expression,
    pub then_branch: StatementList,
    pub else_branch: StatementList,
}

#[derive(Debug)]
pub struct Variable {
    pub name: String,
    pub indices: Vec<Expression>,
}

#[derive(Debug)]
pub enum Expression {
    Integer(i32),
    Real(f64),
    UnOp(Box<UnOp>),
    BinOp(Box<BinOp>),
    Variable(Variable),
    Expression(Box<Expression>),
}

#[derive(Debug, Clone)]
pub enum OpType {
    Plus,
    Minus,
    Mul,
    Div,
    Equal,
    NotEqual,
    Greater,
    GreaterEqual,
    Lower,
    LowerEqual,
    And,
    Or,
    Not,
}

#[derive(Debug)]
pub struct UnOp {
    pub op: OpType,
    pub right: Expression,
}

#[derive(Debug)]
pub struct BinOp {
    pub op: OpType,
    pub left: Expression,
    pub right: Expression,
}
