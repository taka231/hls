use std::fmt::{Display, Formatter, Result as FmtResult};

#[derive(Debug, Clone)]
pub struct Program {
    pub import_names: Vec<String>,
    pub main: Component,
    pub components: Vec<Component>,
}

impl Display for Program {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        // Format import statements
        for import in &self.import_names {
            writeln!(f, "import \"{}\";", import)?;
        }

        // Add blank line after imports if there are any
        if !self.import_names.is_empty() {
            writeln!(f)?;
        }

        // Format main component
        writeln!(f, "{}", self.main)?;

        // Format other components
        for component in &self.components {
            writeln!(f)?;
            writeln!(f, "{}", component)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Component {
    pub name: String,
    pub params: Vec<(String, Type)>,
    pub result: Vec<(String, Type)>,
    pub cells: Vec<Cell>,
    pub wires: Wires,
    pub control: Vec<Control>,
}

impl Component {
    pub fn push_control(&mut self, control: Control) {
        if !control.is_empty() {
            self.control.push(control);
        }
    }
}

pub type Type = usize;

impl Component {
    pub fn new(name: &str, params: Vec<(String, Type)>, result: Vec<(String, Type)>) -> Self {
        Component {
            name: name.to_string(),
            params,
            result,
            cells: vec![],
            wires: Wires {
                static_wires: vec![],
                groups: vec![],
            },
            control: vec![],
        }
    }
}

impl Display for Component {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        // Format component declaration
        write!(f, "component {}", self.name)?;

        write!(f, "(")?;
        for (i, (name, ty)) in self.params.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}: {}", name, ty)?;
        }
        write!(f, ") -> (")?;

        for (i, (name, ty)) in self.result.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}: {}", name, ty)?;
        }
        write!(f, ")")?;

        writeln!(f, " {{")?;

        // Format cells section
        writeln!(f, "  cells {{")?;
        for cell in &self.cells {
            writeln!(f, "    {}", cell)?;
        }
        writeln!(f, "  }}")?;

        // Format wires section
        writeln!(f, "  wires {{")?;
        writeln!(f, "{}", self.wires)?;
        writeln!(f, "  }}")?;

        // Format control section
        writeln!(f, "  control {{")?;
        writeln!(f, "    seq {{")?;
        for control in &self.control {
            writeln!(f, "      {}", control)?;
        }
        writeln!(f, "    }}")?;
        writeln!(f, "  }}")?;

        write!(f, "}}")
    }
}

#[derive(Debug, Clone)]
pub struct Cell {
    pub name: String,
    pub is_external: bool,
    pub circuit: Circuit,
}

impl Display for Cell {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let ref_ = if self.circuit.is_memory() { "ref" } else { "" };
        if self.is_external {
            write!(f, "@external(1) {} = {};", self.name, self.circuit)
        } else {
            write!(f, "{ref_} {} = {};", self.name, self.circuit)
        }
    }
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

impl Circuit {
    pub fn is_memory(&self) -> bool {
        matches!(self, Circuit::CombMemD1 { .. })
    }
}

impl Display for Circuit {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Circuit::CombMemD1 {
                data_width,
                len,
                address_width,
            } => {
                write!(f, "comb_mem_d1({}, {}, {})", data_width, len, address_width)
            }
            Circuit::StdReg { width } => write!(f, "std_reg({})", width),
            Circuit::StdAdd { width } => write!(f, "std_add({})", width),
            Circuit::StdMul { width } => write!(f, "std_mul({})", width),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Wires {
    pub static_wires: Vec<Wire>,
    pub groups: Vec<Group>,
}

impl Display for Wires {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        for wire in &self.static_wires {
            writeln!(f, "    {}", wire)?;
        }
        for group in &self.groups {
            writeln!(f, "    {}", group)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Group {
    pub name: String,
    pub done: Option<Src>,
    pub wires: Vec<Wire>,
}

impl Display for Group {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        if self.done.is_none() {
            writeln!(f, "comb group {} {{", self.name)?;
        } else {
            writeln!(f, "group {} {{", self.name)?;
        }

        for wire in &self.wires {
            writeln!(f, "      {}", wire)?;
        }
        if let Some(done) = &self.done {
            writeln!(f, "      {}[done] = {};", self.name, done)?;
        }

        write!(f, "    }}")
    }
}

#[derive(Debug, Clone)]
pub struct Wire {
    pub dest: Port,
    pub src: Src,
}

impl Display for Wire {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{} = {};", self.dest, self.src)
    }
}

#[derive(Debug, Clone)]
pub struct Port {
    pub cell: String,
    pub port: String,
}

impl Display for Port {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}.{}", self.cell, self.port)
    }
}

impl Port {
    pub fn port(&self, port: &str) -> Self {
        Port {
            cell: self.cell.clone(),
            port: port.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Src {
    Port(Port),
    Int { width: usize, value: isize },
}

impl From<Port> for Src {
    fn from(port: Port) -> Self {
        Src::Port(port)
    }
}

impl Display for Src {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Src::Port(port) => write!(f, "{}", port),
            Src::Int { width, value } => write!(f, "{}'d{}", width, value),
        }
    }
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

impl Control {
    pub fn empty() -> Self {
        Control::Seq(vec![])
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Control::Seq(controls) => controls.is_empty(),
            _ => false,
        }
    }
}

impl Display for Control {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Control::Seq(controls) => {
                writeln!(f, "seq {{")?;
                for control in controls {
                    let control_str = format!("{}", control);
                    for line in control_str.lines() {
                        writeln!(f, "  {}", line)?;
                    }
                }
                write!(f, "}}")
            }
            Control::Par(controls) => {
                writeln!(f, "par {{")?;
                for control in controls {
                    let control_str = format!("{}", control);
                    for line in control_str.lines() {
                        writeln!(f, "  {}", line)?;
                    }
                }
                write!(f, "}}")
            }
            Control::GroupName(name) => write!(f, "{};", name),
            Control::While {
                condition,
                with,
                body,
            } => {
                if let Some(with_group) = with {
                    writeln!(f, "while {} with {} {{", condition, with_group)?;
                } else {
                    writeln!(f, "while {} {{", condition)?;
                }
                for control in body {
                    let control_str = format!("{}", control);
                    for line in control_str.lines() {
                        writeln!(f, "  {}", line)?;
                    }
                }
                write!(f, "}}")
            }
        }
    }
}
