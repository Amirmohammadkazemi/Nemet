use crate::parser::block::Block;
use crate::parser::expr::Expr;

#[derive(Debug, Clone)]
pub enum VariableType {
    Custom(String),
    Array(Box<VariableType>, usize),
    String,
    Int,
    UInt,
    Bool,
    Char,
}
impl VariableType {
    pub fn from_string(literal: String) -> Self {
        match literal.as_str() {
            "int" | "i32" => Self::Int,
            "uint" | "u32" => Self::UInt,
            "char" | "u8" => Self::Char,
            "bool" => Self::Bool,
            "str" => Self::String,
            _ => Self::Custom(literal),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Stmt {
    // expr
    Expr(Expr),
    VariableDecl(VariableDeclare),
    // expr = expr
    Assgin(Assgin),
    Print(Expr),
    While(WhileStmt),
    If(IFStmt),
    Return(Expr),
    Break,
    Continue,
}

#[derive(Debug, Clone)]
pub struct Assgin {
    pub left: Expr,
    pub right: Expr,
    pub op: AssginOp,
}

#[derive(Debug, Clone)]
pub enum AssginOp {
    Eq,
}

#[derive(Debug, Clone)]
pub struct IFStmt {
    pub condition: Expr,
    pub then_block: Block,
    pub else_block: Box<ElseBlock>,
}

#[derive(Debug, Clone)]
pub enum ElseBlock {
    Elif(IFStmt),
    Else(Block),
    None,
}

#[derive(Debug, Clone)]
pub struct WhileStmt {
    pub condition: Expr,
    pub block: Block,
}

#[derive(Debug, Clone)]
pub struct VariableDeclare {
    pub mutable: bool,
    pub is_static: bool,
    pub ident: String,
    pub v_type: Option<VariableType>,
    pub init_value: Option<Expr>,
}