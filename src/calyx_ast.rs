use crate::ast::Type;

#[derive(Debug, Clone)]
pub struct Program {
    pub import_names: Vec<String>,
    pub main: Component,
    pub components: Vec<Component>,
}

#[derive(Debug, Clone)]
pub struct Component {
    pub name: String,
    pub params: Vec<(String, Type)>,
    pub cells: Vec<Cell>,
    pub wires: Vec<Group>,
    pub control: Vec<Control>,
}

#[derive(Debug, Clone)]
pub struct Cell {
    pub name: String,
    pub is_external: bool,
    pub circuit: Circuit,
}

#[derive(Debug, Clone)]
pub enum Circuit {
    CombMemD1 {
        data_width: usize,
        len: usize,
        address_width: usize,
    },
    StdReg {
        width: usize,
    },
    StdAdd {
        width: usize,
    },
    StdMul {
        width: usize,
    },
}

#[derive(Debug, Clone)]
pub struct Group {
    pub name: String,
    pub is_comb: bool,
    pub wires: Vec<Wire>,
}

#[derive(Debug, Clone)]
pub struct Wire {
    pub dest: Port,
    pub src: Src,
}

#[derive(Debug, Clone)]
pub enum Port {
    Port(String, String),
    Done(String),
}

#[derive(Debug, Clone)]
pub enum Src {
    Port(Port),
    Int { width: usize, value: isize },
}

#[derive(Debug, Clone)]
pub enum Control {
    Seq(Vec<Control>),
    Par(Vec<Control>),
    GroupName(String),
    While {
        condition: Port,
        with: Option<String>,
        body: Vec<Control>,
    },
}
