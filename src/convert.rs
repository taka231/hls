use std::collections::HashMap;

use crate::{
    ast::{self, ANormalBindLet, ANormalNoBindLet, Type},
    calyx_ast,
};
use anyhow::Result;

const ADDRESS_WIDTH: usize = 32;

#[derive(Debug, Clone)]
pub struct Converter {
    pub program: calyx_ast::Program,
    pub fresh_idx: usize,
    pub current_func: Option<String>,
    // HashMap<VariableName, CellName>
    pub env: HashMap<String, calyx_ast::Src>,
    pub type_env: HashMap<String, ast::Type>,
}

impl Converter {
    pub fn init() -> Self {
        let import_names = vec![
            "primitives/core.futil".to_string(),
            "primitives/binary_operators.futil".to_string(),
            "primitives/memories/comb.futil".to_string(),
        ];
        let program = calyx_ast::Program {
            import_names,
            main: calyx_ast::Component::new("main", vec![], vec![]),
            components: vec![],
        };
        Converter {
            program,
            fresh_idx: 0,
            current_func: None,
            env: HashMap::new(),
            type_env: HashMap::new(),
        }
    }

    fn fresh_name(&mut self) -> String {
        let name = format!("_tmp_{}", self.fresh_idx);
        self.fresh_idx += 1;
        name
    }

    fn find_src_by_var(&self, var: &str) -> Result<calyx_ast::Src> {
        if let Some(cell_name) = self.env.get(var) {
            Ok(cell_name.clone())
        } else {
            Err(anyhow::anyhow!("Variable {} not found in cell map", var))
        }
    }

    fn get_current_func(&mut self) -> Result<&mut calyx_ast::Component> {
        if let Some(func_name) = &self.current_func {
            if func_name == "main" {
                return Ok(&mut self.program.main);
            }
            if let Some(component) = self
                .program
                .components
                .iter_mut()
                .find(|c| c.name == *func_name)
            {
                return Ok(component);
            }
        }
        Err(anyhow::anyhow!(
            "No current function set or function not found"
        ))
    }

    fn new_group(&mut self) -> calyx_ast::Group {
        let name = format!("_group_{}", self.fresh_idx);
        self.fresh_idx += 1;
        calyx_ast::Group {
            name,
            done: None,
            wires: vec![],
        }
    }

    pub fn convert(&mut self, ast: ast::ANormalProgram) -> Result<()> {
        for decl in ast {
            match decl {
                ast::ANormalTopLevel::ExternalDecl(decl) => {
                    self.convert_external_decl(&decl)?;
                }
                ast::ANormalTopLevel::FunDef(fundef) => {
                    self.convert_fundef(&fundef)?;
                }
            }
        }
        Ok(())
    }

    fn convert_fundef(&mut self, fundef: &ast::ANormalFunDef) -> Result<()> {
        let ast::FunDef_ {
            name,
            params,
            return_type,
            body,
        } = fundef;
        self.current_func = Some(name.clone());
        let is_main = name == "main";
        if is_main && !(params.is_empty() && return_type.is_none()) {
            return Err(anyhow::anyhow!(
                "Main function must have no parameters and no return type"
            ));
        }
        if !is_main {
            todo!()
        }

        // let out = if let Some(ty) = return_type {
        //     self.program.main.result = Some((name.clone(), ty.clone()));
        //     Some(name.clone())
        // } else {
        //     None
        // };
        //
        self.convert_expr(body, None)?;

        Ok(())
    }

    fn convert_expr(&mut self, expr: &ast::ANormalExpr, out: Option<String>) -> Result<()> {
        let ast::Expr_(lets, body) = expr;
        for let_binding in lets {
            let control = self.convert_let(let_binding)?;
            self.get_current_func()?.push_control(control);
        }
        let control = self.convert_base_expr(body)?(out)?;
        self.get_current_func()?.push_control(control);
        Ok(())
    }

    fn convert_let(&mut self, let_binding: &ast::ANormalLet) -> Result<calyx_ast::Control> {
        match let_binding {
            ast::ANormalLet::BindLet(ANormalBindLet { name, value, ty }) => {
                self.type_env.insert(name.clone(), ty.clone());
                self.convert_base_expr(value)?(Some(name.clone()))
            }
            ast::ANormalLet::NoBindLet(ANormalNoBindLet { value }) => {
                self.convert_base_expr(value)?(None)
            }
        }
    }

