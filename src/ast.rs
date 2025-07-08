use std::ops::{Add, Mul};

pub type Ident = String;

pub type Program = Vec<TopLevel>;

#[derive(Debug, Clone)]
pub enum TopLevel {
    ExternalDecl(ExternalDecl),
    FunDef(FunDef),
}

#[derive(Debug, Clone)]
pub struct ExternalDecl {
    pub name: Ident,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub struct FunDef {
    pub name: Ident,
    pub params: Vec<(Ident, Type)>,
    pub return_type: Option<Type>,
    pub body: Expr,
}

#[derive(Debug, Clone)]
pub enum Type {
    I(usize),
    Array(Box<Type>, usize),
}

impl Type {
    pub fn array(ty: Type, size: usize) -> Self {
        Type::Array(Box::new(ty), size)
    }

    pub fn i32() -> Self {
        Type::I(32)
    }

    pub fn bool() -> Self {
        Type::I(1)
    }
}

#[derive(Debug, Clone)]
pub enum BaseExpr {
    Int(i32),
    Var(Ident),
    Add(Box<BaseExpr>, Box<BaseExpr>),
    Mul(Box<BaseExpr>, Box<BaseExpr>),
    NewArray(Box<Type>, usize),
    Map(Vec<BaseExpr>, Vec<Ident>, Box<Expr>),
    Reduce(Box<BaseExpr>, Ident, Ident, Box<Expr>),
    Call(Ident, Vec<BaseExpr>),
    ArraySet(Ident, Box<BaseExpr>, Box<BaseExpr>),
}

#[derive(Debug, Clone)]
pub struct BindLet {
    pub name: Ident,
    pub ty: Type,
    pub value: BaseExpr,
}

#[derive(Debug, Clone)]
pub struct NoBindLet {
    pub value: BaseExpr,
}

#[derive(Debug, Clone)]
pub enum Let {
    BindLet(BindLet),
    NoBindLet(NoBindLet),
}

pub fn let_(name: &str, ty: Type, value: BaseExpr) -> BindLet {
    BindLet {
        name: name.to_string(),
        ty,
        value,
    }
}

#[derive(Debug, Clone)]
pub struct Expr(pub Vec<Let>, pub BaseExpr);

impl BaseExpr {
    pub fn var(name: &str) -> Self {
        BaseExpr::Var(name.to_string())
    }

    pub fn new_array(ty: Type, size: usize) -> Self {
        BaseExpr::NewArray(Box::new(ty), size)
    }

    pub fn map(arrays: Vec<BaseExpr>, params: Vec<&str>, body: Expr) -> Self {
        let param_strings: Vec<String> = params.iter().map(|p| p.to_string()).collect();
        BaseExpr::Map(arrays, param_strings, Box::new(body))
    }

    pub fn reduce(array: BaseExpr, param1: &str, param2: &str, body: Expr) -> Self {
        BaseExpr::Reduce(
            array.into(),
            param1.to_string(),
            param2.to_string(),
            body.into(),
        )
    }

    pub fn call(name: &str, args: Vec<BaseExpr>) -> Self {
        BaseExpr::Call(name.to_string(), args)
    }
}

impl Add for BaseExpr {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        BaseExpr::Add(Box::new(self), Box::new(other))
    }
}

impl Mul for BaseExpr {
    type Output = Self;

    fn mul(self, other: Self) -> Self::Output {
        BaseExpr::Mul(Box::new(self), Box::new(other))
    }
}