    fn convert_base_expr<'a: 'b, 'b>(
        &'a mut self,
        base_expr: &'a ast::ANormalBaseExpr,
    ) -> Result<Box<dyn FnOnce(Option<String>) -> Result<calyx_ast::Control> + 'b>> {
        match base_expr {
            ast::ANormalBaseExpr::Int(n) => Ok(Box::new(|dest: Option<String>| {
                if let Some(dest) = dest {
                    let Some(Type::I(width)) = self.type_env.get(&dest) else {
                        return Err(anyhow::anyhow!("{} is required to be Int", dest));
                    };
                    self.env.insert(
                        dest.clone(),
                        calyx_ast::Src::Int {
                            value: *n as isize,
                            width: *width,
                        },
                    );
                }
                Ok(calyx_ast::Control::empty())
            })),
            ast::ANormalBaseExpr::Bool(b) => Ok(Box::new(|dest: Option<String>| {
                if let Some(dest) = dest {
                    self.env.insert(
                        dest.clone(),
                        calyx_ast::Src::Int {
                            value: if *b { 1 } else { 0 },
                            width: 1,
                        },
                    );
                }
                Ok(calyx_ast::Control::empty())
            })),
            ast::ANormalBaseExpr::Var(var) => Ok(Box::new(move |dest: Option<String>| {
                let src = self.find_src_by_var(var)?;
                if let Some(dest) = dest {
                    self.env.insert(dest.clone(), src);
                }
                Ok(calyx_ast::Control::empty())
            })),
            ast::ANormalBaseExpr::Add(var1, var2) => todo!(),
            ast::ANormalBaseExpr::Mul(var1, var2) => todo!(),
            ast::ANormalBaseExpr::NewArray(_, _) => todo!(),
            ast::ANormalBaseExpr::Map(vars, items, expr) => todo!(),
            ast::ANormalBaseExpr::Reduce(var, _, _, expr) => todo!(),
            ast::ANormalBaseExpr::Call(_, vars) => todo!(),
            ast::ANormalBaseExpr::ArraySet(array, index, value) => {
                let calyx_ast::Src::Port(array) = self.find_src_by_var(array)? else {
                    return Err(anyhow::anyhow!("Expected a port for array variable"));
                };
                let index = self.find_src_by_var(index)?;
                let value = self.find_src_by_var(value)?;

                Ok(Box::new(move |dest: Option<String>| {
                    let mut group = self.new_group();
                    group.wires.push(calyx_ast::Wire {
                        dest: array.port("addr0"),
                        src: index.clone(),
                    });
                    group.wires.push(calyx_ast::Wire {
                        dest: array.port("write_data"),
                        src: value.clone(),
                    });
                    group.wires.push(calyx_ast::Wire {
                        dest: array.port("write_en"),
                        src: calyx_ast::Src::Int { value: 1, width: 1 },
                    });
                    group.done = Some(array.port("done").into());
                    let group_name = group.name.clone();
                    self.get_current_func()?.wires.push(group);
                    if let Some(dest) = dest {
                        self.env.insert(dest.clone(), value);
                    }
                    Ok(calyx_ast::Control::GroupName(group_name))
                }))
            }
        }
    }

    fn convert_external_decl(&mut self, decl: &ast::ExternalDecl) -> Result<()> {
        let ast::Type::Array(ty, size) = &decl.ty else {
            return Err(anyhow::anyhow!("Unsupported type in external declaration"));
        };
        let ast::Type::I(width) = &**ty else {
            return Err(anyhow::anyhow!("Unsupported type in external declaration"));
        };
        self.program.main.cells.push(calyx_ast::Cell {
            name: decl.name.clone(),
            is_external: true,
            circuit: calyx_ast::Circuit::CombMemD1 {
                data_width: *width,
                len: *size,
                address_width: ADDRESS_WIDTH,
            },
        });
        // let mem_port = calyx_ast::Port::Port(decl.name.clone(), "read_data".to_string());
        let mem_port: calyx_ast::Src = calyx_ast::Port {
            cell: decl.name.clone(),
            port: "read_data".to_string(),
        }
        .into();
        self.env.insert(decl.name.clone(), mem_port);
        Ok(())
    }
}
